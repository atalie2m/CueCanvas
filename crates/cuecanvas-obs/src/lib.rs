use std::{collections::BTreeMap, fmt};

use cuecanvas_model::Stage;
use serde::{Deserialize, Serialize};
use serde_json::json;

pub const DEFAULT_SCENE_NAME: &str = "CueCanvas Graphics";
pub const DEFAULT_SOURCE_NAME: &str = "CueCanvas Program";
pub const BROWSER_SOURCE_KIND: &str = "browser_source";
pub const RECOMMENDED_CSS: &str =
    "html, body { margin: 0; background: transparent; overflow: hidden; }";

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsConnectionRequest {
    pub host: String,
    pub port: u16,
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub mock: bool,
}

impl Default for ObsConnectionRequest {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: 4455,
            password: None,
            mock: false,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsConnectionStatus {
    pub connected: bool,
    pub host: String,
    pub port: u16,
    pub obs_version: Option<String>,
    pub websocket_version: Option<String>,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BrowserSourceDesiredState {
    pub scene_name: String,
    pub source_name: String,
    pub url: String,
    pub stage: Stage,
    pub shutdown_when_not_visible: bool,
    pub refresh_when_active: bool,
    pub custom_css: String,
}

impl BrowserSourceDesiredState {
    pub fn new(url: String, stage: Stage) -> Self {
        Self {
            scene_name: DEFAULT_SCENE_NAME.to_string(),
            source_name: DEFAULT_SOURCE_NAME.to_string(),
            url,
            stage,
            shutdown_when_not_visible: false,
            refresh_when_active: false,
            custom_css: RECOMMENDED_CSS.to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsSetupRequest {
    pub connection: ObsConnectionRequest,
    pub desired: BrowserSourceDesiredState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsSetupResponse {
    pub connection: ObsConnectionStatus,
    pub desired: BrowserSourceDesiredState,
    pub observed: ObsObservedState,
    pub issues: Vec<ObsIssue>,
    pub actions: Vec<String>,
}

pub type ObsObservedState = BrowserSourceObservedState;
pub type ObsIssue = ObsHealthIssue;

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
    pub custom_css: String,
    pub visible_in_scene_path: bool,
    pub overlay_connected: bool,
}

impl BrowserSourceObservedState {
    pub fn missing() -> Self {
        Self {
            scene_exists: false,
            source_exists: false,
            url: String::new(),
            width: 0,
            height: 0,
            shutdown_when_not_visible: false,
            refresh_when_active: false,
            custom_css: String::new(),
            visible_in_scene_path: false,
            overlay_connected: false,
        }
    }

    pub fn from_desired(desired: &BrowserSourceDesiredState, overlay_connected: bool) -> Self {
        Self {
            scene_exists: true,
            source_exists: true,
            url: desired.url.clone(),
            width: desired.stage.width,
            height: desired.stage.height,
            shutdown_when_not_visible: desired.shutdown_when_not_visible,
            refresh_when_active: desired.refresh_when_active,
            custom_css: desired.custom_css.clone(),
            visible_in_scene_path: true,
            overlay_connected,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum ObsHealthSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ObsHealthIssue {
    pub rule_id: String,
    pub severity: ObsHealthSeverity,
    pub message: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObsError {
    message: String,
}

impl ObsError {
    pub fn new(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ObsError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl std::error::Error for ObsError {}

#[allow(async_fn_in_trait)]
pub trait ObsTransport {
    async fn connect(
        &mut self,
        request: &ObsConnectionRequest,
    ) -> Result<ObsConnectionStatus, ObsError>;
    async fn apply(
        &mut self,
        desired: &BrowserSourceDesiredState,
        overlay_connected: bool,
    ) -> Result<ObsSetupResponse, ObsError>;
    async fn verify(
        &mut self,
        desired: &BrowserSourceDesiredState,
        overlay_connected: bool,
    ) -> Result<ObsSetupResponse, ObsError>;
}

pub async fn connect_with_transport<T: ObsTransport>(
    transport: &mut T,
    request: &ObsConnectionRequest,
) -> Result<ObsConnectionStatus, ObsError> {
    transport.connect(request).await
}

pub async fn apply_with_transport<T: ObsTransport>(
    transport: &mut T,
    request: &ObsSetupRequest,
    overlay_connected: bool,
) -> Result<ObsSetupResponse, ObsError> {
    transport.connect(&request.connection).await?;
    transport.apply(&request.desired, overlay_connected).await
}

pub async fn verify_with_transport<T: ObsTransport>(
    transport: &mut T,
    request: &ObsSetupRequest,
    overlay_connected: bool,
) -> Result<ObsSetupResponse, ObsError> {
    transport.connect(&request.connection).await?;
    transport.verify(&request.desired, overlay_connected).await
}

pub async fn connect_real(request: &ObsConnectionRequest) -> Result<ObsConnectionStatus, ObsError> {
    let client = obws::Client::connect(
        request.host.as_str(),
        request.port,
        request.password.as_deref(),
    )
    .await
    .map_err(|error| ObsError::new(format!("OBS WebSocket connect failed: {error}")))?;
    let version = client
        .general()
        .version()
        .await
        .map_err(|error| ObsError::new(format!("OBS version check failed: {error}")))?;
    Ok(ObsConnectionStatus {
        connected: true,
        host: request.host.clone(),
        port: request.port,
        obs_version: Some(version.obs_studio_version.to_string()),
        websocket_version: Some(version.obs_web_socket_version.to_string()),
        message: "OBS WebSocket authenticated".to_string(),
    })
}

pub async fn apply_real(
    request: &ObsSetupRequest,
    overlay_connected: bool,
) -> Result<ObsSetupResponse, ObsError> {
    let client = obws::Client::connect(
        request.connection.host.as_str(),
        request.connection.port,
        request.connection.password.as_deref(),
    )
    .await
    .map_err(|error| ObsError::new(format!("OBS WebSocket connect failed: {error}")))?;
    apply_with_client(&client, request, overlay_connected).await
}

pub async fn verify_real(
    request: &ObsSetupRequest,
    overlay_connected: bool,
) -> Result<ObsSetupResponse, ObsError> {
    let client = obws::Client::connect(
        request.connection.host.as_str(),
        request.connection.port,
        request.connection.password.as_deref(),
    )
    .await
    .map_err(|error| ObsError::new(format!("OBS WebSocket connect failed: {error}")))?;
    verify_with_client(&client, request, overlay_connected).await
}

async fn apply_with_client(
    client: &obws::Client,
    request: &ObsSetupRequest,
    overlay_connected: bool,
) -> Result<ObsSetupResponse, ObsError> {
    let desired = &request.desired;
    let mut actions = Vec::new();
    let _ = client.scenes().create(&desired.scene_name).await.map(|_| {
        actions.push(format!("Created scene '{}'", desired.scene_name));
    });

    let settings = browser_source_settings(desired);
    let source_exists = client
        .inputs()
        .settings::<BrowserSourceSettings>(obws::requests::inputs::InputId::Name(
            &desired.source_name,
        ))
        .await
        .is_ok();

    if source_exists {
        client
            .inputs()
            .set_settings(obws::requests::inputs::SetSettings {
                input: obws::requests::inputs::InputId::Name(&desired.source_name),
                settings: &settings,
                overlay: Some(true),
            })
            .await
            .map_err(|error| ObsError::new(format!("OBS Browser Source update failed: {error}")))?;
        actions.push(format!("Updated Browser Source '{}'", desired.source_name));
    } else {
        client
            .inputs()
            .create(obws::requests::inputs::Create {
                scene: obws::requests::scenes::SceneId::Name(&desired.scene_name),
                input: &desired.source_name,
                kind: BROWSER_SOURCE_KIND,
                settings: Some(settings),
                enabled: Some(true),
            })
            .await
            .map_err(|error| ObsError::new(format!("OBS Browser Source create failed: {error}")))?;
        actions.push(format!("Created Browser Source '{}'", desired.source_name));
    }

    let mut response = verify_with_client(client, request, overlay_connected).await?;
    response.actions = actions;
    Ok(response)
}

async fn verify_with_client(
    client: &obws::Client,
    request: &ObsSetupRequest,
    overlay_connected: bool,
) -> Result<ObsSetupResponse, ObsError> {
    let desired = &request.desired;
    let settings = client
        .inputs()
        .settings::<BrowserSourceSettings>(obws::requests::inputs::InputId::Name(
            &desired.source_name,
        ))
        .await
        .map(|settings| settings.settings)
        .ok();
    let observed = settings
        .map(|settings| BrowserSourceObservedState {
            scene_exists: true,
            source_exists: true,
            url: settings.url.unwrap_or_default(),
            width: settings.width.unwrap_or_default(),
            height: settings.height.unwrap_or_default(),
            shutdown_when_not_visible: settings.shutdown.unwrap_or_default(),
            refresh_when_active: settings.restart_when_active.unwrap_or_default(),
            custom_css: settings.css.unwrap_or_default(),
            visible_in_scene_path: true,
            overlay_connected,
        })
        .unwrap_or_else(BrowserSourceObservedState::missing);
    let connection = connect_real(&request.connection).await?;
    Ok(setup_response(
        connection,
        desired.clone(),
        observed,
        vec![],
    ))
}

fn browser_source_settings(desired: &BrowserSourceDesiredState) -> BrowserSourceSettings {
    BrowserSourceSettings {
        url: Some(desired.url.clone()),
        width: Some(desired.stage.width),
        height: Some(desired.stage.height),
        shutdown: Some(desired.shutdown_when_not_visible),
        restart_when_active: Some(desired.refresh_when_active),
        css: Some(desired.custom_css.clone()),
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct BrowserSourceSettings {
    #[serde(default)]
    url: Option<String>,
    #[serde(default)]
    width: Option<u32>,
    #[serde(default)]
    height: Option<u32>,
    #[serde(default)]
    shutdown: Option<bool>,
    #[serde(default)]
    restart_when_active: Option<bool>,
    #[serde(default)]
    css: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MockObsTransport {
    pub connection: ObsConnectionStatus,
    pub observed: BTreeMap<(String, String), BrowserSourceObservedState>,
}

impl MockObsTransport {
    pub fn connected() -> Self {
        Self {
            connection: ObsConnectionStatus {
                connected: true,
                host: "127.0.0.1".to_string(),
                port: 4455,
                obs_version: Some("mock-obs".to_string()),
                websocket_version: Some("mock-websocket".to_string()),
                message: "Mock OBS connected".to_string(),
            },
            observed: BTreeMap::new(),
        }
    }
}

impl Default for MockObsTransport {
    fn default() -> Self {
        Self::connected()
    }
}

impl ObsTransport for MockObsTransport {
    async fn connect(
        &mut self,
        request: &ObsConnectionRequest,
    ) -> Result<ObsConnectionStatus, ObsError> {
        self.connection.host = request.host.clone();
        self.connection.port = request.port;
        Ok(self.connection.clone())
    }

    async fn apply(
        &mut self,
        desired: &BrowserSourceDesiredState,
        overlay_connected: bool,
    ) -> Result<ObsSetupResponse, ObsError> {
        let key = (desired.scene_name.clone(), desired.source_name.clone());
        let action = if self.observed.contains_key(&key) {
            format!("Updated Browser Source '{}'", desired.source_name)
        } else {
            format!("Created Browser Source '{}'", desired.source_name)
        };
        let observed = BrowserSourceObservedState::from_desired(desired, overlay_connected);
        self.observed.insert(key, observed.clone());
        Ok(setup_response(
            self.connection.clone(),
            desired.clone(),
            observed,
            vec![
                format!("Ensured scene '{}'", desired.scene_name),
                action,
                "Applied recommended custom CSS".to_string(),
            ],
        ))
    }

    async fn verify(
        &mut self,
        desired: &BrowserSourceDesiredState,
        overlay_connected: bool,
    ) -> Result<ObsSetupResponse, ObsError> {
        let key = (desired.scene_name.clone(), desired.source_name.clone());
        let mut observed = self
            .observed
            .get(&key)
            .cloned()
            .unwrap_or_else(BrowserSourceObservedState::missing);
        observed.overlay_connected = overlay_connected;
        Ok(setup_response(
            self.connection.clone(),
            desired.clone(),
            observed,
            vec!["Verified Browser Source settings".to_string()],
        ))
    }
}

pub fn setup_response(
    connection: ObsConnectionStatus,
    desired: BrowserSourceDesiredState,
    observed: BrowserSourceObservedState,
    actions: Vec<String>,
) -> ObsSetupResponse {
    let issues = verify_browser_source(&desired, &observed);
    ObsSetupResponse {
        connection,
        desired,
        observed,
        issues,
        actions,
    }
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
    if observed.custom_css.trim() != desired.custom_css.trim() {
        issues.push(warning(
            "obs.customCssMismatch",
            "Browser Source custom CSS differs from CueCanvas recommendation",
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

fn error(rule_id: impl Into<String>, message: impl Into<String>) -> ObsHealthIssue {
    ObsHealthIssue {
        rule_id: rule_id.into(),
        severity: ObsHealthSeverity::Error,
        message: message.into(),
    }
}

fn warning(rule_id: impl Into<String>, message: impl Into<String>) -> ObsHealthIssue {
    ObsHealthIssue {
        rule_id: rule_id.into(),
        severity: ObsHealthSeverity::Warning,
        message: message.into(),
    }
}

pub fn health_messages(issues: &[ObsHealthIssue]) -> (cuecanvas_model::HealthLevel, Vec<String>) {
    if issues
        .iter()
        .any(|issue| issue.severity == ObsHealthSeverity::Error)
    {
        (
            cuecanvas_model::HealthLevel::Error,
            issues.iter().map(|issue| issue.message.clone()).collect(),
        )
    } else if issues
        .iter()
        .any(|issue| issue.severity == ObsHealthSeverity::Warning)
    {
        (
            cuecanvas_model::HealthLevel::Warning,
            issues.iter().map(|issue| issue.message.clone()).collect(),
        )
    } else {
        (
            cuecanvas_model::HealthLevel::Healthy,
            vec!["OBS Browser Source verified".to_string()],
        )
    }
}

pub fn status_payload(response: &ObsSetupResponse) -> serde_json::Value {
    json!({
        "connection": response.connection,
        "desired": response.desired,
        "observed": response.observed,
        "issues": response.issues,
        "actions": response.actions,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn health_check_detects_size_url_css_and_recovery_risks() {
        let desired = BrowserSourceDesiredState::new(
            "http://127.0.0.1:4317/overlay/program".to_string(),
            Stage {
                width: 1920,
                height: 1080,
            },
        );
        let observed = BrowserSourceObservedState {
            scene_exists: true,
            source_exists: true,
            url: "http://127.0.0.1:9999/overlay/program".to_string(),
            width: 1280,
            height: 720,
            shutdown_when_not_visible: true,
            refresh_when_active: true,
            custom_css: "body { background: black; }".to_string(),
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
        assert!(issues.iter().any(|issue| {
            issue.rule_id == "obs.customCssMismatch" && issue.severity == ObsHealthSeverity::Warning
        }));
        assert!(
            issues
                .iter()
                .any(|issue| issue.rule_id == "overlay.disconnected")
        );
    }

    #[tokio::test]
    async fn mock_transport_creates_updates_and_verifies_browser_source() {
        let mut transport = MockObsTransport::connected();
        let desired = BrowserSourceDesiredState::new(
            "http://127.0.0.1:4317/overlay/program?token=overlay-test".to_string(),
            Stage {
                width: 1920,
                height: 1080,
            },
        );
        let request = ObsSetupRequest {
            connection: ObsConnectionRequest {
                mock: true,
                ..ObsConnectionRequest::default()
            },
            desired: desired.clone(),
        };

        let applied = apply_with_transport(&mut transport, &request, true)
            .await
            .unwrap();
        assert_eq!(applied.observed.url, desired.url);
        assert!(applied.issues.is_empty());
        assert!(
            applied
                .actions
                .iter()
                .any(|action| action.contains("Created Browser Source"))
        );

        let updated = apply_with_transport(&mut transport, &request, true)
            .await
            .unwrap();
        assert!(
            updated
                .actions
                .iter()
                .any(|action| action.contains("Updated Browser Source"))
        );

        let verified = verify_with_transport(&mut transport, &request, true)
            .await
            .unwrap();
        assert_eq!(verified.issues, vec![]);
    }
}
