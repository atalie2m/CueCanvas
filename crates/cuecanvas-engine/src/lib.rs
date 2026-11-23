use std::collections::{BTreeMap, BTreeSet};

use cuecanvas_model::*;
use serde_json::{Value, json};
use thiserror::Error;
use uuid::Uuid;

#[derive(Debug, Error, PartialEq, Eq)]
pub enum EngineError {
    #[error("cue not found: {0}")]
    CueNotFound(String),
    #[error("template instance not found: {0}")]
    TemplateInstanceNotFound(String),
    #[error("template definition not found: {0}")]
    TemplateDefinitionNotFound(String),
    #[error("preview snapshot belongs to a different run session")]
    PreviewSessionMismatch,
    #[error("preview revision mismatch")]
    PreviewRevisionMismatch,
    #[error("preview has blocking preflight errors")]
    BlockingPreflight,
    #[error("expected program revision {expected}, found {actual}")]
    ProgramRevisionMismatch { expected: u64, actual: u64 },
    #[error("no retained program snapshot exists")]
    NoRetainedProgram,
}

pub struct CueEngine;

impl CueEngine {
    pub fn preview_cue(
        show: &ShowDefinition,
        run: &RunSession,
        cue_id: &str,
        preview_revision: u64,
        now: impl Into<String>,
    ) -> Result<PreviewSnapshot, EngineError> {
        let cue = show
            .cue_sheet
            .cues
            .iter()
            .find(|cue| cue.id == cue_id)
            .ok_or_else(|| EngineError::CueNotFound(cue_id.to_string()))?;
        let mut preflight_items = vec![];
        let resolved = resolve_cue(show, cue, &mut preflight_items)?;
        let overlay_state = build_overlay_state(show, cue, &resolved, None, &mut preflight_items)?;

        Ok(PreviewSnapshot {
            id: format!("preview-{}", Uuid::new_v4()),
            run_session_id: run.id.clone(),
            show_definition_id: show.id.clone(),
            show_definition_revision: show.revision,
            source_cue_id: cue.id.clone(),
            preview_revision,
            created_at: now.into(),
            origin_chain: vec![cue.origin.clone()],
            resolved_data: resolved,
            overlay_state,
            preflight_result: PreflightResult {
                items: preflight_items,
            },
        })
    }

    pub fn take_preview(
        run: &mut RunSession,
        preview: &PreviewSnapshot,
        output_target_id: &str,
        expected_program_revision: u64,
        idempotency_key: &str,
        actor: Actor,
        now: impl Into<String>,
    ) -> Result<ProgramSnapshot, EngineError> {
        if let Some(existing) = run.live_command_results.get(idempotency_key) {
            return Ok(existing.clone());
        }
        if preview.run_session_id != run.id {
            return Err(EngineError::PreviewSessionMismatch);
        }
        if run.revisions.preview_revision != preview.preview_revision {
            return Err(EngineError::PreviewRevisionMismatch);
        }
        if run.revisions.program_revision != expected_program_revision {
            return Err(EngineError::ProgramRevisionMismatch {
                expected: expected_program_revision,
                actual: run.revisions.program_revision,
            });
        }
        if preview.preflight_result.has_blocking_error() {
            return Err(EngineError::BlockingPreflight);
        }

        let next_program_revision = run.revisions.program_revision + 1;
        let mut overlay_state = preview.overlay_state.clone();
        overlay_state.program_revision = Some(next_program_revision);

        let program = ProgramSnapshot {
            id: format!("program-{}", Uuid::new_v4()),
            run_session_id: run.id.clone(),
            show_definition_id: preview.show_definition_id.clone(),
            show_definition_revision: preview.show_definition_revision,
            source_preview_snapshot_id: preview.id.clone(),
            source_cue_id: preview.source_cue_id.clone(),
            program_revision: next_program_revision,
            created_at: now.into(),
            resolved_data: preview.resolved_data.clone(),
            asset_manifest: collect_asset_manifest(&overlay_state),
            overlay_state,
            origin_chain: preview.origin_chain.clone(),
            output_target_id: output_target_id.to_string(),
        };

        run.revisions.program_revision = next_program_revision;
        run.revisions.runtime_revision += 1;
        run.program_snapshot = Some(program.clone());
        run.live_command_results
            .insert(idempotency_key.to_string(), program.clone());
        push_log(
            run,
            actor,
            "live.take",
            format!(
                "Took PreviewSnapshot {} to ProgramRevision {}",
                preview.id, next_program_revision
            ),
            Some(preview.preview_revision),
            Some(next_program_revision),
            Some(idempotency_key.to_string()),
            program.created_at.clone(),
        );

        Ok(program)
    }

