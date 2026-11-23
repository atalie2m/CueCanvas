use std::collections::BTreeSet;

use cuecanvas_model::{
    ConnectionState, EntityType, ExtensionRegistry, FixAction, HealthLevel, Origin, OriginStatus,
    PreflightItem, PreflightResult, PreflightScope, PreflightSeverity, PreviewSnapshot, RunSession,
    ShowDefinition, SlotKind,
};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PreflightPolicy {
    pub block_take_when_output_disconnected: bool,
    pub warn_on_missing_plugin_origin: bool,
}

impl Default for PreflightPolicy {
    fn default() -> Self {
        Self {
            block_take_when_output_disconnected: true,
            warn_on_missing_plugin_origin: true,
        }
    }
}

pub struct PreflightEngine;

impl PreflightEngine {
    pub fn run_show(
        show: &ShowDefinition,
        run: &RunSession,
        policy: &PreflightPolicy,
    ) -> PreflightResult {
        let mut items = vec![];

        if show.output_targets.is_empty() {
            items.push(PreflightItem {
                rule_id: "outputTarget.missing".to_string(),
                source: Origin::system(),
                severity: PreflightSeverity::Error,
                scope: PreflightScope::Show,
                message: "Show has no output target".to_string(),
                fix_actions: vec![],
            });
        }

        if policy.block_take_when_output_disconnected
            && run.output_connection_state == ConnectionState::Disconnected
        {
            items.push(PreflightItem {
                rule_id: "output.disconnected".to_string(),
                source: Origin::system(),
                severity: PreflightSeverity::Error,
                scope: show
                    .output_targets
                    .first()
                    .map(|target| PreflightScope::OutputTarget {
                        output_target_id: target.id.clone(),
                    })
                    .unwrap_or(PreflightScope::Show),
                message: "Program output is not connected".to_string(),
                fix_actions: vec![],
            });
        }

        if run.obs_health.status == HealthLevel::Warning
            || run.overlay_health.status == HealthLevel::Warning
        {
            let health_messages = run
                .obs_health
                .messages
                .iter()
                .chain(run.overlay_health.messages.iter())
                .cloned()
                .collect::<Vec<_>>();
            items.push(PreflightItem {
                rule_id: "output.healthWarning".to_string(),
                source: Origin::system(),
                severity: PreflightSeverity::Warning,
                scope: PreflightScope::Show,
                message: if health_messages.is_empty() {
                    "OBS or Overlay health has warnings".to_string()
                } else {
                    format!(
                        "OBS or Overlay health warning: {}",
                        health_messages.join("; ")
                    )
                },
                fix_actions: vec![],
            });
        }

        items.extend(validate_template_instances(show));
        items.extend(validate_asset_references(show));

        PreflightResult { items }
    }

    pub fn run_preview(
        run: &RunSession,
        preview: &PreviewSnapshot,
        current_show_revision: u64,
    ) -> PreflightResult {
        let mut items = preview.preflight_result.items.clone();

        if preview.run_session_id != run.id {
            items.push(PreflightItem {
                rule_id: "preview.sessionMismatch".to_string(),
                source: Origin::system(),
                severity: PreflightSeverity::Error,
                scope: PreflightScope::Cue {
                    cue_id: preview.source_cue_id.clone(),
                },
                message: "PreviewSnapshot belongs to a different RunSession".to_string(),
                fix_actions: vec![FixAction::RefreshPreview],
            });
        }

        if preview.show_definition_revision != current_show_revision {
            items.push(PreflightItem {
                rule_id: "preview.staleShowRevision".to_string(),
                source: Origin::system(),
                severity: PreflightSeverity::Warning,
                scope: PreflightScope::Cue {
                    cue_id: preview.source_cue_id.clone(),
                },
                message: "PreviewSnapshot was generated from an older ShowDefinition revision"
                    .to_string(),
                fix_actions: vec![FixAction::RefreshPreview],
            });
        }

        PreflightResult { items }
    }

    pub fn missing_plugin_origin_warnings(
        registry: &ExtensionRegistry,
        origin_chain: &[Origin],
    ) -> Vec<PreflightItem> {
        origin_chain
            .iter()
            .filter_map(|origin| match origin {
                Origin::Plugin {
                    plugin_id,
                    plugin_version,
                } => {
                    let missing = registry.known_origins.iter().any(|known| {
                        known.origin == *origin && known.status == OriginStatus::Missing
                    });
                    missing.then(|| PreflightItem {
                        rule_id: "extension.pluginOriginMissing".to_string(),
                        source: Origin::system(),
                        severity: PreflightSeverity::Warning,
                        scope: PreflightScope::Show,
                        message: format!(
                            "Plugin origin {plugin_id} {plugin_version} is not installed; artifact is preserved"
                        ),
                        fix_actions: vec![],
                    })
                }
                _ => None,
            })
            .collect()
    }
}

