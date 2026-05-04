use std::{
    collections::BTreeMap,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    path::{Component, Path, PathBuf},
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Json, Router,
    extract::{
        Path as AxumPath, Query, State,
        ws::{Message, WebSocketUpgrade},
    },
    http::{HeaderMap, Response, StatusCode},
    response::{Html, IntoResponse},
    routing::{get, post},
};
mod commands;

use cuecanvas_data::{DataImportReport, DataImportRequest};
use cuecanvas_engine::demo_project;
use cuecanvas_model::{
    Actor, AssetKind, ConnectionState, HealthLevel, HealthStatus, PreviewSnapshot, ProgramSnapshot,
    ProjectPackage,
};
use cuecanvas_project::{ProjectIoError, load_project_package, save_project_package};
use cuecanvas_protocol::{
    CommandEnvelope, EmergencyLivePayload, HealthResponse, PreflightState, PreviewCuePayload,
    ProjectPersistenceRequest, ProjectPersistenceResponse, RuntimeCommandEnvelope,
    RuntimeCommandResponse, RuntimeEvent, TakePayload,
};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use tokio::sync::{Mutex, broadcast};
use uuid::Uuid;

pub type SharedRuntimeState = Arc<Mutex<RuntimeState>>;

#[derive(Debug)]
pub struct RuntimeState {
    pub project: ProjectPackage,
    pub package_dir: Option<PathBuf>,
    pub tokens: RuntimeTokens,
    pub event_tx: broadcast::Sender<RuntimeEvent>,
    pub command_results: BTreeMap<String, RuntimeCommandResponse>,
}

impl Default for RuntimeState {
    fn default() -> Self {
        let (event_tx, _) = broadcast::channel(128);
        Self {
            project: demo_project(),
            package_dir: None,
            tokens: RuntimeTokens::generate(),
            event_tx,
            command_results: BTreeMap::new(),
        }
    }
}

impl RuntimeState {
    pub fn new(project: ProjectPackage, tokens: RuntimeTokens) -> Self {
        let (event_tx, _) = broadcast::channel(128);
        Self {
            project,
            package_dir: None,
            tokens,
            event_tx,
            command_results: BTreeMap::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RuntimeTokens {
    pub editor_token: String,
    pub overlay_token: String,
}

impl RuntimeTokens {
    pub fn generate() -> Self {
        Self {
            editor_token: format!("editor-{}", Uuid::new_v4()),
            overlay_token: format!("overlay-{}", Uuid::new_v4()),
        }
    }

    #[cfg(test)]
    fn for_tests() -> Self {
        Self {
            editor_token: "test-editor-token".to_string(),
            overlay_token: "test-overlay-token".to_string(),
        }
    }
}

pub fn app(state: SharedRuntimeState) -> Router {
    Router::new()
        .route("/health", get(health))
        .route("/api/project", get(project))
        .route("/api/preflight", get(preflight))
        .route("/api/runtime/health", get(runtime_health))
        .route("/api/recovery/candidates", get(recovery_candidates))
        .route("/api/project/save", post(save_project))
        .route("/api/project/open", post(open_project))
        .route("/api/commands", post(command))
        .route("/api/data/import", post(import_data))
        .route("/api/live/preview/:cue_id", post(preview_cue))
        .route("/api/live/take", post(take))
        .route("/api/live/clear", post(clear))
        .route("/api/live/blackout", post(blackout))
        .route("/api/live/restore", post(restore))
        .route("/api/overlay/program/snapshot", get(program_snapshot))
        .route("/assets/:asset_id", get(asset_file))
        .route("/fonts/:font_id", get(font_file))
        .route("/editor", get(editor_index))
        .route("/editor/", get(editor_index))
        .route("/editor/*path", get(editor_asset))
        .route("/overlay/program", get(overlay_program_html))
        .route("/ws/editor", get(editor_ws))
        .route("/ws/overlay/program", get(overlay_program_ws))
        .with_state(state)
}

pub fn default_state() -> SharedRuntimeState {
    Arc::new(Mutex::new(RuntimeState::default()))
}

pub fn state_with_tokens(tokens: RuntimeTokens) -> SharedRuntimeState {
    Arc::new(Mutex::new(RuntimeState::new(demo_project(), tokens)))
}

pub fn default_addr() -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), 4317)
}

pub fn addr_for_port(port: u16) -> SocketAddr {
    SocketAddr::new(IpAddr::V4(Ipv4Addr::LOCALHOST), port)
}

async fn health() -> Json<HealthResponse> {
    Json(HealthResponse {
        ok: true,
        service: "cuecanvas-runtime".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
    })
}

#[derive(Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase")]
struct AuthQuery {
    token: Option<String>,
    editor_token: Option<String>,
    overlay_token: Option<String>,
}

async fn project(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
) -> Result<Json<ProjectPackage>, (StatusCode, Json<Value>)> {
    let state = state.lock().await;
    require_editor(&headers, &auth, &state.tokens)?;
    Ok(Json(state.project.clone()))
}

async fn preflight(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
) -> Result<Json<PreflightState>, (StatusCode, Json<Value>)> {
    let state = state.lock().await;
    require_editor(&headers, &auth, &state.tokens)?;
    commands::preflight_state_for_project(&state.project)
        .map(Json)
        .map_err(command_error)
}

async fn runtime_health(State(state): State<SharedRuntimeState>) -> Json<Value> {
    let state = state.lock().await;
    let run = state.project.run_sessions.first();
    Json(json!({
        "ok": true,
        "service": "cuecanvas-runtime",
        "version": env!("CARGO_PKG_VERSION"),
        "packageDir": state.package_dir.as_ref().map(|path| path.to_string_lossy().to_string()),
        "liveState": run.map(|run| &run.live_state),
        "outputConnectionState": run.map(|run| &run.output_connection_state),
        "obsHealth": run.map(|run| &run.obs_health),
        "overlayHealth": run.map(|run| &run.overlay_health),
    }))
}