    pub fn clear_program(
        run: &mut RunSession,
        output_target_id: &str,
        idempotency_key: &str,
        actor: Actor,
        now: impl Into<String>,
    ) -> ProgramSnapshot {
        if let Some(existing) = run.live_command_results.get(idempotency_key) {
            return existing.clone();
        }

        let now = now.into();
        let next_program_revision = run.revisions.program_revision + 1;
        run.retained_program_snapshot = run.program_snapshot.clone();
        let overlay_state = OverlayState {
            protocol_version: 1,
            stage: run
                .program_snapshot
                .as_ref()
                .map(|snapshot| snapshot.overlay_state.stage.clone())
                .unwrap_or(Stage {
                    width: 1920,
                    height: 1080,
                }),
            program_revision: Some(next_program_revision),
            items: vec![OverlayItem::Clear],
        };
        let program = emergency_program(
            run,
            output_target_id,
            "clear",
            overlay_state,
            next_program_revision,
            now.clone(),
        );
        run.live_command_results
            .insert(idempotency_key.to_string(), program.clone());
        push_log(
            run,
            actor,
            "live.clear",
            "Cleared Program output".to_string(),
            None,
            Some(next_program_revision),
            Some(idempotency_key.to_string()),
            now,
        );
        program
    }

    pub fn blackout_program(
        run: &mut RunSession,
        output_target_id: &str,
        idempotency_key: &str,
        actor: Actor,
        now: impl Into<String>,
    ) -> ProgramSnapshot {
        if let Some(existing) = run.live_command_results.get(idempotency_key) {
            return existing.clone();
        }

        let now = now.into();
        let next_program_revision = run.revisions.program_revision + 1;
        run.retained_program_snapshot = run.program_snapshot.clone();
        let overlay_state = OverlayState {
            protocol_version: 1,
            stage: run
                .program_snapshot
                .as_ref()
                .map(|snapshot| snapshot.overlay_state.stage.clone())
                .unwrap_or(Stage {
                    width: 1920,
                    height: 1080,
                }),
            program_revision: Some(next_program_revision),
            items: vec![OverlayItem::Blackout],
        };
        let program = emergency_program(
            run,
            output_target_id,
            "blackout",
            overlay_state,
            next_program_revision,
            now.clone(),
        );
        run.live_command_results
            .insert(idempotency_key.to_string(), program.clone());
        push_log(
            run,
            actor,
            "live.blackout",
            "Blackout Program output".to_string(),
            None,
            Some(next_program_revision),
            Some(idempotency_key.to_string()),
            now,
        );
        program
    }

    pub fn restore_program(
        run: &mut RunSession,
        idempotency_key: &str,
        actor: Actor,
        now: impl Into<String>,
    ) -> Result<ProgramSnapshot, EngineError> {
        if let Some(existing) = run.live_command_results.get(idempotency_key) {
            return Ok(existing.clone());
        }

        let now = now.into();
        let retained = run
            .retained_program_snapshot
            .clone()
            .ok_or(EngineError::NoRetainedProgram)?;
        let next_program_revision = run.revisions.program_revision + 1;
        let mut restored = retained;
        restored.id = format!("program-{}", Uuid::new_v4());
        restored.program_revision = next_program_revision;
        restored.created_at = now.clone();
        restored.overlay_state.program_revision = Some(next_program_revision);

        run.revisions.program_revision = next_program_revision;
        run.revisions.runtime_revision += 1;
        run.program_snapshot = Some(restored.clone());
        run.live_command_results
            .insert(idempotency_key.to_string(), restored.clone());
        push_log(
            run,
            actor,
            "live.restoreProgram",
            format!("Restored ProgramRevision {}", next_program_revision),
            None,
            Some(next_program_revision),
            Some(idempotency_key.to_string()),
            now,
        );
        Ok(restored)
    }

    pub fn test_pattern_program(
        run: &mut RunSession,
        stage: Stage,
        output_target_id: &str,
        idempotency_key: &str,
        actor: Actor,
        now: impl Into<String>,
    ) -> ProgramSnapshot {
        if let Some(existing) = run.live_command_results.get(idempotency_key) {
            return existing.clone();
        }

        let now = now.into();
        let next_program_revision = run.revisions.program_revision + 1;
        let overlay_state = OverlayState {
            protocol_version: 1,
            stage: stage.clone(),
            program_revision: Some(next_program_revision),
            items: test_pattern_items(&stage),
        };
        let program = emergency_program(
            run,
            output_target_id,
            "test-pattern",
            overlay_state,
            next_program_revision,
            now.clone(),
        );
        run.live_command_results
            .insert(idempotency_key.to_string(), program.clone());
        push_log(
            run,
            actor,
            "live.testPattern",
            format!("Displayed Test Pattern at ProgramRevision {next_program_revision}"),
            None,
            Some(next_program_revision),
            Some(idempotency_key.to_string()),
            now,
        );
        program
    }
}