fn validate_template_instances(show: &ShowDefinition) -> Vec<PreflightItem> {
    let mut items = Vec::new();

    for instance in &show.template_instances {
        let Some(definition) = show
            .template_definitions
            .iter()
            .find(|definition| definition.id == instance.template_definition_id)
        else {
            items.push(PreflightItem {
                rule_id: "template.definitionMissing".to_string(),
                source: Origin::system(),
                severity: PreflightSeverity::Error,
                scope: PreflightScope::TemplateInstance {
                    template_instance_id: instance.id.clone(),
                },
                message: format!(
                    "Template definition '{}' is missing",
                    instance.template_definition_id
                ),
                fix_actions: vec![],
            });
            continue;
        };

        if !definition
            .variants
            .iter()
            .any(|variant| variant.id == instance.variant_id)
        {
            items.push(PreflightItem {
                rule_id: "template.variantInvalid".to_string(),
                source: Origin::system(),
                severity: PreflightSeverity::Error,
                scope: PreflightScope::TemplateInstance {
                    template_instance_id: instance.id.clone(),
                },
                message: format!("Variant '{}' is not available", instance.variant_id),
                fix_actions: vec![],
            });
        }

        if instance.frame.width == 0
            || instance.frame.height == 0
            || instance.frame.x < 0
            || instance.frame.y < 0
            || instance.frame.x as u32 + instance.frame.width > show.stage.width
            || instance.frame.y as u32 + instance.frame.height > show.stage.height
        {
            items.push(PreflightItem {
                rule_id: "template.frameUnsafe".to_string(),
                source: Origin::system(),
                severity: PreflightSeverity::Error,
                scope: PreflightScope::TemplateInstance {
                    template_instance_id: instance.id.clone(),
                },
                message: "Template frame must fit inside the stage".to_string(),
                fix_actions: vec![],
            });
        }

        for slot in &definition.local_slots {
            if slot.required
                && slot.fallback.is_none()
                && !instance.slot_bindings.contains_key(&slot.key)
            {
                items.push(PreflightItem {
                    rule_id: "template.slotBindingMissing".to_string(),
                    source: Origin::system(),
                    severity: PreflightSeverity::Error,
                    scope: PreflightScope::TemplateInstance {
                        template_instance_id: instance.id.clone(),
                    },
                    message: format!("Required slot '{}' is not bound", slot.label),
                    fix_actions: vec![],
                });
            }

            if let Some(binding) = instance.slot_bindings.get(&slot.key) {
                if !field_allowed(&binding.entity_type, &binding.field) {
                    items.push(PreflightItem {
                        rule_id: "template.slotBindingInvalid".to_string(),
                        source: Origin::system(),
                        severity: PreflightSeverity::Error,
                        scope: PreflightScope::TemplateInstance {
                            template_instance_id: instance.id.clone(),
                        },
                        message: format!(
                            "Slot '{}' is bound to unsupported field '{}'",
                            slot.label, binding.field
                        ),
                        fix_actions: vec![],
                    });
                }
            }
        }
    }

    for cue in &show.cue_sheet.cues {
        for (instance_id, overrides) in &cue.slot_overrides {
            let Some(instance) = show
                .template_instances
                .iter()
                .find(|instance| &instance.id == instance_id)
            else {
                continue;
            };
            let Some(definition) = show
                .template_definitions
                .iter()
                .find(|definition| definition.id == instance.template_definition_id)
            else {
                continue;
            };
            for slot in definition
                .local_slots
                .iter()
                .filter(|slot| matches!(slot.kind, SlotKind::AssetImage))
            {
                if let Some(asset_id) = overrides.get(&slot.key).and_then(|value| value.as_str()) {
                    if !show.assets.iter().any(|asset| asset.id == asset_id) {
                        items.push(PreflightItem {
                            rule_id: "template.assetOverrideMissing".to_string(),
                            source: Origin::system(),
                            severity: PreflightSeverity::Error,
                            scope: PreflightScope::Cue {
                                cue_id: cue.id.clone(),
                            },
                            message: format!("Cue references missing asset '{}'", asset_id),
                            fix_actions: vec![FixAction::OpenAssetResolver],
                        });
                    }
                }
            }
        }
    }

    items
}