async fn recovery_candidates(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
) -> Result<Json<Value>, (StatusCode, Json<Value>)> {
    let state = state.lock().await;
    require_editor(&headers, &auth, &state.tokens)?;
    let Some(package_dir) = &state.package_dir else {
        return Ok(Json(json!({ "candidates": [] })));
    };
    let autosave = package_dir.join("autosave");
    let candidates = [
        "latest.project.json",
        "latest.show-definition.json",
        "latest.run-session.json",
        "latest.program-snapshot.json",
        "retained.program-snapshot.json",
        "recovery-info.json",
    ]
    .into_iter()
    .filter_map(|name| {
        let path = autosave.join(name);
        path.exists().then(|| {
            json!({
                "kind": name.trim_end_matches(".json"),
                "path": path.to_string_lossy(),
            })
        })
    })
    .collect::<Vec<_>>();
    Ok(Json(json!({ "candidates": candidates })))
}

async fn save_project(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    Json(request): Json<ProjectPersistenceRequest>,
) -> Result<Json<ProjectPersistenceResponse>, (StatusCode, Json<Value>)> {
    let package_dir = package_path(&request.package_dir)?;
    let project = {
        let state = state.lock().await;
        require_editor(&headers, &auth, &state.tokens)?;
        state.project.clone()
    };

    save_project_package(&package_dir, &project).map_err(project_io_error)?;

    {
        let mut state = state.lock().await;
        state.package_dir = Some(package_dir.clone());
    }

    Ok(Json(ProjectPersistenceResponse {
        package_dir: package_dir.to_string_lossy().to_string(),
        project,
    }))
}

async fn open_project(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    Json(request): Json<ProjectPersistenceRequest>,
) -> Result<Json<ProjectPersistenceResponse>, (StatusCode, Json<Value>)> {
    let package_dir = package_path(&request.package_dir)?;
    {
        let state = state.lock().await;
        require_editor(&headers, &auth, &state.tokens)?;
    }

    let project = load_project_package(&package_dir).map_err(project_io_error)?;

    {
        let mut state = state.lock().await;
        state.project = project.clone();
        state.package_dir = Some(package_dir.clone());
    }

    Ok(Json(ProjectPersistenceResponse {
        package_dir: package_dir.to_string_lossy().to_string(),
        project,
    }))
}

async fn command(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    Json(envelope): Json<RuntimeCommandEnvelope>,
) -> Result<Json<RuntimeCommandResponse>, (StatusCode, Json<Value>)> {
    dispatch_editor_command(state, auth, headers, envelope).await
}

async fn import_data(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    Json(request): Json<DataImportRequest>,
) -> Result<Json<DataImportReport>, (StatusCode, Json<Value>)> {
    let payload = serde_json::to_value(request).map_err(|source| {
        api_error(
            StatusCode::BAD_REQUEST,
            format!("Cannot encode import request: {source}"),
        )
    })?;
    let response = dispatch_editor_command(
        state,
        auth,
        headers,
        CommandEnvelope {
            command: "data.import".to_string(),
            actor: Actor::local_user(),
            expected_revision: None,
            idempotency_key: None,
            payload,
        },
    )
    .await?;
    serde_json::from_value(response.0.result)
        .map(Json)
        .map_err(|source| api_error(StatusCode::INTERNAL_SERVER_ERROR, source.to_string()))
}

async fn dispatch_editor_command(
    state: SharedRuntimeState,
    auth: AuthQuery,
    headers: HeaderMap,
    envelope: RuntimeCommandEnvelope,
) -> Result<Json<RuntimeCommandResponse>, (StatusCode, Json<Value>)> {
    let mut state = state.lock().await;
    require_editor(&headers, &auth, &state.tokens)?;
    let outcome = commands::dispatch_command(&mut state, envelope).map_err(command_error)?;
    for event in outcome.events {
        let _ = state.event_tx.send(event);
    }
    Ok(Json(outcome.response))
}

async fn emergency_dispatch(
    state: SharedRuntimeState,
    auth: AuthQuery,
    headers: HeaderMap,
    command: &str,
    request: EmergencyRequest,
) -> Result<Json<ProgramSnapshot>, (StatusCode, Json<Value>)> {
    let response = dispatch_editor_command(
        state,
        auth,
        headers,
        CommandEnvelope {
            command: command.to_string(),
            actor: Actor::local_user(),
            expected_revision: None,
            idempotency_key: request.idempotency_key,
            payload: json!(EmergencyLivePayload {
                run_session_id: None,
                output_target_id: request.output_target_id,
            }),
        },
    )
    .await?;
    serde_json::from_value(response.0.result)
        .map(Json)
        .map_err(|source| api_error(StatusCode::INTERNAL_SERVER_ERROR, source.to_string()))
}