fn resolve_cue(
    show: &ShowDefinition,
    cue: &Cue,
    preflight_items: &mut Vec<PreflightItem>,
) -> Result<BTreeMap<String, Value>, EngineError> {
    let mut resolved = BTreeMap::new();
    for state in cue.template_states.iter().filter(|state| state.visible) {
        let instance = show
            .template_instances
            .iter()
            .find(|instance| instance.id == state.template_instance_id)
            .ok_or_else(|| {
                EngineError::TemplateInstanceNotFound(state.template_instance_id.clone())
            })?;
        let definition = show
            .template_definitions
            .iter()
            .find(|definition| definition.id == instance.template_definition_id)
            .ok_or_else(|| {
                EngineError::TemplateDefinitionNotFound(instance.template_definition_id.clone())
            })?;

        for slot in &definition.local_slots {
            let value = resolve_slot_value(show, cue, instance, slot);
            let value = cue
                .slot_overrides
                .get(&instance.id)
                .and_then(|overrides| overrides.get(&slot.key))
                .cloned()
                .or(value)
                .or_else(|| slot.fallback.clone());

            if slot.required && value.as_ref().is_none_or(is_empty_value) {
                preflight_items.push(PreflightItem {
                    rule_id: "slot.requiredMissing".to_string(),
                    source: Origin::system(),
                    severity: PreflightSeverity::Error,
                    scope: PreflightScope::TemplateInstance {
                        template_instance_id: instance.id.clone(),
                    },
                    message: format!("Required slot '{}' is missing", slot.label),
                    fix_actions: vec![],
                });
            }

            if let Some(value) = value {
                if let Some(max_length) = slot.max_length {
                    if value
                        .as_str()
                        .is_some_and(|text| text.chars().count() > max_length)
                    {
                        preflight_items.push(PreflightItem {
                            rule_id: "text.maxLength".to_string(),
                            source: Origin::system(),
                            severity: PreflightSeverity::Warning,
                            scope: PreflightScope::TemplateInstance {
                                template_instance_id: instance.id.clone(),
                            },
                            message: format!("Slot '{}' may overflow", slot.label),
                            fix_actions: vec![FixAction::SelectCue {
                                cue_id: cue.id.clone(),
                            }],
                        });
                    }
                }
                resolved.insert(format!("{}.{}", instance.id, slot.key), value);
            }
        }
    }
    Ok(resolved)
}

fn resolve_slot_value(
    show: &ShowDefinition,
    cue: &Cue,
    instance: &TemplateInstance,
    slot: &LocalSlot,
) -> Option<Value> {
    let binding = instance.slot_bindings.get(&slot.key)?;
    let entity_ref = cue
        .entity_refs
        .values()
        .find(|entity_ref| entity_ref.entity_type == binding.entity_type)?;

    match entity_ref.entity_type {
        EntityType::People => show
            .typed_entities
            .people
            .iter()
            .find(|person| person.id == entity_ref.entity_id)
            .and_then(|person| person_field(person, &binding.field)),
        EntityType::Sessions => show
            .typed_entities
            .sessions
            .iter()
            .find(|session| session.id == entity_ref.entity_id)
            .and_then(|session| session_field(session, &binding.field)),
        EntityType::Sponsors => show
            .typed_entities
            .sponsors
            .iter()
            .find(|sponsor| sponsor.id == entity_ref.entity_id)
            .and_then(|sponsor| sponsor_field(sponsor, &binding.field)),
    }
}

fn person_field(person: &Person, field: &str) -> Option<Value> {
    match field {
        "displayName" => Some(json!(person.display_name)),
        "role" => person.role.as_ref().map(|value| json!(value)),
        "organization" => person.organization.as_ref().map(|value| json!(value)),
        "photo" => person.photo.as_ref().map(|value| json!(value)),
        _ => None,
    }
}

fn session_field(session: &Session, field: &str) -> Option<Value> {
    match field {
        "title" => Some(json!(session.title)),
        "track" => session.track.as_ref().map(|value| json!(value)),
        "startTime" => session.start_time.as_ref().map(|value| json!(value)),
        _ => None,
    }
}

fn sponsor_field(sponsor: &Sponsor, field: &str) -> Option<Value> {
    match field {
        "name" => Some(json!(sponsor.name)),
        "logo" => sponsor.logo.as_ref().map(|value| json!(value)),
        "tier" => sponsor.tier.as_ref().map(|value| json!(value)),
        _ => None,
    }
}

