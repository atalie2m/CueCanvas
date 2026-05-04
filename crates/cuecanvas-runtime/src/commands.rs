use std::collections::{BTreeMap, BTreeSet};

use axum::http::StatusCode;
use cuecanvas_data::{
    ColumnMapping as DataColumnMapping, DataImportError, DataImportRequest,
    DelimitedFormat as DataDelimitedFormat, ImportMode as DataImportMode, apply_import,
};
use cuecanvas_engine::{CueEngine, demo_run, ensure_builtin_templates};
use cuecanvas_model::{
    Actor, Cue, CueMode, CueSheet, EntityRef, EntityType, ExtensionRegistry, LiveState,
    OperationLogEntry, Origin, OutputTarget, OutputTargetKind, PreflightItem, PreflightResult,
    PreflightScope, PreflightSeverity, ProjectMetadata, ProjectPackage, Rect, RunRevisions,
    RunSession, ShowDefinition, Stage, TemplateKind, TemplateState, Transition, TypedEntities,
};
use cuecanvas_preflight::{PreflightEngine, PreflightPolicy};
use cuecanvas_protocol::{
    BulkGenerateCuesPayload, CreateShowPayload, CueStillRequestPayload, DataImportPayload,
    DeleteCuePayload, DeleteTemplateInstancePayload, DeleteTypedEntityPayload,
    EmergencyLivePayload, ImportMode, LiveStateTransitionPayload, PreflightDisplayItem,
    PreflightGroups, PreflightState, PreflightTotals, PreviewCuePayload, ReorderCuesPayload,
    RuntimeCommandEnvelope, RuntimeCommandResponse, RuntimeEvent, TakePayload,
    TemplateInstancePayload, TypedEntityPayload, UpdateOutputTargetPayload, UpdateStagePayload,
    WarningOverridePayload,
};
use serde::de::DeserializeOwned;
use serde_json::{Value, json};
use uuid::Uuid;

use super::{RuntimeState, now_string};

pub struct CommandOutcome {
    pub response: RuntimeCommandResponse,
    pub events: Vec<RuntimeEvent>,
}

#[derive(Debug)]
pub struct CommandError {
    pub status: StatusCode,
    pub message: String,
}

impl CommandError {
    fn new(status: StatusCode, message: impl Into<String>) -> Self {
        Self {
            status,
            message: message.into(),
        }
    }
}

