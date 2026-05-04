use std::{fs, path::PathBuf};

use anyhow::{Context, bail};
use cuecanvas_engine::{CueEngine, demo_project};
use cuecanvas_model::{Actor, OverlayState, ProjectPackage};
use cuecanvas_project::save_project_package;
use cuecanvas_protocol::{RuntimeCommandEnvelope, RuntimeCommandResponse};

fn main() -> anyhow::Result<()> {
    let mut args = std::env::args().skip(1);
    match args.next().as_deref() {
        Some("demo-json") => {
            println!("{}", serde_json::to_string_pretty(&demo_project())?);
        }
        Some("schema") => {
            let schema = schemars::schema_for!(ProjectPackage);
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }
        Some("overlay-schema") => {
            let schema = schemars::schema_for!(OverlayState);
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }
        Some("runtime-command-schema") => {
            let schema = schemars::schema_for!(RuntimeCommandEnvelope);
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }
        Some("runtime-command-response-schema") => {
            let schema = schemars::schema_for!(RuntimeCommandResponse);
            println!("{}", serde_json::to_string_pretty(&schema)?);
        }
        Some("write-fixtures") => {
            let root = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("fixtures"));
            write_fixtures(root)?;
        }
        Some("demo-package") => {
            let path = args
                .next()
                .map(PathBuf::from)
                .unwrap_or_else(|| PathBuf::from("CueCanvas Demo.cuecanvas"));
            save_project_package(&path, &demo_project())
                .with_context(|| format!("failed to write {}", path.display()))?;
            println!("wrote {}", path.display());
        }
        Some("demo-take") => {
            let mut project = demo_project();
            let show = project.show_definitions[0].clone();
            let run = &mut project.run_sessions[0];
            run.revisions.preview_revision = 1;
            let preview = CueEngine::preview_cue(&show, run, "cue-001", 1, "2026-05-04T00:00:00Z")?;
            let program = CueEngine::take_preview(
                run,
                &preview,
                "output-program-obs",
                0,
                "cli-demo-take",
                Actor::local_user(),
                "2026-05-04T00:00:01Z",
            )?;
            println!("{}", serde_json::to_string_pretty(&program)?);
        }
        _ => {
            bail!(
                "usage: cuecanvas <demo-json|schema|overlay-schema|runtime-command-schema|runtime-command-response-schema|write-fixtures [root]|demo-package [path]|demo-take>"
            );
        }
    }
    Ok(())
}

fn write_fixtures(root: PathBuf) -> anyhow::Result<()> {
    let project = demo_project();
    let show = project.show_definitions[0].clone();
    let run = project.run_sessions[0].clone();
    let preview = CueEngine::preview_cue(&show, &run, "cue-001", 1, "2026-05-04T00:00:00Z")?;

    let projects_dir = root.join("projects");
    let overlays_dir = root.join("overlay-states");
    fs::create_dir_all(&projects_dir)?;
    fs::create_dir_all(&overlays_dir)?;

    fs::write(
        projects_dir.join("demo.project.json"),
        serde_json::to_vec_pretty(&project)?,
    )?;
    fs::write(
        overlays_dir.join("demo.preview.overlay.json"),
        serde_json::to_vec_pretty(&preview.overlay_state)?,
    )?;
    println!("wrote fixtures under {}", root.display());
    Ok(())
}