fn build_overlay_state(
    show: &ShowDefinition,
    cue: &Cue,
    resolved: &BTreeMap<String, Value>,
    program_revision: Option<u64>,
    preflight_items: &mut Vec<PreflightItem>,
) -> Result<OverlayState, EngineError> {
    let mut items = vec![];
    let asset_ids: BTreeSet<_> = show.assets.iter().map(|asset| asset.id.as_str()).collect();

    for state in cue.template_states.iter().filter(|state| state.visible) {
        let instance = show
            .template_instances
            .iter()
            .find(|instance| instance.id == state.template_instance_id)
            .ok_or_else(|| {
                EngineError::TemplateInstanceNotFound(state.template_instance_id.clone())
            })?;
        let definition = show
            .template_definitions
            .iter()
            .find(|definition| definition.id == instance.template_definition_id)
            .ok_or_else(|| {
                EngineError::TemplateDefinitionNotFound(instance.template_definition_id.clone())
            })?;

        for (index, slot) in definition.local_slots.iter().enumerate() {
            let key = format!("{}.{}", instance.id, slot.key);
            let Some(value) = resolved.get(&key) else {
                continue;
            };
            match slot.kind {
                SlotKind::Text => {
                    let Some(text) = value.as_str() else {
                        continue;
                    };
                    let frame = text_slot_frame(instance.frame, index);
                    let style = TextStyle {
                        font_family: "Inter, system-ui, sans-serif".to_string(),
                        font_size: if index == 0 { 48 } else { 30 },
                        color: "#f8fafc".to_string(),
                        weight: if index == 0 {
                            FontWeight::Bold
                        } else {
                            FontWeight::Semibold
                        },
                    };
                    if text_overflows_frame(text, style.font_size, frame.width) {
                        preflight_items.push(PreflightItem {
                            rule_id: "renderer.textOverflow".to_string(),
                            source: Origin::system(),
                            severity: PreflightSeverity::Warning,
                            scope: PreflightScope::TemplateInstance {
                                template_instance_id: instance.id.clone(),
                            },
                            message: format!(
                                "Text in slot '{}' may overflow the renderer frame",
                                slot.label
                            ),
                            fix_actions: vec![FixAction::SelectCue {
                                cue_id: cue.id.clone(),
                            }],
                        });
                    }
                    items.push(OverlayItem::Text(OverlayText {
                        id: format!("{}-{}", instance.id, slot.key),
                        template_instance_id: instance.id.clone(),
                        slot_key: slot.key.clone(),
                        text: text.to_string(),
                        frame,
                        z_index: instance.z_index + index as i32,
                        style,
                    }));
                }
                SlotKind::AssetImage => {
                    let Some(asset_id) = value.as_str() else {
                        continue;
                    };
                    if !asset_ids.contains(asset_id) {
                        preflight_items.push(PreflightItem {
                            rule_id: "asset.missing".to_string(),
                            source: Origin::system(),
                            severity: PreflightSeverity::Error,
                            scope: PreflightScope::TemplateInstance {
                                template_instance_id: instance.id.clone(),
                            },
                            message: format!("Asset '{}' is missing", asset_id),
                            fix_actions: vec![FixAction::OpenAssetResolver],
                        });
                        continue;
                    }
                    items.push(OverlayItem::Image(OverlayImage {
                        id: format!("{}-{}", instance.id, slot.key),
                        template_instance_id: instance.id.clone(),
                        slot_key: slot.key.clone(),
                        asset_id: asset_id.to_string(),
                        frame: image_slot_frame(instance.frame),
                        z_index: instance.z_index + index as i32,
                        fit: ImageFit::Cover,
                    }));
                }
            }
        }
    }

    Ok(OverlayState {
        protocol_version: 1,
        stage: show.stage.clone(),
        program_revision,
        items,
    })
}

fn text_slot_frame(frame: Rect, slot_index: usize) -> Rect {
    Rect {
        x: frame.x + 184,
        y: frame.y + 32 + (slot_index.saturating_sub(1) as i32 * 48),
        width: frame.width.saturating_sub(216),
        height: if slot_index == 0 { 62 } else { 44 },
    }
}

fn image_slot_frame(frame: Rect) -> Rect {
    Rect {
        x: frame.x + 28,
        y: frame.y + 24,
        width: 128,
        height: 128,
    }
}

fn collect_asset_manifest(overlay_state: &OverlayState) -> Vec<String> {
    overlay_state
        .items
        .iter()
        .filter_map(|item| match item {
            OverlayItem::Image(image) => Some(image.asset_id.clone()),
            _ => None,
        })
        .collect()
}

fn text_overflows_frame(text: &str, font_size: u32, frame_width: u32) -> bool {
    let estimated_width = text.chars().count() as u32 * font_size / 2;
    estimated_width > frame_width
}