async fn preview_cue(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    AxumPath(cue_id): AxumPath<String>,
) -> Result<Json<PreviewSnapshot>, (StatusCode, Json<Value>)> {
    let response = dispatch_editor_command(
        state,
        auth,
        headers,
        CommandEnvelope {
            command: "live.previewCue".to_string(),
            actor: Actor::local_user(),
            expected_revision: None,
            idempotency_key: None,
            payload: json!(PreviewCuePayload {
                run_session_id: None,
                cue_id,
            }),
        },
    )
    .await?;
    serde_json::from_value(response.0.result)
        .map(Json)
        .map_err(|source| api_error(StatusCode::INTERNAL_SERVER_ERROR, source.to_string()))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct TakeRequest {
    output_target_id: String,
    preview_snapshot_id: String,
    preview_revision: u64,
    expected_program_revision: u64,
    idempotency_key: Option<String>,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EmergencyRequest {
    output_target_id: String,
    idempotency_key: Option<String>,
}

async fn take(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    Json(request): Json<TakeRequest>,
) -> Result<Json<ProgramSnapshot>, (StatusCode, Json<Value>)> {
    let response = dispatch_editor_command(
        state,
        auth,
        headers,
        CommandEnvelope {
            command: "live.take".to_string(),
            actor: Actor::local_user(),
            expected_revision: None,
            idempotency_key: request.idempotency_key,
            payload: json!(TakePayload {
                run_session_id: None,
                output_target_id: request.output_target_id,
                preview_snapshot_id: request.preview_snapshot_id,
                preview_revision: request.preview_revision,
                expected_program_revision: request.expected_program_revision,
            }),
        },
    )
    .await?;
    serde_json::from_value(response.0.result)
        .map(Json)
        .map_err(|source| api_error(StatusCode::INTERNAL_SERVER_ERROR, source.to_string()))
}

async fn clear(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    Json(request): Json<EmergencyRequest>,
) -> Result<Json<ProgramSnapshot>, (StatusCode, Json<Value>)> {
    emergency_dispatch(state, auth, headers, "live.clear", request).await
}

async fn blackout(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    Json(request): Json<EmergencyRequest>,
) -> Result<Json<ProgramSnapshot>, (StatusCode, Json<Value>)> {
    emergency_dispatch(state, auth, headers, "live.blackout", request).await
}

async fn restore(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    Json(request): Json<EmergencyRequest>,
) -> Result<Json<ProgramSnapshot>, (StatusCode, Json<Value>)> {
    emergency_dispatch(state, auth, headers, "live.restore", request).await
}

async fn program_snapshot(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
) -> Result<Json<Option<ProgramSnapshot>>, (StatusCode, Json<Value>)> {
    let state = state.lock().await;
    require_overlay(&headers, &auth, &state.tokens)?;
    let run = state
        .project
        .run_sessions
        .first()
        .ok_or_else(|| api_error(StatusCode::CONFLICT, "No active RunSession"))?;
    Ok(Json(run.program_snapshot.clone()))
}

async fn asset_file(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    AxumPath(asset_id): AxumPath<String>,
) -> Result<Response<axum::body::Body>, (StatusCode, Json<Value>)> {
    {
        let state = state.lock().await;
        require_overlay(&headers, &auth, &state.tokens)?;
    }
    serve_project_asset(state, &asset_id, &[AssetKind::Image, AssetKind::Video]).await
}

async fn font_file(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    AxumPath(font_id): AxumPath<String>,
) -> Result<Response<axum::body::Body>, (StatusCode, Json<Value>)> {
    {
        let state = state.lock().await;
        require_overlay(&headers, &auth, &state.tokens)?;
    }
    serve_project_asset(state, &font_id, &[AssetKind::Font]).await
}

async fn overlay_program_html(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
) -> Result<impl IntoResponse, (StatusCode, Json<Value>)> {
    let state = state.lock().await;
    require_overlay(&headers, &auth, &state.tokens)?;
    let token = auth
        .overlay_token
        .as_deref()
        .or(auth.token.as_deref())
        .ok_or_else(|| {
            api_error(
                StatusCode::BAD_REQUEST,
                "Overlay HTML requires a token query",
            )
        })?;
    let token_json = serde_json::to_string(token).unwrap_or_else(|_| "\"\"".to_string());

    let html = r#"<!doctype html>
<html lang="en">
  <head>
    <meta charset="utf-8" />
    <meta name="viewport" content="width=device-width, initial-scale=1" />
    <title>CueCanvas Program Overlay</title>
    <style>
      html, body, #overlay { margin: 0; width: 100%; height: 100%; background: transparent; overflow: hidden; }
      body { font-family: Inter, system-ui, sans-serif; }
      .item { position: absolute; box-sizing: border-box; }
      .text { text-shadow: 0 6px 20px rgba(0, 0, 0, .45); }
      .image { object-fit: cover; border-radius: 10px; background: rgba(15, 23, 42, .65); }
      .blackout { position: fixed; inset: 0; background: #000; }
    </style>
  </head>
  <body>
    <div id="overlay"></div>
    <script>
      const token = __CUECANVAS_OVERLAY_TOKEN__;
      const query = token ? `?token=${encodeURIComponent(token)}` : '';
      let ws = null;
      async function loadSnapshot() {
        const res = await fetch('/api/overlay/program/snapshot' + query);
        const snapshot = await res.json();
        render(snapshot && snapshot.overlayState ? snapshot.overlayState : { items: [] });
      }
      function ackRendered(state) {
        if (ws && ws.readyState === WebSocket.OPEN) {
          ws.send(JSON.stringify({ event: 'overlay.rendered', programRevision: state.programRevision }));
        }
      }
      function render(state) {
        const root = document.getElementById('overlay');
        root.innerHTML = '';
        for (const item of state.items || []) {
          if (item.kind === 'blackout') {
            root.innerHTML = '<div class="blackout"></div>';
            ackRendered(state);
            return;
          }
          if (item.kind === 'clear') continue;
          const frame = item.frame;
          const el = document.createElement(item.kind === 'image' ? 'img' : 'div');
          el.className = 'item ' + item.kind;
          el.style.left = frame.x + 'px';
          el.style.top = frame.y + 'px';
          el.style.width = frame.width + 'px';
          el.style.height = frame.height + 'px';
          el.style.zIndex = item.zIndex;
          if (item.kind === 'text') {
            el.textContent = item.text;
            el.style.fontSize = item.style.fontSize + 'px';
            el.style.color = item.style.color;
            el.style.fontWeight = item.style.weight === 'bold' ? '800' : '650';
          } else if (item.kind === 'image') {
            el.src = '/assets/' + encodeURIComponent(item.assetId) + query;
            el.alt = '';
            el.style.objectFit = item.fit;
          }
          root.appendChild(el);
        }
        ackRendered(state);
      }
      loadSnapshot();
      const protocol = location.protocol === 'https:' ? 'wss' : 'ws';
      ws = new WebSocket(`${protocol}://${location.host}/ws/overlay/program${query}`);
      ws.onmessage = event => {
        const snapshot = JSON.parse(event.data);
        if (snapshot && snapshot.overlayState) render(snapshot.overlayState);
      };
    </script>
  </body>
</html>"#
    .replace("__CUECANVAS_OVERLAY_TOKEN__", &token_json);

    Ok(Html(html).into_response())
}

async fn editor_index() -> Result<Response<axum::body::Body>, (StatusCode, Json<Value>)> {
    serve_web_asset("editor", "").await
}

async fn editor_asset(
    AxumPath(path): AxumPath<String>,
) -> Result<Response<axum::body::Body>, (StatusCode, Json<Value>)> {
    serve_web_asset("editor", &path).await
}

async fn editor_ws(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response<axum::body::Body>, (StatusCode, Json<Value>)> {
    let mut rx = {
        let state = state.lock().await;
        require_editor(&headers, &auth, &state.tokens)?;
        state.event_tx.subscribe()
    };

    Ok(ws.on_upgrade(move |mut socket| async move {
        loop {
            tokio::select! {
                message = socket.next() => {
                    match message {
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Ok(_)) => {}
                        Some(Err(_)) => break,
                    }
                }
                event = rx.recv() => {
                    match event {
                        Ok(event) => {
                            let Ok(text) = serde_json::to_string(&event) else {
                                continue;
                            };
                            if socket.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                        Err(broadcast::error::RecvError::Lagged(_)) => continue,
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }
    }))
}

async fn overlay_program_ws(
    State(state): State<SharedRuntimeState>,
    Query(auth): Query<AuthQuery>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response<axum::body::Body>, (StatusCode, Json<Value>)> {
    let mut rx = {
        let mut state = state.lock().await;
        require_overlay(&headers, &auth, &state.tokens)?;
        mark_overlay_connected(&mut state.project);
        let revision = state
            .project
            .run_sessions
            .first()
            .map(|run| run.revisions.runtime_revision)
            .unwrap_or_default();
        let _ = state.event_tx.send(RuntimeEvent::OverlayConnectionChanged {
            source: Actor::System,
            revision,
            payload: json!({ "state": "connected" }),
        });
        state.event_tx.subscribe()
    };

    Ok(ws.on_upgrade(move |mut socket| async move {
        let snapshot = {
            let state = state.lock().await;
            state
                .project
                .run_sessions
                .first()
                .and_then(|run| run.program_snapshot.clone())
        };
        if let Ok(text) = serde_json::to_string(&snapshot) {
            let _ = socket.send(Message::Text(text)).await;
        }

        loop {
            tokio::select! {
                message = socket.next() => {
                    match message {
                        Some(Ok(Message::Text(text))) => {
                            if let Ok(message) = serde_json::from_str::<OverlayClientMessage>(&text) {
                                if message.event.as_deref() == Some("overlay.rendered") {
                                    let mut state = state.lock().await;
                                    mark_overlay_rendered(&mut state.project, message.program_revision);
                                    let revision = state
                                        .project
                                        .run_sessions
                                        .first()
                                        .map(|run| run.revisions.runtime_revision)
                                        .unwrap_or_default();
                                    let _ = state.event_tx.send(RuntimeEvent::OverlayRendered {
                                        source: Actor::System,
                                        revision,
                                        payload: json!({ "programRevision": message.program_revision }),
                                    });
                                }
                            }
                        }
                        Some(Ok(Message::Close(_))) | None => break,
                        Some(Ok(_)) => {}
                        Some(Err(_)) => break,
                    }
                }
                event = rx.recv() => {
                    match event {
                        Ok(RuntimeEvent::ProgramChanged { payload, .. }) => {
                            let Ok(text) = serde_json::to_string(&payload) else {
                                continue;
                            };
                            if socket.send(Message::Text(text)).await.is_err() {
                                break;
                            }
                        }
                        Ok(_) => {}
                        Err(broadcast::error::RecvError::Lagged(_)) => {
                            let snapshot = {
                                let state = state.lock().await;
                                state
                                    .project
                                    .run_sessions
                                    .first()
                                    .and_then(|run| run.program_snapshot.clone())
                            };
                            if let Ok(text) = serde_json::to_string(&snapshot) {
                                let _ = socket.send(Message::Text(text)).await;
                            }
                        }
                        Err(broadcast::error::RecvError::Closed) => break,
                    }
                }
            }
        }

        let mut state = state.lock().await;
        mark_overlay_disconnected(&mut state.project);
        let revision = state
            .project
            .run_sessions
            .first()
            .map(|run| run.revisions.runtime_revision)
            .unwrap_or_default();
        let _ = state.event_tx.send(RuntimeEvent::OverlayConnectionChanged {
            source: Actor::System,
            revision,
            payload: json!({ "state": "disconnected" }),
        });
    }))
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct OverlayClientMessage {
    event: Option<String>,
    program_revision: Option<u64>,
}

fn mark_overlay_connected(project: &mut ProjectPackage) {
    if let Some(run) = project.run_sessions.first_mut() {
        run.output_connection_state = ConnectionState::Connected;
        run.overlay_health = HealthStatus {
            status: HealthLevel::Healthy,
            messages: vec!["Program Overlay WebSocket connected".to_string()],
        };
        run.revisions.runtime_revision += 1;
    }
}

fn mark_overlay_rendered(project: &mut ProjectPackage, program_revision: Option<u64>) {
    if let Some(run) = project.run_sessions.first_mut() {
        run.output_connection_state = ConnectionState::Connected;
        run.overlay_health = HealthStatus {
            status: HealthLevel::Healthy,
            messages: vec![match program_revision {
                Some(revision) => format!("Program Overlay rendered ProgramRevision {revision}"),
                None => "Program Overlay rendered latest snapshot".to_string(),
            }],
        };
        run.revisions.runtime_revision += 1;
    }
}

fn mark_overlay_disconnected(project: &mut ProjectPackage) {
    if let Some(run) = project.run_sessions.first_mut() {
        run.output_connection_state = ConnectionState::Disconnected;
        run.overlay_health = HealthStatus {
            status: HealthLevel::Warning,
            messages: vec!["Program Overlay WebSocket disconnected".to_string()],
        };
        run.revisions.runtime_revision += 1;
    }
}

async fn serve_project_asset(
    state: SharedRuntimeState,
    asset_id: &str,
    allowed_kinds: &[AssetKind],
) -> Result<Response<axum::body::Body>, (StatusCode, Json<Value>)> {
    let (package_dir, asset_path) = {
        let state = state.lock().await;
        let package_dir = state.package_dir.clone().ok_or_else(|| {
            api_error(
                StatusCode::CONFLICT,
                "Project package directory is not set; asset URLs require an opened or saved .cuecanvas package",
            )
        })?;
        let asset = state
            .project
            .show_definitions
            .iter()
            .flat_map(|show| show.assets.iter())
            .find(|asset| asset.id == asset_id && allowed_kinds.contains(&asset.kind))
            .ok_or_else(|| api_error(StatusCode::NOT_FOUND, "Asset not found"))?;
        (package_dir, asset.path.clone())
    };

    let root = package_dir.canonicalize().map_err(|source| {
        api_error(
            StatusCode::BAD_REQUEST,
            format!("Cannot canonicalize package directory: {source}"),
        )
    })?;
    let path = root.join(&asset_path).canonicalize().map_err(|source| {
        api_error(
            StatusCode::NOT_FOUND,
            format!("Cannot read asset path {asset_path}: {source}"),
        )
    })?;
    if !path.starts_with(&root) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Asset path escapes the .cuecanvas package",
        ));
    }

    let bytes = std::fs::read(&path).map_err(|source| {
        api_error(
            StatusCode::NOT_FOUND,
            format!("Cannot read asset file {}: {source}", path.display()),
        )
    })?;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", content_type_for_path(&path))
        .body(axum::body::Body::from(bytes))
        .map_err(|source| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Cannot build asset response: {source}"),
            )
        })
}

async fn serve_web_asset(
    app_name: &str,
    request_path: &str,
) -> Result<Response<axum::body::Body>, (StatusCode, Json<Value>)> {
    let root = web_dist_dir(app_name).ok_or_else(|| {
        api_error(
            StatusCode::NOT_FOUND,
            format!("Built {app_name} assets were not found"),
        )
    })?;
    let relative = safe_relative_path(request_path)?;
    let mut path = if relative.as_os_str().is_empty() {
        root.join("index.html")
    } else {
        root.join(&relative)
    };
    if !path.exists()
        && request_path
            .split('/')
            .last()
            .is_some_and(|name| !name.contains('.'))
    {
        path = root.join("index.html");
    }
    let canonical_root = root.canonicalize().map_err(|source| {
        api_error(
            StatusCode::BAD_REQUEST,
            format!("Cannot canonicalize web asset directory: {source}"),
        )
    })?;
    let canonical_path = path.canonicalize().map_err(|source| {
        api_error(
            StatusCode::NOT_FOUND,
            format!("Cannot read web asset: {source}"),
        )
    })?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(api_error(
            StatusCode::FORBIDDEN,
            "Web asset path escapes the bundled asset directory",
        ));
    }
    let bytes = std::fs::read(&canonical_path).map_err(|source| {
        api_error(
            StatusCode::NOT_FOUND,
            format!(
                "Cannot read web asset {}: {source}",
                canonical_path.display()
            ),
        )
    })?;
    Response::builder()
        .status(StatusCode::OK)
        .header("content-type", content_type_for_path(&canonical_path))
        .body(axum::body::Body::from(bytes))
        .map_err(|source| {
            api_error(
                StatusCode::INTERNAL_SERVER_ERROR,
                format!("Cannot build web asset response: {source}"),
            )
        })
}

fn web_dist_dir(app_name: &str) -> Option<PathBuf> {
    let env_name = format!("CUECANVAS_{}_DIST", app_name.to_ascii_uppercase());
    if let Some(path) = std::env::var_os(env_name).map(PathBuf::from) {
        if path.join("index.html").exists() {
            return Some(path);
        }
    }

    if let Ok(exe) = std::env::current_exe() {
        if let Some(contents_dir) = exe.parent().and_then(|path| path.parent()) {
            let bundled = contents_dir.join("Resources").join(app_name);
            if bundled.join("index.html").exists() {
                return Some(bundled);
            }
        }
    }

    let dev_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .ancestors()
        .nth(2)
        .map(Path::to_path_buf)?;
    let dev_dist = dev_root.join("apps").join(app_name).join("dist");
    dev_dist.join("index.html").exists().then_some(dev_dist)
}

fn safe_relative_path(value: &str) -> Result<PathBuf, (StatusCode, Json<Value>)> {
    let mut path = PathBuf::new();
    for component in Path::new(value).components() {
        match component {
            Component::Normal(part) => path.push(part),
            Component::CurDir => {}
            _ => {
                return Err(api_error(
                    StatusCode::FORBIDDEN,
                    "Path traversal is not allowed",
                ));
            }
        }
    }
    Ok(path)
}

fn content_type_for_path(path: &Path) -> &'static str {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("avif") => "image/avif",
        Some("gif") => "image/gif",
        Some("jpg" | "jpeg") => "image/jpeg",
        Some("png") => "image/png",
        Some("svg") => "image/svg+xml",
        Some("webp") => "image/webp",
        Some("mp4") => "video/mp4",
        Some("webm") => "video/webm",
        Some("css") => "text/css; charset=utf-8",
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("json") => "application/json",
        Some("map") => "application/json",
        Some("otf") => "font/otf",
        Some("ttf") => "font/ttf",
        Some("woff") => "font/woff",
        Some("woff2") => "font/woff2",
        _ => "application/octet-stream",
    }
}

fn package_path(value: &str) -> Result<PathBuf, (StatusCode, Json<Value>)> {
    let raw_path = PathBuf::from(value.trim());
    let path = safe_relative_or_absolute_path(&raw_path)?;
    if path.as_os_str().is_empty() {
        return Err(api_error(StatusCode::BAD_REQUEST, "packageDir is required"));
    }
    if !path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".cuecanvas"))
    {
        return Err(api_error(
            StatusCode::BAD_REQUEST,
            "packageDir must point to a .cuecanvas package directory",
        ));
    }
    let parent = path.parent().unwrap_or_else(|| Path::new("."));
    let canonical_parent = parent.canonicalize().map_err(|source| {
        api_error(
            StatusCode::BAD_REQUEST,
            format!("Cannot canonicalize package parent: {source}"),
        )
    })?;
    Ok(canonical_parent.join(path.file_name().unwrap_or_default()))
}

