use cuecanvas_model::Stage;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSourceDesiredState {
    pub scene_name: String,
    pub source_name: String,
    pub url: String,
    pub stage: Stage,
    pub shutdown_when_not_visible: bool,
    pub refresh_when_active: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSourceObservedState {
    pub scene_exists: bool,
    pub source_exists: bool,
    pub url: String,
    pub width: u32,
    pub height: u32,
    pub shutdown_when_not_visible: bool,
    pub refresh_when_active: bool,
    pub visible_in_scene_path: bool,
    pub overlay_connected: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ObsHealthSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsHealthIssue {
    pub rule_id: &'static str,
    pub severity: ObsHealthSeverity,
    pub message: String,
}

pub fn verify_browser_source(
    desired: &BrowserSourceDesiredState,
    observed: &BrowserSourceObservedState,
) -> Vec<ObsHealthIssue> {
    let mut issues = vec![];

    if !observed.scene_exists {
        issues.push(error(
            "obs.sceneMissing",
            "CueCanvas Graphics scene is missing",
        ));
    }
    if !observed.source_exists {
        issues.push(error(
            "obs.browserSourceMissing",
            "CueCanvas Program Browser Source is missing",
        ));
    }
    if observed.url != desired.url {
        issues.push(error(
            "obs.browserSourceUrlMismatch",
            format!(
                "Browser Source URL differs from Runtime URL: expected {}",
                desired.url
            ),
        ));
    }
    if observed.width != desired.stage.width || observed.height != desired.stage.height {
        issues.push(error(
            "obs.browserSourceSizeMismatch",
            format!(
                "Browser Source size is {}x{}, expected {}x{}",
                observed.width, observed.height, desired.stage.width, desired.stage.height
            ),
        ));
    }
    if observed.shutdown_when_not_visible {
        issues.push(warning(
            "obs.shutdownWhenNotVisible",
            "Shutdown source when not visible may interrupt overlay recovery",
        ));
    }
    if observed.refresh_when_active {
        issues.push(warning(
            "obs.refreshWhenActive",
            "Refresh browser source when scene becomes active may reload Program",
        ));
    }
    if !observed.visible_in_scene_path {
        issues.push(warning(
            "obs.notVisibleInScenePath",
            "CueCanvas source is not visible in the intended scene path",
        ));
    }
    if !observed.overlay_connected {
        issues.push(error(
            "overlay.disconnected",
            "Program Overlay WebSocket is not connected",
        ));
    }

    issues
}

fn error(rule_id: &'static str, message: impl Into<String>) -> ObsHealthIssue {
    ObsHealthIssue {
        rule_id,
        severity: ObsHealthSeverity::Error,
        message: message.into(),
    }
}

fn warning(rule_id: &'static str, message: impl Into<String>) -> ObsHealthIssue {
    ObsHealthIssue {
        rule_id,
        severity: ObsHealthSeverity::Warning,
        message: message.into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_check_detects_size_url_and_recovery_risks() {
        let desired = BrowserSourceDesiredState {
            scene_name: "CueCanvas Graphics".to_string(),
            source_name: "CueCanvas Program".to_string(),
            url: "http://127.0.0.1:4317/overlay/program".to_string(),
            stage: Stage {
                width: 1920,
                height: 1080,
            },
            shutdown_when_not_visible: false,
            refresh_when_active: false,
        };
        let observed = BrowserSourceObservedState {
            scene_exists: true,
            source_exists: true,
            url: "http://127.0.0.1:9999/overlay/program".to_string(),
            width: 1280,
            height: 720,
            shutdown_when_not_visible: true,
            refresh_when_active: true,
            visible_in_scene_path: false,
            overlay_connected: false,
        };

        let issues = verify_browser_source(&desired, &observed);

        assert!(issues.iter().any(|issue| {
            issue.rule_id == "obs.browserSourceUrlMismatch"
                && issue.severity == ObsHealthSeverity::Error
        }));
        assert!(issues.iter().any(|issue| {
            issue.rule_id == "obs.refreshWhenActive" && issue.severity == ObsHealthSeverity::Warning
        }));
        assert!(
            issues
                .iter()
                .any(|issue| issue.rule_id == "overlay.disconnected")
        );
    }
}