fn test_pattern_items(stage: &Stage) -> Vec<OverlayItem> {
    let colors = [
        "#ffffff", "#facc15", "#22c55e", "#06b6d4", "#3b82f6", "#a855f7", "#ef4444",
    ];
    let bar_width = stage.width / colors.len() as u32;
    let remainder = stage.width - (bar_width * colors.len() as u32);
    let mut items = colors
        .iter()
        .enumerate()
        .map(|(index, color)| {
            OverlayItem::Rect(OverlayRect {
                id: format!("test-pattern-bar-{index}"),
                frame: Rect {
                    x: (index as i32) * bar_width as i32,
                    y: 0,
                    width: if index == colors.len() - 1 {
                        bar_width + remainder
                    } else {
                        bar_width
                    },
                    height: stage.height,
                },
                z_index: index as i32,
                style: RectStyle {
                    fill: (*color).to_string(),
                    opacity: 100,
                },
            })
        })
        .collect::<Vec<_>>();
    items.push(OverlayItem::Text(OverlayText {
        id: "test-pattern-label".to_string(),
        template_instance_id: "test-pattern".to_string(),
        slot_key: "label".to_string(),
        text: "CueCanvas Test Pattern".to_string(),
        frame: Rect {
            x: (stage.width / 20) as i32,
            y: (stage.height.saturating_sub(stage.height / 5)) as i32,
            width: stage.width.saturating_sub(stage.width / 10),
            height: stage.height / 11,
        },
        z_index: 20,
        style: TextStyle {
            font_family: "Inter, system-ui, sans-serif".to_string(),
            font_size: 58,
            color: "#0f172a".to_string(),
            weight: FontWeight::Bold,
        },
    }));
    items
}

fn is_empty_value(value: &Value) -> bool {
    match value {
        Value::Null => true,
        Value::String(value) => value.trim().is_empty(),
        _ => false,
    }
}

fn emergency_program(
    run: &mut RunSession,
    output_target_id: &str,
    source_cue_id: &str,
    overlay_state: OverlayState,
    program_revision: u64,
    now: String,
) -> ProgramSnapshot {
    let program = ProgramSnapshot {
        id: format!("program-{}", Uuid::new_v4()),
        run_session_id: run.id.clone(),
        show_definition_id: run.show_definition_id.clone(),
        show_definition_revision: run.show_definition_revision,
        source_preview_snapshot_id: "emergency-operation".to_string(),
        source_cue_id: source_cue_id.to_string(),
        program_revision,
        created_at: now,
        resolved_data: BTreeMap::new(),
        asset_manifest: collect_asset_manifest(&overlay_state),
        overlay_state,
        origin_chain: vec![Origin::system()],
        output_target_id: output_target_id.to_string(),
    };
    run.revisions.program_revision = program_revision;
    run.revisions.runtime_revision += 1;
    run.program_snapshot = Some(program.clone());
    program
}

#[allow(clippy::too_many_arguments)]
fn push_log(
    run: &mut RunSession,
    actor: Actor,
    action: &str,
    message: String,
    preview_revision: Option<u64>,
    program_revision: Option<u64>,
    idempotency_key: Option<String>,
    at: String,
) {
    run.operation_log.push(OperationLogEntry {
        id: format!("op-{}", Uuid::new_v4()),
        at,
        actor,
        action: action.to_string(),
        message,
        preview_revision,
        program_revision,
        idempotency_key,
    });
}

pub fn demo_project() -> ProjectPackage {
    let show = demo_show();
    let run = demo_run(&show);
    ProjectPackage {
        format_version: 5,
        migrations_applied: vec!["v5-extension-readiness".to_string()],
        metadata: ProjectMetadata {
            id: "project-demo".to_string(),
            name: "CueCanvas Demo".to_string(),
            created_at: "2026-05-04T00:00:00Z".to_string(),
            updated_at: "2026-05-04T00:00:00Z".to_string(),
        },
        show_definitions: vec![show],
        run_sessions: vec![run],
        extension_registry: ExtensionRegistry {
            installed_plugins: vec![],
            known_origins: vec![KnownOrigin {
                origin: Origin::Plugin {
                    plugin_id: "com.example.future-pack".to_string(),
                    plugin_version: "1.0.0".to_string(),
                },
                status: OriginStatus::Missing,
            }],
        },
        settings: json!({}),
        extensions: BTreeMap::from([(
            "com.example.future-pack".to_string(),
            json!({ "preserved": true }),
        )]),
    }
}