fn safe_relative_or_absolute_path(path: &Path) -> Result<PathBuf, (StatusCode, Json<Value>)> {
    let mut clean = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                clean.push(component.as_os_str())
            }
            Component::CurDir => {}
            Component::ParentDir => {
                return Err(api_error(
                    StatusCode::FORBIDDEN,
                    "Path traversal is not allowed in packageDir",
                ));
            }
        }
    }
    Ok(clean)
}

fn require_editor(
    headers: &HeaderMap,
    auth: &AuthQuery,
    tokens: &RuntimeTokens,
) -> Result<(), (StatusCode, Json<Value>)> {
    if bearer_matches(headers, &tokens.editor_token)
        || header_matches(headers, "x-cuecanvas-editor-token", &tokens.editor_token)
        || auth
            .editor_token
            .as_deref()
            .is_some_and(|token| secure_eq(token, &tokens.editor_token))
        || auth
            .token
            .as_deref()
            .is_some_and(|token| secure_eq(token, &tokens.editor_token))
    {
        Ok(())
    } else {
        Err(api_error(
            StatusCode::UNAUTHORIZED,
            "Missing or invalid editor token",
        ))
    }
}

fn require_overlay(
    headers: &HeaderMap,
    auth: &AuthQuery,
    tokens: &RuntimeTokens,
) -> Result<(), (StatusCode, Json<Value>)> {
    if bearer_matches(headers, &tokens.overlay_token)
        || header_matches(headers, "x-cuecanvas-overlay-token", &tokens.overlay_token)
        || auth
            .overlay_token
            .as_deref()
            .is_some_and(|token| secure_eq(token, &tokens.overlay_token))
        || auth
            .token
            .as_deref()
            .is_some_and(|token| secure_eq(token, &tokens.overlay_token))
    {
        Ok(())
    } else {
        Err(api_error(
            StatusCode::UNAUTHORIZED,
            "Missing or invalid overlay token",
        ))
    }
}

