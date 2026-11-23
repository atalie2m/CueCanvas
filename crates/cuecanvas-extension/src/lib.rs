use std::{collections::BTreeSet, path::Path};

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginManifest {
    pub manifest_version: u32,
    pub plugin_id: String,
    pub name: String,
    pub version: String,
    pub publisher: String,
    pub kind: PluginKind,
    pub minimum_cue_canvas_version: String,
    pub capabilities: Vec<Capability>,
    pub contributes: DeclarativeContributions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PluginKind {
    DeclarativePack,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum Capability {
    ContributeTemplates,
    ContributeAssets,
    ContributeCsvMappings,
    ContributeShowStarters,
    WriteTypedEntities,
    CreateCues,
    EmitPreflightResults,
    ReadShowDefinition,
    ReadRunSession,
}

#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeclarativeContributions {
    #[serde(default)]
    pub templates: Vec<String>,
    #[serde(default)]
    pub assets: Vec<String>,
    #[serde(default)]
    pub csv_mappings: Vec<String>,
    #[serde(default)]
    pub show_starters: Vec<String>,
}

#[derive(Debug, Error, PartialEq, Eq)]
pub enum ManifestError {
    #[error("unsupported manifest version {0}")]
    UnsupportedManifestVersion(u32),
    #[error("plugin id is invalid")]
    InvalidPluginId,
    #[error("plugin version is invalid")]
    InvalidVersion,
    #[error("contribution path must be relative and must not traverse outside the pack: {0}")]
    UnsafePath(String),
    #[error("missing capability {capability:?} for contribution {contribution}")]
    MissingCapability {
        capability: Capability,
        contribution: &'static str,
    },
}

pub struct ManifestValidator;

impl ManifestValidator {
    pub fn validate(manifest: &PluginManifest) -> Result<(), ManifestError> {
        if manifest.manifest_version != 1 {
            return Err(ManifestError::UnsupportedManifestVersion(
                manifest.manifest_version,
            ));
        }
        if !is_valid_plugin_id(&manifest.plugin_id) {
            return Err(ManifestError::InvalidPluginId);
        }
        if !looks_like_semver(&manifest.version) {
            return Err(ManifestError::InvalidVersion);
        }

        for path in manifest.contributes.all_paths() {
            if !is_safe_relative_path(path) {
                return Err(ManifestError::UnsafePath(path.to_string()));
            }
        }

        let capabilities: BTreeSet<_> = manifest.capabilities.iter().cloned().collect();
        require_capability(
            &capabilities,
            !manifest.contributes.templates.is_empty(),
            Capability::ContributeTemplates,
            "templates",
        )?;
        require_capability(
            &capabilities,
            !manifest.contributes.assets.is_empty(),
            Capability::ContributeAssets,
            "assets",
        )?;
        require_capability(
            &capabilities,
            !manifest.contributes.csv_mappings.is_empty(),
            Capability::ContributeCsvMappings,
            "csvMappings",
        )?;
        require_capability(
            &capabilities,
            !manifest.contributes.show_starters.is_empty(),
            Capability::ContributeShowStarters,
            "showStarters",
        )?;

        Ok(())
    }
}

impl DeclarativeContributions {
    fn all_paths(&self) -> impl Iterator<Item = &str> {
        self.templates
            .iter()
            .chain(self.assets.iter())
            .chain(self.csv_mappings.iter())
            .chain(self.show_starters.iter())
            .map(String::as_str)
    }
}

fn require_capability(
    capabilities: &BTreeSet<Capability>,
    condition: bool,
    capability: Capability,
    contribution: &'static str,
) -> Result<(), ManifestError> {
    if condition && !capabilities.contains(&capability) {
        return Err(ManifestError::MissingCapability {
            capability,
            contribution,
        });
    }
    Ok(())
}

fn is_valid_plugin_id(plugin_id: &str) -> bool {
    plugin_id.split('.').all(|part| {
        !part.is_empty()
            && part
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '-')
    }) && plugin_id.contains('.')
}

fn looks_like_semver(version: &str) -> bool {
    let parts: Vec<_> = version.split('.').collect();
    parts.len() == 3 && parts.iter().all(|part| part.parse::<u64>().is_ok())
}

fn is_safe_relative_path(path: &str) -> bool {
    let path = Path::new(path);
    path.is_relative()
        && path
            .components()
            .all(|component| matches!(component, std::path::Component::Normal(_)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declarative_pack_manifest_validates_capabilities_and_paths() {
        let manifest = PluginManifest {
            manifest_version: 1,
            plugin_id: "com.example.event-pack".to_string(),
            name: "Example Event Pack".to_string(),
            version: "1.0.0".to_string(),
            publisher: "Example Studio".to_string(),
            kind: PluginKind::DeclarativePack,
            minimum_cue_canvas_version: "0.3.0".to_string(),
            capabilities: vec![
                Capability::ContributeTemplates,
                Capability::ContributeAssets,
                Capability::ContributeCsvMappings,
            ],
            contributes: DeclarativeContributions {
                templates: vec!["templates/lower-third.json".to_string()],
                assets: vec!["assets/logo.png".to_string()],
                csv_mappings: vec!["mappings/speakers-basic.json".to_string()],
                show_starters: vec![],
            },
        };

        assert_eq!(ManifestValidator::validate(&manifest), Ok(()));
    }

    #[test]
    fn manifest_rejects_path_traversal() {
        let manifest = PluginManifest {
            manifest_version: 1,
            plugin_id: "com.example.bad".to_string(),
            name: "Bad".to_string(),
            version: "1.0.0".to_string(),
            publisher: "Example".to_string(),
            kind: PluginKind::DeclarativePack,
            minimum_cue_canvas_version: "0.3.0".to_string(),
            capabilities: vec![Capability::ContributeTemplates],
            contributes: DeclarativeContributions {
                templates: vec!["../escape.json".to_string()],
                ..Default::default()
            },
        };

        assert!(matches!(
            ManifestValidator::validate(&manifest),
            Err(ManifestError::UnsafePath(_))
        ));
    }

    #[test]
    fn manifest_rejects_missing_capability() {
        let manifest = PluginManifest {
            manifest_version: 1,
            plugin_id: "com.example.bad".to_string(),
            name: "Bad".to_string(),
            version: "1.0.0".to_string(),
            publisher: "Example".to_string(),
            kind: PluginKind::DeclarativePack,
            minimum_cue_canvas_version: "0.3.0".to_string(),
            capabilities: vec![],
            contributes: DeclarativeContributions {
                templates: vec!["templates/lower-third.json".to_string()],
                ..Default::default()
            },
        };

        assert!(matches!(
            ManifestValidator::validate(&manifest),
            Err(ManifestError::MissingCapability {
                capability: Capability::ContributeTemplates,
                ..
            })
        ));
    }
}