pub fn built_in_template_definitions() -> Vec<TemplateDefinition> {
    vec![
        TemplateDefinition {
            id: "template-lower-third-basic".to_string(),
            name: "Lower Third".to_string(),
            kind: TemplateKind::LowerThird,
            version: 1,
            local_slots: vec![
                text_slot("primaryText", "Primary Text", true, Some(32)),
                text_slot("secondaryText", "Secondary Text", false, Some(48)),
                asset_slot("portrait", "Portrait", false),
            ],
            variants: default_variants(&["default", "compact"]),
            origin: Origin::system(),
            extensions: BTreeMap::new(),
        },
        TemplateDefinition {
            id: "template-session-title-basic".to_string(),
            name: "Session Title".to_string(),
            kind: TemplateKind::SessionTitle,
            version: 1,
            local_slots: vec![
                text_slot("title", "Title", true, Some(72)),
                text_slot("subtitle", "Subtitle", false, Some(72)),
                text_slot("time", "Time", false, Some(24)),
            ],
            variants: default_variants(&["default", "centered"]),
            origin: Origin::system(),
            extensions: BTreeMap::new(),
        },
        TemplateDefinition {
            id: "template-sponsor-bug-basic".to_string(),
            name: "Sponsor Bug".to_string(),
            kind: TemplateKind::SponsorBug,
            version: 1,
            local_slots: vec![
                asset_slot("logo", "Logo", true),
                text_slot("name", "Name", false, Some(32)),
                text_slot("tier", "Tier", false, Some(24)),
            ],
            variants: default_variants(&["default", "minimal"]),
            origin: Origin::system(),
            extensions: BTreeMap::new(),
        },
        TemplateDefinition {
            id: "template-break-screen-basic".to_string(),
            name: "Break Screen".to_string(),
            kind: TemplateKind::BreakScreen,
            version: 1,
            local_slots: vec![
                text_slot("headline", "Headline", true, Some(64)),
                text_slot("subhead", "Subhead", false, Some(96)),
                text_slot("resumeTime", "Resume Time", false, Some(24)),
            ],
            variants: default_variants(&["default", "quiet"]),
            origin: Origin::system(),
            extensions: BTreeMap::new(),
        },
        TemplateDefinition {
            id: "template-speaker-card-basic".to_string(),
            name: "Speaker Card".to_string(),
            kind: TemplateKind::SpeakerCard,
            version: 1,
            local_slots: vec![
                text_slot("name", "Name", true, Some(40)),
                text_slot("role", "Role", false, Some(56)),
                text_slot("organization", "Organization", false, Some(56)),
                asset_slot("photo", "Photo", false),
            ],
            variants: default_variants(&["default", "wide"]),
            origin: Origin::system(),
            extensions: BTreeMap::new(),
        },
    ]
}

pub fn ensure_builtin_templates(show: &mut ShowDefinition) {
    for definition in built_in_template_definitions() {
        if !show
            .template_definitions
            .iter()
            .any(|existing| existing.id == definition.id)
        {
            show.template_definitions.push(definition);
        }
    }
}

fn text_slot(key: &str, label: &str, required: bool, max_length: Option<usize>) -> LocalSlot {
    LocalSlot {
        key: key.to_string(),
        kind: SlotKind::Text,
        label: label.to_string(),
        required,
        max_length,
        fallback: None,
    }
}

fn asset_slot(key: &str, label: &str, required: bool) -> LocalSlot {
    LocalSlot {
        key: key.to_string(),
        kind: SlotKind::AssetImage,
        label: label.to_string(),
        required,
        max_length: None,
        fallback: None,
    }
}

fn default_variants(ids: &[&str]) -> Vec<TemplateVariant> {
    ids.iter()
        .map(|id| TemplateVariant {
            id: (*id).to_string(),
            name: id
                .split('-')
                .map(|part| {
                    let mut chars = part.chars();
                    match chars.next() {
                        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
                        None => String::new(),
                    }
                })
                .collect::<Vec<_>>()
                .join(" "),
        })
        .collect()
}