fn validate_asset_references(show: &ShowDefinition) -> Vec<PreflightItem> {
    let mut items = Vec::new();
    let asset_ids = show
        .assets
        .iter()
        .map(|asset| asset.id.as_str())
        .collect::<BTreeSet<_>>();

    for person in &show.typed_entities.people {
        if let Some(photo) = person.photo.as_deref() {
            if !asset_ids.contains(photo) {
                items.push(missing_asset_item(
                    format!(
                        "Person '{}' references missing photo '{}'",
                        person.id, photo
                    ),
                    PreflightScope::Show,
                ));
            }
        }
    }
    for sponsor in &show.typed_entities.sponsors {
        if let Some(logo) = sponsor.logo.as_deref() {
            if !asset_ids.contains(logo) {
                items.push(missing_asset_item(
                    format!(
                        "Sponsor '{}' references missing logo '{}'",
                        sponsor.id, logo
                    ),
                    PreflightScope::Show,
                ));
            }
        }
    }

    items
}

fn missing_asset_item(message: String, scope: PreflightScope) -> PreflightItem {
    PreflightItem {
        rule_id: "asset.referenceMissing".to_string(),
        source: Origin::system(),
        severity: PreflightSeverity::Error,
        scope,
        message,
        fix_actions: vec![FixAction::OpenAssetResolver],
    }
}

fn field_allowed(entity_type: &EntityType, field: &str) -> bool {
    match entity_type {
        EntityType::People => matches!(field, "displayName" | "role" | "organization" | "photo"),
        EntityType::Sessions => matches!(field, "title" | "track" | "startTime"),
        EntityType::Sponsors => matches!(field, "name" | "logo" | "tier"),
    }
}

#[cfg(test)]
mod tests {
    use cuecanvas_engine::{CueEngine, demo_project};
    use cuecanvas_model::{Actor, ConnectionState, Origin};

    use super::*;

    #[test]
    fn disconnected_output_is_blocking_for_show_preflight() {
        let project = demo_project();
        let result = PreflightEngine::run_show(
            &project.show_definitions[0],
            &project.run_sessions[0],
            &PreflightPolicy::default(),
        );

        assert!(result.items.iter().any(|item| {
            item.rule_id == "output.disconnected" && item.severity == PreflightSeverity::Error
        }));
    }

    #[test]
    fn stale_preview_is_warning_not_reresolve() {
        let project = demo_project();
        let show = &project.show_definitions[0];
        let mut run = project.run_sessions[0].clone();
        run.output_connection_state = ConnectionState::Connected;
        run.revisions.preview_revision = 1;
        let preview = CueEngine::preview_cue(show, &run, "cue-001", 1, "now").unwrap();

        let result = PreflightEngine::run_preview(&run, &preview, show.revision + 1);

        assert!(result.items.iter().any(|item| {
            item.rule_id == "preview.staleShowRevision"
                && item.severity == PreflightSeverity::Warning
        }));

        let program = CueEngine::take_preview(
            &mut run,
            &preview,
            "output-program-obs",
            0,
            "take-stale-preview",
            Actor::local_user(),
            "later",
        )
        .unwrap();
        assert_eq!(program.source_preview_snapshot_id, preview.id);
    }

    #[test]
    fn missing_plugin_origin_preserves_artifact_with_warning() {
        let project = demo_project();
        let warnings = PreflightEngine::missing_plugin_origin_warnings(
            &project.extension_registry,
            &[Origin::Plugin {
                plugin_id: "com.example.future-pack".to_string(),
                plugin_version: "1.0.0".to_string(),
            }],
        );

        assert_eq!(warnings.len(), 1);
        assert_eq!(warnings[0].severity, PreflightSeverity::Warning);
    }

    #[test]
    fn invalid_template_instances_are_preflight_errors() {
        let mut project = demo_project();
        let run = project.run_sessions[0].clone();
        let show = &mut project.show_definitions[0];
        show.template_instances[0].variant_id = "missing".to_string();
        show.template_instances[0].frame.width = show.stage.width + 1;

        let result = PreflightEngine::run_show(show, &run, &PreflightPolicy::default());

        assert!(result.items.iter().any(|item| {
            item.rule_id == "template.variantInvalid" && item.severity == PreflightSeverity::Error
        }));
        assert!(result.items.iter().any(|item| {
            item.rule_id == "template.frameUnsafe" && item.severity == PreflightSeverity::Error
        }));
    }
}