pub fn dispatch_command(
    state: &mut RuntimeState,
    envelope: RuntimeCommandEnvelope,
) -> Result<CommandOutcome, CommandError> {
    let cache_key = envelope
        .idempotency_key
        .as_ref()
        .map(|key| format!("{}:{key}", envelope.command));
    if let Some(cache_key) = cache_key.as_ref() {
        if let Some(response) = state.command_results.get(cache_key) {
            return Ok(CommandOutcome {
                response: response.clone(),
                events: vec![],
            });
        }
    }

    check_live_state_policy(state, &envelope.command)?;
    check_expected_revision(state, &envelope)?;

    let now = now_string();
    let actor = envelope.actor.clone();
    let emits_common_events = envelope.command != "cue.previewStill";
    let mut events = Vec::new();
    let result = match envelope.command.as_str() {
        "project.createShow" => {
            let payload: CreateShowPayload = decode(envelope.payload)?;
            let mut show = empty_show(payload.name, payload.stage, &now);
            ensure_builtin_templates(&mut show);
            let run = demo_run(&show);
            state.project.show_definitions.insert(0, show.clone());
            state.project.run_sessions.insert(0, run);
            state.project.metadata.updated_at = now.clone();
            push_operation(
                state,
                &actor,
                "project.createShow",
                format!("Created show '{}'", show.name),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(show)
        }
        "show.updateStage" => {
            let payload: UpdateStagePayload = decode(envelope.payload)?;
            let show = active_show_mut(state, payload.show_definition_id.as_deref())?;
            show.stage = payload.stage.clone();
            for target in &mut show.output_targets {
                target.stage = payload.stage.clone();
            }
            let show_id = show.id.clone();
            let message = format!(
                "Updated stage to {}x{}",
                payload.stage.width, payload.stage.height
            );
            touch_show(state, &show_id, &now, false);
            push_operation(
                state,
                &actor,
                "show.updateStage",
                message,
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(payload.stage)
        }
        "show.updateOutputTarget" => {
            let payload: UpdateOutputTargetPayload = decode(envelope.payload)?;
            let show = active_show_mut(state, payload.show_definition_id.as_deref())?;
            if let Some(existing) = show
                .output_targets
                .iter_mut()
                .find(|target| target.id == payload.output_target.id)
            {
                *existing = payload.output_target.clone();
            } else {
                show.output_targets.push(payload.output_target.clone());
            }
            let show_id = show.id.clone();
            touch_show(state, &show_id, &now, false);
            push_operation(
                state,
                &actor,
                "show.updateOutputTarget",
                format!("Updated output target '{}'", payload.output_target.name),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(payload.output_target)
        }
        "run.create" => {
            let show = active_show(state)?;
            let mut run = demo_run(show);
            run.id = format!("run-{}", Uuid::new_v4());
            run.live_state = LiveState::Setup;
            state.project.run_sessions.insert(0, run.clone());
            state.project.metadata.updated_at = now.clone();
            push_operation(
                state,
                &actor,
                "run.create",
                format!("Created run session '{}'", run.id),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(run)
        }
        "run.open" => {
            let payload: cuecanvas_protocol::RunSessionPayload = decode(envelope.payload)?;
            let run_id = payload
                .run_session_id
                .or_else(|| payload.run_session.as_ref().map(|run| run.id.clone()))
                .ok_or_else(|| {
                    CommandError::new(StatusCode::BAD_REQUEST, "runSessionId is required")
                })?;
            if let Some(run) = payload.run_session {
                upsert_run_session(state, run);
            }
            let position = state
                .project
                .run_sessions
                .iter()
                .position(|run| run.id == run_id)
                .ok_or_else(|| CommandError::new(StatusCode::NOT_FOUND, "RunSession not found"))?;
            let run = state.project.run_sessions.remove(position);
            state.project.run_sessions.insert(0, run.clone());
            push_operation(
                state,
                &actor,
                "run.open",
                format!("Opened run session '{}'", run.id),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(run)
        }
        "cue.upsert" => {
            let payload: cuecanvas_protocol::CuePayload = decode(envelope.payload)?;
            let show = active_show_mut(state, payload.show_definition_id.as_deref())?;
            upsert_cue(show, payload.cue.clone());
            let show_id = show.id.clone();
            touch_show(state, &show_id, &now, false);
            push_operation(
                state,
                &actor,
                "cue.upsert",
                format!("Saved cue '{}'", payload.cue.name),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(payload.cue)
        }
        "cue.delete" => {
            let payload: DeleteCuePayload = decode(envelope.payload)?;
            let show = active_show_mut(state, payload.show_definition_id.as_deref())?;
            let before = show.cue_sheet.cues.len();
            show.cue_sheet.cues.retain(|cue| cue.id != payload.cue_id);
            if show.cue_sheet.cues.len() == before {
                return Err(CommandError::new(StatusCode::NOT_FOUND, "Cue not found"));
            }
            let show_id = show.id.clone();
            touch_show(state, &show_id, &now, false);
            push_operation(
                state,
                &actor,
                "cue.delete",
                format!("Deleted cue '{}'", payload.cue_id),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!({ "cueId": payload.cue_id })
        }
        "cue.reorder" => {
            let payload: ReorderCuesPayload = decode(envelope.payload)?;
            let show = active_show_mut(state, payload.show_definition_id.as_deref())?;
            reorder_cues(&mut show.cue_sheet.cues, &payload.cue_ids)?;
            let show_id = show.id.clone();
            touch_show(state, &show_id, &now, false);
            push_operation(
                state,
                &actor,
                "cue.reorder",
                "Reordered cue sheet".to_string(),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!({ "cueIds": payload.cue_ids })
        }
        "cue.bulkGenerate" => {
            let payload: BulkGenerateCuesPayload = decode(envelope.payload)?;
            let show = active_show_mut(state, payload.show_definition_id.as_deref())?;
            let entity_ids = match &payload.entity_type {
                EntityType::People => show
                    .typed_entities
                    .people
                    .iter()
                    .map(|entity| entity.id.clone())
                    .collect::<Vec<_>>(),
                EntityType::Sessions => show
                    .typed_entities
                    .sessions
                    .iter()
                    .map(|entity| entity.id.clone())
                    .collect::<Vec<_>>(),
                EntityType::Sponsors => show
                    .typed_entities
                    .sponsors
                    .iter()
                    .map(|entity| entity.id.clone())
                    .collect::<Vec<_>>(),
            };
            let mut generated = Vec::new();
            for entity_id in entity_ids {
                if let Some(cue) =
                    built_in_cue_for_entity(show, payload.entity_type.clone(), entity_id)
                {
                    generated.push(cue);
                }
            }
            for cue in &generated {
                upsert_cue(show, cue.clone());
            }
            let show_id = show.id.clone();
            touch_show(state, &show_id, &now, false);
            push_operation(
                state,
                &actor,
                "cue.bulkGenerate",
                format!("Generated {} cue(s)", generated.len()),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(generated)
        }
        "cue.previewStill" => {
            let payload: CueStillRequestPayload = decode(envelope.payload)?;
            let show = active_show_for_id(state, payload.show_definition_id.as_deref())?;
            let cue = show
                .cue_sheet
                .cues
                .iter()
                .find(|cue| cue.id == payload.cue_id)
                .ok_or_else(|| CommandError::new(StatusCode::NOT_FOUND, "Cue not found"))?;
            json!(deterministic_cue_still(show, cue))
        }
        "templateInstance.upsert" => {
            let payload: TemplateInstancePayload = decode(envelope.payload)?;
            let show = active_show_mut(state, payload.show_definition_id.as_deref())?;
            if let Some(existing) = show
                .template_instances
                .iter_mut()
                .find(|instance| instance.id == payload.template_instance.id)
            {
                *existing = payload.template_instance.clone();
            } else {
                show.template_instances
                    .push(payload.template_instance.clone());
            }
            let show_id = show.id.clone();
            touch_show(state, &show_id, &now, false);
            push_operation(
                state,
                &actor,
                "templateInstance.upsert",
                format!(
                    "Saved template instance '{}'",
                    payload.template_instance.name
                ),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(payload.template_instance)
        }
        "templateInstance.delete" => {
            let payload: DeleteTemplateInstancePayload = decode(envelope.payload)?;
            let show = active_show_mut(state, payload.show_definition_id.as_deref())?;
            show.template_instances
                .retain(|instance| instance.id != payload.template_instance_id);
            for cue in &mut show.cue_sheet.cues {
                cue.template_states
                    .retain(|state| state.template_instance_id != payload.template_instance_id);
                cue.slot_overrides.remove(&payload.template_instance_id);
            }
            let show_id = show.id.clone();
            touch_show(state, &show_id, &now, false);
            push_operation(
                state,
                &actor,
                "templateInstance.delete",
                format!(
                    "Deleted template instance '{}'",
                    payload.template_instance_id
                ),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!({ "templateInstanceId": payload.template_instance_id })
        }
        "entity.upsert" => {
            let payload: TypedEntityPayload = decode(envelope.payload)?;
            let (show_id, message, value) = upsert_entity(state, payload)?;
            touch_show(state, &show_id, &now, true);
            push_operation(
                state,
                &actor,
                "entity.upsert",
                message,
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            value
        }
        "entity.delete" => {
            let payload: DeleteTypedEntityPayload = decode(envelope.payload)?;
            let show = active_show_mut(state, payload.show_definition_id.as_deref())?;
            delete_entity(show, &payload.entity_type, &payload.entity_id)?;
            let show_id = show.id.clone();
            touch_show(state, &show_id, &now, true);
            push_operation(
                state,
                &actor,
                "entity.delete",
                format!("Deleted {:?} '{}'", payload.entity_type, payload.entity_id),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!({ "entityType": payload.entity_type, "entityId": payload.entity_id })
        }
        "data.import" => {
            let payload: DataImportPayload = decode(envelope.payload)?;
            let request = data_request(payload);
            let show = active_show_mut(state, None)?;
            let report = apply_import(show, request).map_err(data_error)?;
            let show_id = show.id.clone();
            touch_show(state, &show_id, &now, true);
            push_operation(
                state,
                &actor,
                "data.import",
                format!(
                    "Imported {} {:?} row(s) from {}",
                    report.imported_rows, report.entity_type, report.source_name
                ),
                None,
                active_run(state).ok().and_then(|run| {
                    run.program_snapshot
                        .as_ref()
                        .map(|snapshot| snapshot.program_revision)
                }),
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(report)
        }
        "warningOverride.upsert" => {
            let payload: WarningOverridePayload = decode(envelope.payload)?;
            let preflight = preflight_state_for_project(&state.project)?;
            let matching_warning = preflight.groups.warnings.iter().any(|item| {
                item.warning_key.as_deref() == Some(payload.warning_key.as_str())
                    && item.value_hash.as_deref() == Some(payload.value_hash.as_str())
            });
            if !matching_warning {
                return Err(CommandError::new(
                    StatusCode::CONFLICT,
                    "Warning override no longer matches current preflight value",
                ));
            }
            let run = active_run_mut(state, payload.run_session_id.as_deref())?;
            run.warning_overrides
                .insert(payload.warning_key.clone(), payload.value_hash.clone());
            run.revisions.runtime_revision += 1;
            push_operation(
                state,
                &actor,
                "warningOverride.upsert",
                format!("Overrode warning '{}'", payload.warning_key),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(payload)
        }
        "live.transition" => {
            let payload: LiveStateTransitionPayload = decode(envelope.payload)?;
            if matches!(
                payload.live_state,
                LiveState::PreflightReady | LiveState::LiveArmed | LiveState::LiveRunning
            ) {
                let preflight = preflight_state_for_project(&state.project)?;
                if preflight.totals.errors > 0 {
                    return Err(CommandError::new(
                        StatusCode::CONFLICT,
                        "Live start blocked by preflight errors",
                    ));
                }
            }
            let run = active_run_mut(state, payload.run_session_id.as_deref())?;
            run.live_state = payload.live_state.clone();
            run.revisions.runtime_revision += 1;
            push_operation(
                state,
                &actor,
                "live.transition",
                format!("Set live state to {:?}", payload.live_state),
                None,
                None,
                envelope.idempotency_key.clone(),
                &now,
            );
            json!(payload)
        }
        "live.previewCue" => {
            let payload: PreviewCuePayload = decode(envelope.payload)?;
            let show = active_show(state)?.clone();
            let run = active_run_mut(state, payload.run_session_id.as_deref())?;
            let preview_revision = run.revisions.preview_revision + 1;
            let preview =
                CueEngine::preview_cue(&show, run, &payload.cue_id, preview_revision, now.clone())
                    .map_err(|error| {
                        CommandError::new(StatusCode::BAD_REQUEST, error.to_string())
                    })?;
            run.revisions.preview_revision = preview_revision;
            run.revisions.runtime_revision += 1;
            run.preview_snapshot = Some(preview.clone());
            run.operation_log.push(OperationLogEntry {
                id: format!("op-{}", Uuid::new_v4()),
                at: now.clone(),
                actor: actor.clone(),
                action: "live.previewCue".to_string(),
                message: format!("Previewed Cue {}", payload.cue_id),
                preview_revision: Some(preview_revision),
                program_revision: None,
                idempotency_key: envelope.idempotency_key.clone(),
            });
            events.push(RuntimeEvent::PreviewChanged {
                source: actor.clone(),
                revision: run.revisions.runtime_revision,
                payload: preview.clone(),
            });
            json!(preview)
        }
        "live.take" => {
            let payload: TakePayload = decode(envelope.payload)?;
            let show = active_show(state)?.clone();
            let run = active_run_mut(state, payload.run_session_id.as_deref())?;
            let preview = run
                .preview_snapshot
                .clone()
                .filter(|preview| {
                    preview.id == payload.preview_snapshot_id
                        && preview.preview_revision == payload.preview_revision
                })
                .ok_or_else(|| {
                    CommandError::new(StatusCode::CONFLICT, "PreviewSnapshot is stale or missing")
                })?;
            reject_blocking_preflight(run, &show, &preview)?;
            let key = envelope
                .idempotency_key
                .clone()
                .unwrap_or_else(|| format!("take-{}", Uuid::new_v4()));
            let program = CueEngine::take_preview(
                run,
                &preview,
                &payload.output_target_id,
                payload.expected_program_revision,
                &key,
                actor.clone(),
                now.clone(),
            )
            .map_err(|error| CommandError::new(StatusCode::CONFLICT, error.to_string()))?;
            events.push(RuntimeEvent::ProgramChanged {
                source: actor.clone(),
                revision: run.revisions.runtime_revision,
                payload: program.clone(),
            });
            json!(program)
        }
        "live.clear" | "live.blackout" | "live.restore" => {
            let payload: EmergencyLivePayload = decode(envelope.payload)?;
            let run = active_run_mut(state, payload.run_session_id.as_deref())?;
            let key = envelope
                .idempotency_key
                .clone()
                .unwrap_or_else(|| format!("{}-{}", envelope.command, Uuid::new_v4()));
            let program = match envelope.command.as_str() {
                "live.clear" => CueEngine::clear_program(
                    run,
                    &payload.output_target_id,
                    &key,
                    actor.clone(),
                    now.clone(),
                ),
                "live.blackout" => CueEngine::blackout_program(
                    run,
                    &payload.output_target_id,
                    &key,
                    actor.clone(),
                    now.clone(),
                ),
                _ => CueEngine::restore_program(run, &key, actor.clone(), now.clone())
                    .map_err(|error| CommandError::new(StatusCode::CONFLICT, error.to_string()))?,
            };
            events.push(RuntimeEvent::ProgramChanged {
                source: actor.clone(),
                revision: run.revisions.runtime_revision,
                payload: program.clone(),
            });
            json!(program)
        }
        other => {
            return Err(CommandError::new(
                StatusCode::BAD_REQUEST,
                format!("Unsupported command '{other}'"),
            ));
        }
    };

    if emits_common_events {
        append_common_events(state, &actor, &mut events);
    }

    let response = RuntimeCommandResponse {
        command: envelope.command,
        revision: active_run(state)
            .map(|run| run.revisions.runtime_revision)
            .unwrap_or(0),
        result,
    };
    if let Some(cache_key) = cache_key {
        state.command_results.insert(cache_key, response.clone());
    }

    Ok(CommandOutcome { response, events })
}

fn check_expected_revision(
    state: &RuntimeState,
    envelope: &RuntimeCommandEnvelope,
) -> Result<(), CommandError> {
    let Some(expected) = envelope.expected_revision else {
        return Ok(());
    };

    let actual = if envelope.command.starts_with("live.")
        || envelope.command.starts_with("run.")
        || envelope.command.starts_with("warningOverride.")
    {
        active_run(state)?.revisions.runtime_revision
    } else {
        active_show(state)?.revision
    };

    if actual == expected {
        Ok(())
    } else {
        Err(CommandError::new(
            StatusCode::CONFLICT,
            format!("Expected revision {expected}, found {actual}"),
        ))
    }
}

fn check_live_state_policy(state: &RuntimeState, command: &str) -> Result<(), CommandError> {
    let Ok(run) = active_run(state) else {
        return Ok(());
    };
    if run.live_state != LiveState::LiveRunning {
        return Ok(());
    }
    if command.starts_with("live.")
        || command.starts_with("warningOverride.")
        || command == "cue.previewStill"
    {
        return Ok(());
    }
    Err(CommandError::new(
        StatusCode::CONFLICT,
        "Edit commands are blocked while LiveRunning",
    ))
}

fn append_common_events(state: &RuntimeState, actor: &Actor, events: &mut Vec<RuntimeEvent>) {
    if let Ok(run) = active_run(state) {
        if let Some(entry) = run.operation_log.last() {
            events.push(RuntimeEvent::OperationLogged {
                source: actor.clone(),
                revision: run.revisions.runtime_revision,
                payload: json!(entry),
            });
        }
    }

    if let (Ok(show), Ok(run)) = (active_show(state), active_run(state)) {
        let preflight = preflight_state_for_project(&state.project).ok();
        events.push(RuntimeEvent::PreflightChanged {
            source: actor.clone(),
            revision: run.revisions.runtime_revision,
            payload: json!(preflight.unwrap_or_else(|| empty_preflight_state(show, run))),
        });
        events.push(RuntimeEvent::AutosaveChanged {
            source: actor.clone(),
            revision: run.revisions.runtime_revision,
            payload: json!({
                "status": if state.package_dir.is_some() { "scheduled" } else { "notConfigured" },
                "packageDir": state.package_dir.as_ref().map(|path| path.to_string_lossy().to_string())
            }),
        });
    }
}

pub fn preflight_state_for_project(
    project: &ProjectPackage,
) -> Result<PreflightState, CommandError> {
    let show = project
        .show_definitions
        .first()
        .ok_or_else(|| CommandError::new(StatusCode::CONFLICT, "No active ShowDefinition"))?;
    let run = project
        .run_sessions
        .first()
        .ok_or_else(|| CommandError::new(StatusCode::CONFLICT, "No active RunSession"))?;
    Ok(build_preflight_state(
        show,
        run,
        &project.extension_registry,
        &PreflightPolicy::default(),
    ))
}

pub fn build_preflight_state(
    show: &ShowDefinition,
    run: &RunSession,
    registry: &ExtensionRegistry,
    policy: &PreflightPolicy,
) -> PreflightState {
    let mut items = PreflightEngine::run_show(show, run, policy).items;
    if policy.warn_on_missing_plugin_origin {
        items.extend(PreflightEngine::missing_plugin_origin_warnings(
            registry,
            &show_origin_chain(show),
        ));
    }
    if let Some(preview) = &run.preview_snapshot {
        items.extend(PreflightEngine::run_preview(run, preview, show.revision).items);
    }

    let mut groups = PreflightGroups::default();
    for item in items {
        let display = display_preflight_item(item, run);
        if display.severity == PreflightSeverity::Error {
            groups.errors.push(display);
        } else if display.severity == PreflightSeverity::Warning {
            groups.warnings.push(display);
        } else {
            groups.infos.push(display);
        }
    }

    let totals = PreflightTotals {
        errors: groups.errors.len(),
        warnings: groups.warnings.len(),
        infos: groups.infos.len(),
        active_warnings: groups
            .warnings
            .iter()
            .filter(|item| !item.overridden)
            .count(),
        overridden_warnings: groups
            .warnings
            .iter()
            .filter(|item| item.overridden)
            .count(),
    };

    PreflightState {
        show_definition_id: show.id.clone(),
        run_session_id: run.id.clone(),
        runtime_revision: run.revisions.runtime_revision,
        groups,
        totals,
    }
}

fn empty_preflight_state(show: &ShowDefinition, run: &RunSession) -> PreflightState {
    PreflightState {
        show_definition_id: show.id.clone(),
        run_session_id: run.id.clone(),
        runtime_revision: run.revisions.runtime_revision,
        groups: PreflightGroups::default(),
        totals: PreflightTotals::default(),
    }
}

fn display_preflight_item(item: PreflightItem, run: &RunSession) -> PreflightDisplayItem {
    let (warning_key, value_hash, overridden) = if item.severity == PreflightSeverity::Warning {
        let key = warning_key_for(&item);
        let hash = warning_value_hash(&item);
        let overridden = run
            .warning_overrides
            .get(&key)
            .is_some_and(|stored| stored == &hash);
        (Some(key), Some(hash), overridden)
    } else {
        (None, None, false)
    };

    PreflightDisplayItem {
        rule_id: item.rule_id,
        severity: item.severity,
        scope_label: scope_label(&item.scope),
        scope: item.scope,
        source_label: origin_label(&item.source),
        source: item.source,
        message: item.message,
        fix_actions: item.fix_actions,
        warning_key,
        value_hash,
        overridden,
    }
}

fn warning_key_for(item: &PreflightItem) -> String {
    format!(
        "{}:{}",
        item.rule_id,
        serde_json::to_string(&item.scope).unwrap_or_else(|_| "scope".to_string())
    )
}

fn warning_value_hash(item: &PreflightItem) -> String {
    let value = json!({
        "ruleId": item.rule_id,
        "scope": item.scope,
        "source": item.source,
        "message": item.message,
        "fixActions": item.fix_actions,
    });
    format!(
        "fnv1a64:{:016x}",
        fnv1a64(serde_json::to_string(&value).unwrap_or_default().as_bytes())
    )
}

fn fnv1a64(bytes: &[u8]) -> u64 {
    let mut hash = 0xcbf29ce484222325_u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    hash
}

fn scope_label(scope: &PreflightScope) -> String {
    match scope {
        PreflightScope::Show => "Show".to_string(),
        PreflightScope::Cue { cue_id } => format!("Cue {cue_id}"),
        PreflightScope::TemplateInstance {
            template_instance_id,
        } => format!("Template {template_instance_id}"),
        PreflightScope::OutputTarget { output_target_id } => {
            format!("Output {output_target_id}")
        }
    }
}

fn origin_label(origin: &Origin) -> String {
    match origin {
        Origin::System => "system".to_string(),
        Origin::User { user_id } => user_id
            .as_deref()
            .map(|id| format!("user:{id}"))
            .unwrap_or_else(|| "user".to_string()),
        Origin::Plugin {
            plugin_id,
            plugin_version,
        } => format!("plugin:{plugin_id}@{plugin_version}"),
        Origin::Import { source } => format!("import:{source}"),
        Origin::Migration { migration_id } => format!("migration:{migration_id}"),
    }
}

fn show_origin_chain(show: &ShowDefinition) -> Vec<Origin> {
    let mut origins = vec![show.origin.clone()];
    origins.extend(
        show.output_targets
            .iter()
            .map(|target| target.origin.clone()),
    );
    origins.extend(show.assets.iter().map(|asset| asset.origin.clone()));
    origins.extend(
        show.template_definitions
            .iter()
            .map(|definition| definition.origin.clone()),
    );
    origins.extend(
        show.template_instances
            .iter()
            .map(|instance| instance.origin.clone()),
    );
    origins.extend(
        show.typed_entities
            .people
            .iter()
            .map(|entity| entity.origin.clone()),
    );
    origins.extend(
        show.typed_entities
            .sessions
            .iter()
            .map(|entity| entity.origin.clone()),
    );
    origins.extend(
        show.typed_entities
            .sponsors
            .iter()
            .map(|entity| entity.origin.clone()),
    );
    origins.extend(show.cue_sheet.cues.iter().map(|cue| cue.origin.clone()));
    origins
}

fn decode<T: DeserializeOwned>(value: Value) -> Result<T, CommandError> {
    serde_json::from_value(value).map_err(|error| {
        CommandError::new(
            StatusCode::BAD_REQUEST,
            format!("Invalid command payload: {error}"),
        )
    })
}

fn active_show(state: &RuntimeState) -> Result<&ShowDefinition, CommandError> {
    state
        .project
        .show_definitions
        .first()
        .ok_or_else(|| CommandError::new(StatusCode::CONFLICT, "No active ShowDefinition"))
}

fn active_show_for_id<'a>(
    state: &'a RuntimeState,
    show_id: Option<&str>,
) -> Result<&'a ShowDefinition, CommandError> {
    if let Some(show_id) = show_id {
        return state
            .project
            .show_definitions
            .iter()
            .find(|show| show.id == show_id)
            .ok_or_else(|| CommandError::new(StatusCode::NOT_FOUND, "ShowDefinition not found"));
    }
    active_show(state)
}

fn active_show_mut<'a>(
    state: &'a mut RuntimeState,
    show_id: Option<&str>,
) -> Result<&'a mut ShowDefinition, CommandError> {
    if let Some(show_id) = show_id {
        return state
            .project
            .show_definitions
            .iter_mut()
            .find(|show| show.id == show_id)
            .ok_or_else(|| CommandError::new(StatusCode::NOT_FOUND, "ShowDefinition not found"));
    }
    state
        .project
        .show_definitions
        .first_mut()
        .ok_or_else(|| CommandError::new(StatusCode::CONFLICT, "No active ShowDefinition"))
}

fn active_run(state: &RuntimeState) -> Result<&RunSession, CommandError> {
    state
        .project
        .run_sessions
        .first()
        .ok_or_else(|| CommandError::new(StatusCode::CONFLICT, "No active RunSession"))
}

fn active_run_mut<'a>(
    state: &'a mut RuntimeState,
    run_id: Option<&str>,
) -> Result<&'a mut RunSession, CommandError> {
    if let Some(run_id) = run_id {
        return state
            .project
            .run_sessions
            .iter_mut()
            .find(|run| run.id == run_id)
            .ok_or_else(|| CommandError::new(StatusCode::NOT_FOUND, "RunSession not found"));
    }
    state
        .project
        .run_sessions
        .first_mut()
        .ok_or_else(|| CommandError::new(StatusCode::CONFLICT, "No active RunSession"))
}

fn touch_show(state: &mut RuntimeState, show_id: &str, now: &str, bump_data: bool) {
    if let Some(show) = state
        .project
        .show_definitions
        .iter_mut()
        .find(|show| show.id == show_id)
    {
        show.revision += 1;
    }
    state.project.metadata.updated_at = now.to_string();
    let show_revision = state
        .project
        .show_definitions
        .iter()
        .find(|show| show.id == show_id)
        .map(|show| show.revision)
        .unwrap_or_default();
    for run in state
        .project
        .run_sessions
        .iter_mut()
        .filter(|run| run.show_definition_id == show_id)
    {
        run.show_definition_revision = show_revision;
        run.revisions.project_revision += 1;
        run.revisions.show_definition_revision = show_revision;
        if bump_data {
            run.revisions.data_revision += 1;
        }
        run.revisions.runtime_revision += 1;
    }
}

#[allow(clippy::too_many_arguments)]
fn push_operation(
    state: &mut RuntimeState,
    actor: &Actor,
    action: &str,
    message: String,
    preview_revision: Option<u64>,
    program_revision: Option<u64>,
    idempotency_key: Option<String>,
    at: &str,
) {
    if let Ok(run) = active_run_mut(state, None) {
        run.operation_log.push(OperationLogEntry {
            id: format!("op-{}", Uuid::new_v4()),
            at: at.to_string(),
            actor: actor.clone(),
            action: action.to_string(),
            message,
            preview_revision,
            program_revision,
            idempotency_key,
        });
    }
}

fn empty_show(name: String, stage: Stage, now: &str) -> ShowDefinition {
    let output_target = OutputTarget {
        id: "output-program-obs".to_string(),
        name: "OBS Program Overlay".to_string(),
        kind: OutputTargetKind::ObsBrowserSource,
        stage: stage.clone(),
        overlay_url: "http://127.0.0.1:4317/overlay/program".to_string(),
        origin: Origin::system(),
        extensions: BTreeMap::new(),
    };
    ShowDefinition {
        id: format!("show-{}", Uuid::new_v4()),
        revision: 1,
        name,
        stage,
        output_targets: vec![output_target],
        assets: vec![],
        template_definitions: vec![],
        template_instances: vec![],
        typed_entities: TypedEntities::default(),
        data_tables: vec![],
        cue_sheet: CueSheet { cues: vec![] },
        show_settings: json!({ "createdAt": now }),
        origin: Origin::local_user(),
        extensions: BTreeMap::new(),
    }
}

fn upsert_run_session(state: &mut RuntimeState, run: RunSession) {
    if let Some(existing) = state
        .project
        .run_sessions
        .iter_mut()
        .find(|existing| existing.id == run.id)
    {
        *existing = run;
    } else {
        state.project.run_sessions.push(run);
    }
}

fn upsert_cue(show: &mut ShowDefinition, cue: Cue) {
    if let Some(existing) = show
        .cue_sheet
        .cues
        .iter_mut()
        .find(|item| item.id == cue.id)
    {
        *existing = cue;
    } else {
        show.cue_sheet.cues.push(cue);
    }
}

fn reorder_cues(cues: &mut Vec<Cue>, cue_ids: &[String]) -> Result<(), CommandError> {
    let existing_ids = cues
        .iter()
        .map(|cue| cue.id.as_str())
        .collect::<BTreeSet<_>>();
    let requested_ids = cue_ids.iter().map(String::as_str).collect::<BTreeSet<_>>();
    if existing_ids != requested_ids {
        return Err(CommandError::new(
            StatusCode::BAD_REQUEST,
            "cueIds must contain every existing cue exactly once",
        ));
    }
    let mut by_id = cues
        .drain(..)
        .map(|cue| (cue.id.clone(), cue))
        .collect::<BTreeMap<_, _>>();
    for cue_id in cue_ids {
        if let Some(cue) = by_id.remove(cue_id) {
            cues.push(cue);
        }
    }
    Ok(())
}

fn upsert_entity(
    state: &mut RuntimeState,
    payload: TypedEntityPayload,
) -> Result<(String, String, Value), CommandError> {
    match payload {
        TypedEntityPayload::People {
            show_definition_id,
            entity,
        } => {
            let show = active_show_mut(state, show_definition_id.as_deref())?;
            if let Some(existing) = show
                .typed_entities
                .people
                .iter_mut()
                .find(|existing| existing.id == entity.id)
            {
                *existing = entity.clone();
            } else {
                show.typed_entities.people.push(entity.clone());
            }
            Ok((
                show.id.clone(),
                format!("Saved person '{}'", entity.display_name),
                json!(entity),
            ))
        }
        TypedEntityPayload::Sessions {
            show_definition_id,
            entity,
        } => {
            let show = active_show_mut(state, show_definition_id.as_deref())?;
            if let Some(existing) = show
                .typed_entities
                .sessions
                .iter_mut()
                .find(|existing| existing.id == entity.id)
            {
                *existing = entity.clone();
            } else {
                show.typed_entities.sessions.push(entity.clone());
            }
            Ok((
                show.id.clone(),
                format!("Saved session '{}'", entity.title),
                json!(entity),
            ))
        }
        TypedEntityPayload::Sponsors {
            show_definition_id,
            entity,
        } => {
            let show = active_show_mut(state, show_definition_id.as_deref())?;
            if let Some(existing) = show
                .typed_entities
                .sponsors
                .iter_mut()
                .find(|existing| existing.id == entity.id)
            {
                *existing = entity.clone();
            } else {
                show.typed_entities.sponsors.push(entity.clone());
            }
            Ok((
                show.id.clone(),
                format!("Saved sponsor '{}'", entity.name),
                json!(entity),
            ))
        }
    }
}

fn delete_entity(
    show: &mut ShowDefinition,
    entity_type: &EntityType,
    entity_id: &str,
) -> Result<(), CommandError> {
    let before = match entity_type {
        EntityType::People => {
            let before = show.typed_entities.people.len();
            show.typed_entities
                .people
                .retain(|entity| entity.id != entity_id);
            before
        }
        EntityType::Sessions => {
            let before = show.typed_entities.sessions.len();
            show.typed_entities
                .sessions
                .retain(|entity| entity.id != entity_id);
            before
        }
        EntityType::Sponsors => {
            let before = show.typed_entities.sponsors.len();
            show.typed_entities
                .sponsors
                .retain(|entity| entity.id != entity_id);
            before
        }
    };
    for cue in &mut show.cue_sheet.cues {
        cue.entity_refs
            .retain(|_, entity_ref| entity_ref.entity_id != entity_id);
    }
    let after = match entity_type {
        EntityType::People => show.typed_entities.people.len(),
        EntityType::Sessions => show.typed_entities.sessions.len(),
        EntityType::Sponsors => show.typed_entities.sponsors.len(),
    };
    if before == after {
        Err(CommandError::new(StatusCode::NOT_FOUND, "Entity not found"))
    } else {
        Ok(())
    }
}

fn data_request(payload: DataImportPayload) -> DataImportRequest {
    DataImportRequest {
        entity_type: payload.entity_type,
        source_name: payload.source_name,
        format: match payload.format {
            cuecanvas_protocol::DelimitedFormat::Csv => DataDelimitedFormat::Csv,
            cuecanvas_protocol::DelimitedFormat::Tsv => DataDelimitedFormat::Tsv,
        },
        content: payload.content,
        mappings: payload
            .mappings
            .into_iter()
            .map(|mapping| DataColumnMapping {
                source: mapping.source,
                target: mapping.target,
            })
            .collect(),
        replace_existing: payload.replace_existing,
        mode: payload.mode.map(|mode| match mode {
            ImportMode::Append => DataImportMode::Append,
            ImportMode::Replace => DataImportMode::Replace,
            ImportMode::Merge => DataImportMode::Merge,
        }),
    }
}

fn reject_blocking_preflight(
    run: &RunSession,
    show: &ShowDefinition,
    preview: &cuecanvas_model::PreviewSnapshot,
) -> Result<(), CommandError> {
    let show_result = PreflightEngine::run_show(show, run, &PreflightPolicy::default());
    let preview_result = PreflightEngine::run_preview(run, preview, show.revision);
    let blocking = show_result
        .items
        .iter()
        .chain(preview_result.items.iter())
        .find(|item| item.severity == cuecanvas_model::PreflightSeverity::Error);
    if let Some(item) = blocking {
        Err(CommandError::new(
            StatusCode::CONFLICT,
            format!(
                "Take blocked by preflight: {}: {}",
                item.rule_id, item.message
            ),
        ))
    } else {
        Ok(())
    }
}

fn data_error(error: DataImportError) -> CommandError {
    CommandError::new(StatusCode::BAD_REQUEST, error.to_string())
}

#[allow(dead_code)]
fn _keep_schema_types(_: ProjectPackage, _: ProjectMetadata, _: RunRevisions, _: PreflightResult) {}

pub fn deterministic_cue_still(
    show: &ShowDefinition,
    cue: &Cue,
) -> cuecanvas_protocol::CueStillPayload {
    cuecanvas_protocol::CueStillPayload {
        cue_id: cue.id.clone(),
        frame: cue
            .template_states
            .first()
            .and_then(|state| {
                show.template_instances
                    .iter()
                    .find(|instance| instance.id == state.template_instance_id)
                    .map(|instance| instance.frame)
            })
            .unwrap_or(Rect {
                x: 0,
                y: 0,
                width: show.stage.width,
                height: show.stage.height,
            }),
        data_fingerprint: format!(
            "{}:{}:{}",
            show.revision,
            cue.id,
            cue.entity_refs
                .values()
                .map(|entity_ref| format!("{:?}:{}", entity_ref.entity_type, entity_ref.entity_id))
                .collect::<Vec<_>>()
                .join("|")
        ),
    }
}

pub fn built_in_cue_for_entity(
    show: &ShowDefinition,
    entity_type: EntityType,
    entity_id: String,
) -> Option<Cue> {
    let (instance, cue_type, label) = match &entity_type {
        EntityType::People => show
            .template_instances
            .iter()
            .find(|instance| {
                show.template_definitions.iter().any(|definition| {
                    definition.id == instance.template_definition_id
                        && matches!(
                            definition.kind,
                            TemplateKind::LowerThird | TemplateKind::SpeakerCard
                        )
                })
            })
            .map(|instance| (instance, TemplateKind::LowerThird, "person"))?,
        EntityType::Sessions => show
            .template_instances
            .iter()
            .find(|instance| {
                show.template_definitions.iter().any(|definition| {
                    definition.id == instance.template_definition_id
                        && definition.kind == TemplateKind::SessionTitle
                })
            })
            .map(|instance| (instance, TemplateKind::SessionTitle, "session"))?,
        EntityType::Sponsors => show
            .template_instances
            .iter()
            .find(|instance| {
                show.template_definitions.iter().any(|definition| {
                    definition.id == instance.template_definition_id
                        && definition.kind == TemplateKind::SponsorBug
                })
            })
            .map(|instance| (instance, TemplateKind::SponsorBug, "sponsor"))?,
    };

    Some(Cue {
        id: format!("cue-{}", Uuid::new_v4()),
        number: format!("{:03}", show.cue_sheet.cues.len() + 1),
        segment: Some("Generated".to_string()),
        name: format!("Generated {}", entity_id),
        mode: CueMode::Absolute,
        cue_type,
        entity_refs: BTreeMap::from([(
            label.to_string(),
            EntityRef {
                entity_type,
                entity_id,
            },
        )]),
        slot_overrides: BTreeMap::new(),
        template_states: vec![TemplateState {
            template_instance_id: instance.id.clone(),
            visible: true,
        }],
        transition: Transition::Cut,
        operator_notes: None,
        origin: Origin::local_user(),
        extensions: BTreeMap::new(),
    })
}
