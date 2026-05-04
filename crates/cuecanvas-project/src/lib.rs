use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

use cuecanvas_model::{
    ExtensionRegistry, ProjectMetadata, ProjectPackage, RunSession, ShowDefinition,
};
use serde::{Deserialize, Serialize};
use serde_json::Value;
use thiserror::Error;

const SUPPORTED_FORMAT_VERSION: u32 = 5;

#[derive(Debug, Error)]
pub enum ProjectIoError {
    #[error("io error at {path}: {source}")]
    Io {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("json error at {path}: {source}")]
    Json {
        path: PathBuf,
        source: serde_json::Error,
    },
    #[error("unsupported package format version {found}; supported version is {supported}")]
    UnsupportedFormat { found: u32, supported: u32 },
    #[error("invalid package path {path}: {message}")]
    InvalidPath { path: PathBuf, message: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageManifest {
    pub format: String,
    pub format_version: u32,
    pub created_by: String,
    pub minimum_runtime_version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProjectRootFile {
    pub format_version: u32,
    pub migrations_applied: Vec<String>,
    pub metadata: ProjectMetadata,
    pub show_definitions: Vec<PackageFileRef>,
    pub run_sessions: Vec<PackageFileRef>,
    #[serde(default)]
    pub settings: Value,
    #[serde(default)]
    pub extensions: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageFileRef {
    pub id: String,
    pub path: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryInfo {
    pub project_id: String,
    pub package_format_version: u32,
    pub latest_run_session_id: Option<String>,
    pub latest_program_revision: Option<u64>,
    pub retained_program_revision: Option<u64>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationResult {
    pub from_version: u32,
    pub to_version: u32,
    pub migrations_applied: Vec<String>,
}

pub fn save_project_package(
    package_dir: impl AsRef<Path>,
    project: &ProjectPackage,
) -> Result<(), ProjectIoError> {
    reject_traversal(package_dir.as_ref())?;
    let package_dir = package_dir.as_ref();
    ensure_supported_version(project.format_version)?;
    fs::create_dir_all(package_dir).map_err(|source| ProjectIoError::Io {
        path: package_dir.to_path_buf(),
        source,
    })?;
    for child in [
        "show-definitions",
        "run-sessions",
        "extensions/metadata",
        "assets/images",
        "assets/videos",
        "fonts",
        "thumbnails",
        "renders/cue-thumbnails",
        "renders/golden",
        "autosave",
    ] {
        let path = package_dir.join(child);
        fs::create_dir_all(&path).map_err(|source| ProjectIoError::Io { path, source })?;
    }

    let manifest = PackageManifest {
        format: "cuecanvas".to_string(),
        format_version: project.format_version,
        created_by: "CueCanvas".to_string(),
        minimum_runtime_version: "0.1.0".to_string(),
    };
    write_json_atomic(package_dir.join("manifest.json"), &manifest)?;

    let show_refs = project
        .show_definitions
        .iter()
        .map(|show| {
            let path = format!("show-definitions/{}.json", safe_file_stem(&show.id));
            write_json_atomic(package_dir.join(&path), show)?;
            Ok(PackageFileRef {
                id: show.id.clone(),
                path,
            })
        })
        .collect::<Result<Vec<_>, ProjectIoError>>()?;
    let run_refs = project
        .run_sessions
        .iter()
        .map(|run| {
            let path = format!("run-sessions/{}.json", safe_file_stem(&run.id));
            write_json_atomic(package_dir.join(&path), run)?;
            Ok(PackageFileRef {
                id: run.id.clone(),
                path,
            })
        })
        .collect::<Result<Vec<_>, ProjectIoError>>()?;

    let root = ProjectRootFile {
        format_version: project.format_version,
        migrations_applied: project.migrations_applied.clone(),
        metadata: project.metadata.clone(),
        show_definitions: show_refs,
        run_sessions: run_refs,
        settings: project.settings.clone(),
        extensions: Value::Object(project.extensions.clone().into_iter().collect()),
    };
    write_json_atomic(package_dir.join("project.json"), &root)?;
    write_json_atomic(
        package_dir.join("extensions/registry.json"),
        &project.extension_registry,
    )?;
    write_json_atomic(package_dir.join("autosave/latest.project.json"), &root)?;

    if let Some(show) = project.show_definitions.first() {
        write_json_atomic(
            package_dir.join("autosave/latest.show-definition.json"),
            show,
        )?;
    }
    if let Some(run) = project.run_sessions.first() {
        write_json_atomic(package_dir.join("autosave/latest.run-session.json"), run)?;
        if let Some(program) = &run.program_snapshot {
            write_json_atomic(
                package_dir.join("autosave/latest.program-snapshot.json"),
                program,
            )?;
        }
        if let Some(program) = &run.retained_program_snapshot {
            write_json_atomic(
                package_dir.join("autosave/retained.program-snapshot.json"),
                program,
            )?;
        }
        write_command_log(package_dir.join("autosave/command-log.jsonl"), run)?;
        write_json_atomic(
            package_dir.join("autosave/recovery-info.json"),
            &RecoveryInfo {
                project_id: project.metadata.id.clone(),
                package_format_version: project.format_version,
                latest_run_session_id: Some(run.id.clone()),
                latest_program_revision: run
                    .program_snapshot
                    .as_ref()
                    .map(|program| program.program_revision),
                retained_program_revision: run
                    .retained_program_snapshot
                    .as_ref()
                    .map(|program| program.program_revision),
            },
        )?;
    }

    Ok(())
}

pub fn load_project_package(
    package_dir: impl AsRef<Path>,
) -> Result<ProjectPackage, ProjectIoError> {
    reject_traversal(package_dir.as_ref())?;
    let manifest_path = package_dir.as_ref().join("manifest.json");
    if manifest_path.exists() {
        let manifest: PackageManifest = read_json(&manifest_path)?;
        ensure_supported_version(manifest.format_version)?;
    }

    let path = package_dir.as_ref().join("project.json");
    let root_value: Value = read_json(&path)?;
    let looks_like_legacy_project = root_value
        .get("showDefinitions")
        .and_then(Value::as_array)
        .and_then(|items| items.first())
        .is_some_and(|item| item.get("revision").is_some());
    if looks_like_legacy_project {
        let project: ProjectPackage =
            serde_json::from_value(root_value).map_err(|source| ProjectIoError::Json {
                path: path.clone(),
                source,
            })?;
        ensure_supported_version(project.format_version)?;
        return Ok(project);
    }

    let root: ProjectRootFile =
        serde_json::from_value(root_value).map_err(|source| ProjectIoError::Json {
            path: path.clone(),
            source,
        })?;
    ensure_supported_version(root.format_version)?;
    let show_definitions = root
        .show_definitions
        .iter()
        .map(|reference| read_package_json::<ShowDefinition>(package_dir.as_ref(), &reference.path))
        .collect::<Result<Vec<_>, ProjectIoError>>()?;
    let run_sessions = root
        .run_sessions
        .iter()
        .map(|reference| read_package_json::<RunSession>(package_dir.as_ref(), &reference.path))
        .collect::<Result<Vec<_>, ProjectIoError>>()?;
    let registry_path = package_dir.as_ref().join("extensions/registry.json");
    let extension_registry = if registry_path.exists() {
        read_json(&registry_path)?
    } else {
        ExtensionRegistry {
            installed_plugins: vec![],
            known_origins: vec![],
        }
    };

    Ok(ProjectPackage {
        format_version: root.format_version,
        migrations_applied: root.migrations_applied,
        metadata: root.metadata,
        show_definitions,
        run_sessions,
        extension_registry,
        settings: root.settings,
        extensions: match root.extensions {
            Value::Object(map) => map.into_iter().collect(),
            _ => Default::default(),
        },
    })
}

fn read_package_json<T: for<'de> Deserialize<'de>>(
    package_dir: &Path,
    relative_path: &str,
) -> Result<T, ProjectIoError> {
    let relative = PathBuf::from(relative_path);
    reject_traversal(&relative)?;
    read_json(&package_dir.join(relative))
}

fn read_json<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, ProjectIoError> {
    let bytes = fs::read(path).map_err(|source| ProjectIoError::Io {
        path: path.to_path_buf(),
        source,
    })?;
    serde_json::from_slice(&bytes).map_err(|source| ProjectIoError::Json {
        path: path.to_path_buf(),
        source,
    })
}

fn write_json_atomic<T: Serialize>(path: PathBuf, value: &T) -> Result<(), ProjectIoError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| ProjectIoError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    let tmp = temp_path_for(&path);
    let bytes = serde_json::to_vec_pretty(value).map_err(|source| ProjectIoError::Json {
        path: path.clone(),
        source,
    })?;
    {
        let mut file = fs::File::create(&tmp).map_err(|source| ProjectIoError::Io {
            path: tmp.clone(),
            source,
        })?;
        file.write_all(&bytes)
            .map_err(|source| ProjectIoError::Io {
                path: tmp.clone(),
                source,
            })?;
        file.sync_all().map_err(|source| ProjectIoError::Io {
            path: tmp.clone(),
            source,
        })?;
    }
    fs::rename(&tmp, &path).map_err(|source| ProjectIoError::Io { path, source })?;
    Ok(())
}

fn write_command_log(path: PathBuf, run: &RunSession) -> Result<(), ProjectIoError> {
    let mut bytes = Vec::new();
    for entry in &run.operation_log {
        serde_json::to_writer(&mut bytes, entry).map_err(|source| ProjectIoError::Json {
            path: path.clone(),
            source,
        })?;
        bytes.push(b'\n');
    }
    write_bytes_atomic(path, &bytes)
}

fn write_bytes_atomic(path: PathBuf, bytes: &[u8]) -> Result<(), ProjectIoError> {
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    fs::create_dir_all(parent).map_err(|source| ProjectIoError::Io {
        path: parent.to_path_buf(),
        source,
    })?;
    let tmp = temp_path_for(&path);
    {
        let mut file = fs::File::create(&tmp).map_err(|source| ProjectIoError::Io {
            path: tmp.clone(),
            source,
        })?;
        file.write_all(bytes).map_err(|source| ProjectIoError::Io {
            path: tmp.clone(),
            source,
        })?;
        file.sync_all().map_err(|source| ProjectIoError::Io {
            path: tmp.clone(),
            source,
        })?;
    }
    fs::rename(&tmp, &path).map_err(|source| ProjectIoError::Io { path, source })?;
    Ok(())
}

fn temp_path_for(path: &Path) -> PathBuf {
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("cuecanvas");
    path.with_file_name(format!("{file_name}.tmp"))
}

fn ensure_supported_version(format_version: u32) -> Result<(), ProjectIoError> {
    if format_version == SUPPORTED_FORMAT_VERSION {
        Ok(())
    } else {
        Err(ProjectIoError::UnsupportedFormat {
            found: format_version,
            supported: SUPPORTED_FORMAT_VERSION,
        })
    }
}

fn safe_file_stem(id: &str) -> String {
    let stem = id
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() || matches!(character, '-' | '_') {
                character
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_string();
    if stem.is_empty() {
        "item".to_string()
    } else {
        stem
    }
}

fn reject_traversal(path: &Path) -> Result<(), ProjectIoError> {
    if path
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return Err(ProjectIoError::InvalidPath {
            path: path.to_path_buf(),
            message: "path traversal is not allowed".to_string(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use cuecanvas_engine::demo_project;
    use cuecanvas_model::{Actor, OperationLogEntry};
    use serde_json::json;

    use super::*;

    #[test]
    fn package_save_load_preserves_unknown_extension_metadata() {
        let mut project = demo_project();
        project.extensions.insert(
            "com.example.roundtrip".to_string(),
            json!({ "future": { "value": 42 } }),
        );
        let dir = std::env::temp_dir().join(format!(
            "cuecanvas-test-{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        save_project_package(&dir, &project).unwrap();
        let loaded = load_project_package(&dir).unwrap();

        assert_eq!(
            loaded.extensions["com.example.roundtrip"]["future"]["value"],
            json!(42)
        );
        assert!(dir.join("manifest.json").exists());
        assert!(dir.join("extensions/registry.json").exists());
        assert!(dir.join("show-definitions/show-main.json").exists());
        assert!(dir.join("run-sessions/run-main-rehearsal.json").exists());
        assert!(dir.join("autosave/latest.project.json").exists());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn package_save_writes_autosave_recovery_and_command_log() {
        let mut project = demo_project();
        project.run_sessions[0]
            .operation_log
            .push(OperationLogEntry {
                id: "op-test".to_string(),
                at: "now".to_string(),
                actor: Actor::local_user(),
                action: "test".to_string(),
                message: "Test entry".to_string(),
                preview_revision: None,
                program_revision: None,
                idempotency_key: Some("idem".to_string()),
            });
        let dir = std::env::temp_dir().join(format!(
            "cuecanvas-autosave-test-{}.cuecanvas",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ));

        save_project_package(&dir, &project).unwrap();

        assert!(dir.join("autosave/latest.show-definition.json").exists());
        assert!(dir.join("autosave/latest.run-session.json").exists());
        assert!(dir.join("autosave/recovery-info.json").exists());
        assert!(
            fs::read_to_string(dir.join("autosave/command-log.jsonl"))
                .unwrap()
                .contains("op-test")
        );
        assert!(
            !fs::read_to_string(dir.join("project.json"))
                .unwrap()
                .contains("overlay-token")
        );

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn package_rejects_traversal_paths() {
        let project = demo_project();
        let err = save_project_package("../bad.cuecanvas", &project).unwrap_err();

        assert!(matches!(err, ProjectIoError::InvalidPath { .. }));
    }
}