pub fn demo_show() -> ShowDefinition {
    let lower_third_id = "template-lower-third-basic".to_string();
    let instance_id = "instance-main-lower-third".to_string();
    ShowDefinition {
        id: "show-main".to_string(),
        revision: 1,
        name: "Main Event".to_string(),
        stage: Stage {
            width: 1920,
            height: 1080,
        },
        output_targets: vec![OutputTarget {
            id: "output-program-obs".to_string(),
            name: "OBS Program Overlay".to_string(),
            kind: OutputTargetKind::ObsBrowserSource,
            stage: Stage {
                width: 1920,
                height: 1080,
            },
            overlay_url: "http://127.0.0.1:4317/overlay/program".to_string(),
            origin: Origin::system(),
            extensions: BTreeMap::new(),
        }],
        assets: vec![
            Asset {
                id: "asset-cat-photo".to_string(),
                kind: AssetKind::Image,
                path: "assets/images/cat.png".to_string(),
                origin: Origin::Import {
                    source: "demo-fixture".to_string(),
                },
                extensions: BTreeMap::new(),
            },
            Asset {
                id: "asset-dog-photo".to_string(),
                kind: AssetKind::Image,
                path: "assets/images/dog.png".to_string(),
                origin: Origin::Import {
                    source: "demo-fixture".to_string(),
                },
                extensions: BTreeMap::new(),
            },
        ],
        template_definitions: built_in_template_definitions(),
        template_instances: vec![TemplateInstance {
            id: instance_id.clone(),
            name: "Main Lower Third".to_string(),
            template_definition_id: lower_third_id,
            template_version_policy: TemplateVersionPolicy::LatestCompatible,
            frame: Rect {
                x: 120,
                y: 760,
                width: 900,
                height: 184,
            },
            z_index: 10,
            variant_id: "default".to_string(),
            slot_bindings: BTreeMap::from([
                (
                    "primaryText".to_string(),
                    SlotBinding {
                        entity_type: EntityType::People,
                        field: "displayName".to_string(),
                    },
                ),
                (
                    "secondaryText".to_string(),
                    SlotBinding {
                        entity_type: EntityType::People,
                        field: "role".to_string(),
                    },
                ),
                (
                    "portrait".to_string(),
                    SlotBinding {
                        entity_type: EntityType::People,
                        field: "photo".to_string(),
                    },
                ),
            ]),
            visible_by_default: false,
            origin: Origin::system(),
            extensions: BTreeMap::new(),
        }],
        typed_entities: TypedEntities {
            people: vec![
                Person {
                    id: "person-cat".to_string(),
                    display_name: "Cat".to_string(),
                    role: Some("CTO".to_string()),
                    organization: Some("CueCanvas".to_string()),
                    photo: Some("asset-cat-photo".to_string()),
                    origin: Origin::Import {
                        source: "demo-fixture".to_string(),
                    },
                    extensions: BTreeMap::new(),
                },
                Person {
                    id: "person-dog".to_string(),
                    display_name: "Dog".to_string(),
                    role: Some("CFO".to_string()),
                    organization: Some("CueCanvas".to_string()),
                    photo: Some("asset-dog-photo".to_string()),
                    origin: Origin::Import {
                        source: "demo-fixture".to_string(),
                    },
                    extensions: BTreeMap::new(),
                },
            ],
            sessions: vec![Session {
                id: "session-opening".to_string(),
                title: "Opening".to_string(),
                track: Some("Main".to_string()),
                start_time: Some("09:00".to_string()),
                speaker_refs: vec!["person-cat".to_string()],
                origin: Origin::Import {
                    source: "demo-fixture".to_string(),
                },
                extensions: BTreeMap::new(),
            }],
            sponsors: vec![Sponsor {
                id: "sponsor-acme".to_string(),
                name: "ACME".to_string(),
                logo: None,
                tier: Some("Gold".to_string()),
                origin: Origin::Import {
                    source: "demo-fixture".to_string(),
                },
                extensions: BTreeMap::new(),
            }],
        },
        data_tables: vec![],
        cue_sheet: CueSheet {
            cues: vec![
                demo_cue(
                    "cue-001",
                    "001",
                    "CTO Cat Lower Third",
                    "person-cat",
                    &instance_id,
                ),
                demo_cue(
                    "cue-002",
                    "002",
                    "CFO Dog Lower Third",
                    "person-dog",
                    &instance_id,
                ),
            ],
        },
        show_settings: json!({}),
        origin: Origin::system(),
        extensions: BTreeMap::new(),
    }
}

fn demo_cue(
    id: &str,
    number: &str,
    name: &str,
    person_id: &str,
    template_instance_id: &str,
) -> Cue {
    Cue {
        id: id.to_string(),
        number: number.to_string(),
        segment: Some("Demo".to_string()),
        name: name.to_string(),
        mode: CueMode::Absolute,
        cue_type: TemplateKind::LowerThird,
        entity_refs: BTreeMap::from([(
            "person".to_string(),
            EntityRef {
                entity_type: EntityType::People,
                entity_id: person_id.to_string(),
            },
        )]),
        slot_overrides: BTreeMap::new(),
        template_states: vec![TemplateState {
            template_instance_id: template_instance_id.to_string(),
            visible: true,
        }],
        transition: Transition::Cut,
        operator_notes: Some("Take after speaker reaches mark".to_string()),
        origin: Origin::local_user(),
        extensions: BTreeMap::from([(
            "com.example.future-pack".to_string(),
            json!({ "sourceRow": number }),
        )]),
    }
}