fn bearer_matches(headers: &HeaderMap, expected: &str) -> bool {
    headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.strip_prefix("Bearer "))
        .is_some_and(|token| secure_eq(token, expected))
}

fn header_matches(headers: &HeaderMap, header_name: &str, expected: &str) -> bool {
    headers
        .get(header_name)
        .and_then(|value| value.to_str().ok())
        .is_some_and(|token| secure_eq(token, expected))
}

fn secure_eq(candidate: &str, expected: &str) -> bool {
    if candidate.len() != expected.len() {
        return false;
    }
    candidate
        .bytes()
        .zip(expected.bytes())
        .fold(0_u8, |acc, (left, right)| acc | (left ^ right))
        == 0
}

fn project_io_error(error: ProjectIoError) -> (StatusCode, Json<Value>) {
    let status = match &error {
        ProjectIoError::Io { path, source }
            if source.kind() == std::io::ErrorKind::NotFound
                && path.ends_with(Path::new("project.json")) =>
        {
            StatusCode::NOT_FOUND
        }
        _ => StatusCode::BAD_REQUEST,
    };
    api_error(status, error.to_string())
}

fn command_error(error: commands::CommandError) -> (StatusCode, Json<Value>) {
    api_error(error.status, error.message)
}

fn api_error(status: StatusCode, message: impl Into<String>) -> (StatusCode, Json<Value>) {
    (status, Json(json!({ "error": message.into() })))
}