pub fn demo_run(show: &ShowDefinition) -> RunSession {
    RunSession {
        id: "run-main-rehearsal".to_string(),
        show_definition_id: show.id.clone(),
        show_definition_revision: show.revision,
        live_state: LiveState::Rehearsal,
        preview_snapshot: None,
        program_snapshot: None,
        retained_program_snapshot: None,
        output_connection_state: ConnectionState::Disconnected,
        obs_health: HealthStatus {
            status: HealthLevel::Unknown,
            messages: vec![],
        },
        overlay_health: HealthStatus {
            status: HealthLevel::Unknown,
            messages: vec![],
        },
        warning_overrides: BTreeMap::new(),
        operation_log: vec![],
        revisions: RunRevisions {
            project_revision: 1,
            show_definition_revision: show.revision,
            data_revision: 1,
            preview_revision: 0,
            program_revision: 0,
            runtime_revision: 1,
        },
        live_command_results: BTreeMap::new(),
        run_settings: json!({}),
        extensions: BTreeMap::new(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const NOW: &str = "2026-05-04T00:00:00Z";

    #[test]
    fn direct_cue_resolution_is_deterministic() {
        let project = demo_project();
        let show = &project.show_definitions[0];
        let run = &project.run_sessions[0];

        let first = CueEngine::preview_cue(show, run, "cue-001", 1, NOW).unwrap();
        let second = CueEngine::preview_cue(show, run, "cue-001", 1, NOW).unwrap();

        assert_eq!(first.resolved_data, second.resolved_data);
        assert_eq!(first.overlay_state.items, second.overlay_state.items);
    }

    #[test]
    fn built_in_templates_cover_mvp_template_kinds() {
        let definitions = built_in_template_definitions();
        let kinds = definitions
            .iter()
            .map(|definition| definition.kind.clone())
            .collect::<Vec<_>>();

        assert!(kinds.contains(&TemplateKind::LowerThird));
        assert!(kinds.contains(&TemplateKind::SessionTitle));
        assert!(kinds.contains(&TemplateKind::SponsorBug));
        assert!(kinds.contains(&TemplateKind::BreakScreen));
        assert!(kinds.contains(&TemplateKind::SpeakerCard));
        assert!(
            definitions
                .iter()
                .all(|definition| !definition.local_slots.is_empty())
        );
    }

    #[test]
    fn take_commits_preview_without_reresolving_cue() {
        let mut project = demo_project();
        let show = project.show_definitions[0].clone();
        let run = &mut project.run_sessions[0];
        run.revisions.preview_revision = 1;

        let preview = CueEngine::preview_cue(&show, run, "cue-001", 1, NOW).unwrap();
        run.preview_snapshot = Some(preview.clone());

        project.show_definitions[0].typed_entities.people[0].display_name =
            "Edited Later".to_string();

        let program = CueEngine::take_preview(
            run,
            &preview,
            "output-program-obs",
            0,
            "take-001",
            Actor::local_user(),
            "2026-05-04T00:00:01Z",
        )
        .unwrap();

        assert_eq!(
            program.resolved_data["instance-main-lower-third.primaryText"],
            json!("Cat")
        );
        assert_eq!(program.source_preview_snapshot_id, preview.id);
    }

    #[test]
    fn take_is_idempotent_by_key() {
        let project = demo_project();
        let show = &project.show_definitions[0];
        let mut run = project.run_sessions[0].clone();
        run.revisions.preview_revision = 1;
        let preview = CueEngine::preview_cue(show, &run, "cue-001", 1, NOW).unwrap();

        let first = CueEngine::take_preview(
            &mut run,
            &preview,
            "output-program-obs",
            0,
            "take-001",
            Actor::local_user(),
            "2026-05-04T00:00:01Z",
        )
        .unwrap();
        let second = CueEngine::take_preview(
            &mut run,
            &preview,
            "output-program-obs",
            0,
            "take-001",
            Actor::local_user(),
            "2026-05-04T00:00:02Z",
        )
        .unwrap();

        assert_eq!(first, second);
        assert_eq!(run.operation_log.len(), 1);
    }

    #[test]
    fn preview_reports_renderer_text_overflow_warning() {
        let mut project = demo_project();
        project.show_definitions[0].typed_entities.people[0].display_name =
            "A very long lower-third primary speaker name that should overflow the frame"
                .to_string();
        let show = &project.show_definitions[0];
        let mut run = project.run_sessions[0].clone();
        run.revisions.preview_revision = 1;

        let preview = CueEngine::preview_cue(show, &run, "cue-001", 1, NOW).unwrap();

        assert!(preview.preflight_result.items.iter().any(|item| {
            item.rule_id == "renderer.textOverflow" && item.severity == PreflightSeverity::Warning
        }));
    }

    #[test]
    fn clear_blackout_and_restore_retain_program() {
        let project = demo_project();
        let show = &project.show_definitions[0];
        let mut run = project.run_sessions[0].clone();
        run.revisions.preview_revision = 1;
        let preview = CueEngine::preview_cue(show, &run, "cue-001", 1, NOW).unwrap();
        let original = CueEngine::take_preview(
            &mut run,
            &preview,
            "output-program-obs",
            0,
            "take-001",
            Actor::local_user(),
            "2026-05-04T00:00:01Z",
        )
        .unwrap();

        CueEngine::blackout_program(
            &mut run,
            "output-program-obs",
            "blackout-001",
            Actor::local_user(),
            "2026-05-04T00:00:02Z",
        );
        let restored = CueEngine::restore_program(
            &mut run,
            "restore-001",
            Actor::local_user(),
            "2026-05-04T00:00:03Z",
        )
        .unwrap();

        assert_eq!(restored.resolved_data, original.resolved_data);
        assert!(matches!(
            run.retained_program_snapshot
                .as_ref()
                .unwrap()
                .overlay_state
                .items[0],
            OverlayItem::Text(_)
        ));
    }
}