fn now_string() -> String {
    let seconds = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| duration.as_secs())
        .unwrap_or_default();
    format!("unix:{seconds}")
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use axum::{
        body::{Body, to_bytes},
        http::{Request, StatusCode},
    };
    use cuecanvas_model::{
        HealthLevel, HealthStatus, PreviewSnapshot, ProgramSnapshot, ProjectPackage,
    };
    use serde::de::DeserializeOwned;
    use serde_json::json;
    use tower::ServiceExt;

    use super::*;

    #[tokio::test]
    async fn runtime_preview_and_take_routes_commit_snapshot() {
        let tokens = RuntimeTokens::for_tests();
        let app = app(connected_state_with_tokens(tokens.clone()));

        let preview_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/live/preview/cue-001")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview_response.status(), StatusCode::OK);
        let preview: PreviewSnapshot = response_json(preview_response).await;

        let take_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/live/take")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "outputTargetId": "output-program-obs",
                            "previewSnapshotId": preview.id,
                            "previewRevision": preview.preview_revision,
                            "expectedProgramRevision": 0,
                            "idempotencyKey": "route-test-take"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(take_response.status(), StatusCode::OK);
        let program: ProgramSnapshot = response_json(take_response).await;

        assert_eq!(program.source_preview_snapshot_id, preview.id);
        assert_eq!(program.program_revision, 1);
        assert_eq!(
            program.resolved_data["instance-main-lower-third.primaryText"],
            json!("Cat")
        );
    }

    #[tokio::test]
    async fn runtime_rejects_missing_or_wrong_capability_tokens() {
        let tokens = RuntimeTokens::for_tests();
        let app = app(state_with_tokens(tokens.clone()));

        let no_token_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/project")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(no_token_response.status(), StatusCode::UNAUTHORIZED);

        let overlay_mutation_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/live/preview/cue-001")
                    .header("x-cuecanvas-overlay-token", &tokens.overlay_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(overlay_mutation_response.status(), StatusCode::UNAUTHORIZED);

        let overlay_read_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/overlay/program/snapshot")
                    .header("x-cuecanvas-overlay-token", &tokens.overlay_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(overlay_read_response.status(), StatusCode::OK);
    }

    #[tokio::test]
    async fn runtime_take_is_blocked_when_program_output_is_disconnected() {
        let tokens = RuntimeTokens::for_tests();
        let app = app(state_with_tokens(tokens.clone()));

        let preview_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/live/preview/cue-001")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview_response.status(), StatusCode::OK);
        let preview: PreviewSnapshot = response_json(preview_response).await;

        let take_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/live/take")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "outputTargetId": "output-program-obs",
                            "previewSnapshotId": preview.id,
                            "previewRevision": preview.preview_revision,
                            "expectedProgramRevision": 0,
                            "idempotencyKey": "route-test-take-disconnected"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(take_response.status(), StatusCode::CONFLICT);
        let body: Value = response_json(take_response).await;
        assert!(
            body["error"]
                .as_str()
                .unwrap()
                .contains("output.disconnected")
        );
    }

    #[tokio::test]
    async fn runtime_take_broadcasts_program_changed_event() {
        let tokens = RuntimeTokens::for_tests();
        let state = connected_state_with_tokens(tokens.clone());
        let mut rx = {
            let state = state.lock().await;
            state.event_tx.subscribe()
        };
        let app = app(state);

        let committed = commit_demo_program(app, &tokens).await;
        let mut saw_program = false;
        for _ in 0..8 {
            let event = rx.recv().await.unwrap();
            if matches!(
                event,
                RuntimeEvent::ProgramChanged { payload, .. } if payload.id == committed.id
            ) {
                saw_program = true;
                break;
            }
        }
        assert!(saw_program);
    }

    #[tokio::test]
    async fn command_route_rejects_stale_revision() {
        let tokens = RuntimeTokens::for_tests();
        let app = app(connected_state_with_tokens(tokens.clone()));

        let response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/commands")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "command": "show.updateStage",
                            "actor": { "kind": "user", "user_id": "local-user" },
                            "expectedRevision": 999,
                            "idempotencyKey": "stale-stage",
                            "payload": { "stage": { "width": 1280, "height": 720 } }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn command_route_blocks_edits_while_live_running() {
        let tokens = RuntimeTokens::for_tests();
        let app = app(connected_state_with_tokens(tokens.clone()));

        let transition_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/commands")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "command": "live.transition",
                            "actor": { "kind": "user", "user_id": "local-user" },
                            "expectedRevision": null,
                            "idempotencyKey": "go-live",
                            "payload": { "liveState": "liveRunning" }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(transition_response.status(), StatusCode::OK);

        let edit_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/commands")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "command": "show.updateStage",
                            "actor": { "kind": "user", "user_id": "local-user" },
                            "expectedRevision": null,
                            "idempotencyKey": "blocked-stage",
                            "payload": { "stage": { "width": 1280, "height": 720 } }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(edit_response.status(), StatusCode::CONFLICT);
    }

    #[tokio::test]
    async fn live_start_is_blocked_by_preflight_errors() {
        let tokens = RuntimeTokens::for_tests();
        let app = app(state_with_tokens(tokens.clone()));

        let transition_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/commands")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "command": "live.transition",
                            "actor": { "kind": "user", "user_id": "local-user" },
                            "expectedRevision": null,
                            "idempotencyKey": "blocked-live-start",
                            "payload": { "liveState": "liveRunning" }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();

        assert_eq!(transition_response.status(), StatusCode::CONFLICT);
        let body: Value = response_json(transition_response).await;
        assert!(body["error"].as_str().unwrap().contains("preflight errors"));
    }

    #[tokio::test]
    async fn preflight_route_marks_warning_overrides_by_value_hash() {
        let tokens = RuntimeTokens::for_tests();
        let state = connected_state_with_tokens(tokens.clone());
        {
            let mut state = state.lock().await;
            state.project.run_sessions[0].overlay_health = HealthStatus {
                status: HealthLevel::Warning,
                messages: vec!["first warning".to_string()],
            };
        }
        let app = app(state.clone());

        let preflight_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/preflight")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preflight_response.status(), StatusCode::OK);
        let preflight: PreflightState = response_json(preflight_response).await;
        let warning = preflight
            .groups
            .warnings
            .iter()
            .find(|item| item.rule_id == "output.healthWarning")
            .unwrap();
        let warning_key = warning.warning_key.clone().unwrap();
        let value_hash = warning.value_hash.clone().unwrap();

        let override_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/commands")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "command": "warningOverride.upsert",
                            "actor": { "kind": "user", "user_id": "local-user" },
                            "expectedRevision": null,
                            "idempotencyKey": "override-health-warning",
                            "payload": {
                                "warningKey": warning_key,
                                "valueHash": value_hash
                            }
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(override_response.status(), StatusCode::OK);

        let overridden_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/preflight")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let overridden: PreflightState = response_json(overridden_response).await;
        assert!(
            overridden
                .groups
                .warnings
                .iter()
                .any(|item| item.rule_id == "output.healthWarning" && item.overridden)
        );

        {
            let mut state = state.lock().await;
            state.project.run_sessions[0].overlay_health.messages =
                vec!["changed warning".to_string()];
        }
        let changed_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/preflight")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let changed: PreflightState = response_json(changed_response).await;
        assert!(
            changed
                .groups
                .warnings
                .iter()
                .any(|item| item.rule_id == "output.healthWarning" && !item.overridden)
        );
    }

    #[tokio::test]
    async fn asset_route_requires_overlay_token_and_serves_package_assets() {
        let tokens = RuntimeTokens::for_tests();
        let state = connected_state_with_tokens(tokens.clone());
        let package_dir = temp_package_dir("asset-route");
        let image_dir = package_dir.join("assets/images");
        fs::create_dir_all(&image_dir).unwrap();
        fs::write(image_dir.join("cat.png"), b"fake png").unwrap();
        {
            let mut state = state.lock().await;
            state.package_dir = Some(package_dir.clone());
        }
        let app = app(state);

        let unauthorized_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/assets/asset-cat-photo")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(unauthorized_response.status(), StatusCode::UNAUTHORIZED);

        let asset_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/assets/asset-cat-photo?token=test-overlay-token")
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(asset_response.status(), StatusCode::OK);
        assert_eq!(asset_response.headers()["content-type"], "image/png");
        assert_eq!(
            to_bytes(asset_response.into_body(), usize::MAX)
                .await
                .unwrap()
                .as_ref(),
            b"fake png"
        );

        let _ = fs::remove_dir_all(package_dir);
    }

    #[tokio::test]
    async fn runtime_save_and_open_recover_latest_program_snapshot() {
        let tokens = RuntimeTokens::for_tests();
        let source_app = app(connected_state_with_tokens(tokens.clone()));
        let committed = commit_demo_program(source_app.clone(), &tokens).await;
        let package_dir = temp_package_dir("runtime-recovery");

        let save_response = source_app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/project/save")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "packageDir": package_dir.to_string_lossy() }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(save_response.status(), StatusCode::OK);
        assert!(
            package_dir
                .join("autosave/latest.program-snapshot.json")
                .exists()
        );

        let recovery_app = app(Arc::new(Mutex::new(RuntimeState::new(
            demo_project(),
            tokens.clone(),
        ))));
        let open_response = recovery_app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/project/open")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({ "packageDir": package_dir.to_string_lossy() }).to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(open_response.status(), StatusCode::OK);

        let project_response = recovery_app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/project")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let recovered: ProjectPackage = response_json(project_response).await;
        let recovered_program = recovered.run_sessions[0]
            .program_snapshot
            .as_ref()
            .expect("saved package should recover the latest ProgramSnapshot");
        assert_eq!(recovered_program.id, committed.id);
        assert_eq!(recovered_program.program_revision, 1);

        let _ = fs::remove_dir_all(package_dir);
    }

    #[tokio::test]
    async fn runtime_data_import_requires_editor_token_and_preserves_program_snapshot() {
        let tokens = RuntimeTokens::for_tests();
        let app = app(connected_state_with_tokens(tokens.clone()));
        let committed = commit_demo_program(app.clone(), &tokens).await;

        let overlay_import_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/data/import")
                    .header("x-cuecanvas-overlay-token", &tokens.overlay_token)
                    .header("content-type", "application/json")
                    .body(Body::from(data_import_payload().to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(overlay_import_response.status(), StatusCode::UNAUTHORIZED);

        let import_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/data/import")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(data_import_payload().to_string()))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(import_response.status(), StatusCode::OK);
        let report: DataImportReport = response_json(import_response).await;
        assert_eq!(report.imported_rows, 1);

        let project_response = app
            .oneshot(
                Request::builder()
                    .method("GET")
                    .uri("/api/project")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        let project: ProjectPackage = response_json(project_response).await;
        let show = &project.show_definitions[0];
        assert_eq!(
            show.typed_entities.people[0].display_name,
            "Imported Speaker"
        );
        assert_eq!(
            show.data_tables
                .iter()
                .find(|table| table.entity_type == cuecanvas_model::EntityType::People)
                .unwrap()
                .rows[0]["displayName"],
            json!("Imported Speaker")
        );

        let program = project.run_sessions[0].program_snapshot.as_ref().unwrap();
        assert_eq!(program.id, committed.id);
        assert_eq!(
            program.resolved_data["instance-main-lower-third.primaryText"],
            json!("Cat")
        );
    }

    async fn commit_demo_program(app: Router, tokens: &RuntimeTokens) -> ProgramSnapshot {
        let preview_response = app
            .clone()
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/live/preview/cue-001")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .body(Body::empty())
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(preview_response.status(), StatusCode::OK);
        let preview: PreviewSnapshot = response_json(preview_response).await;

        let take_response = app
            .oneshot(
                Request::builder()
                    .method("POST")
                    .uri("/api/live/take")
                    .header("x-cuecanvas-editor-token", &tokens.editor_token)
                    .header("content-type", "application/json")
                    .body(Body::from(
                        json!({
                            "outputTargetId": "output-program-obs",
                            "previewSnapshotId": preview.id,
                            "previewRevision": preview.preview_revision,
                            "expectedProgramRevision": 0,
                            "idempotencyKey": "route-test-take"
                        })
                        .to_string(),
                    ))
                    .unwrap(),
            )
            .await
            .unwrap();
        assert_eq!(take_response.status(), StatusCode::OK);
        response_json(take_response).await
    }

    fn data_import_payload() -> Value {
        json!({
            "entityType": "people",
            "sourceName": "people.csv",
            "format": "csv",
            "content": "name,role\nImported Speaker,Host\n",
            "mappings": [
                { "source": "name", "target": "displayName" },
                { "source": "role", "target": "role" }
            ],
            "replaceExisting": true
        })
    }

    async fn response_json<T: DeserializeOwned>(response: Response<Body>) -> T {
        serde_json::from_slice(&to_bytes(response.into_body(), usize::MAX).await.unwrap()).unwrap()
    }

    fn temp_package_dir(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "cuecanvas-{label}-{}.cuecanvas",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .unwrap()
                .as_nanos()
        ))
    }

    fn connected_state_with_tokens(tokens: RuntimeTokens) -> SharedRuntimeState {
        let mut project = demo_project();
        mark_overlay_connected(&mut project);
        Arc::new(Mutex::new(RuntimeState::new(project, tokens)))
    }
}
