use std::collections::BTreeMap;

use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

pub type Extensions = BTreeMap<String, Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPackage {
    pub format_version: u32,
    pub migrations_applied: Vec<String>,
    pub metadata: ProjectMetadata,
    pub show_definitions: Vec<ShowDefinition>,
    pub run_sessions: Vec<RunSession>,
    pub extension_registry: ExtensionRegistry,
    #[serde(default)]
    pub settings: Value,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectMetadata {
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ExtensionRegistry {
    #[serde(default)]
    pub installed_plugins: Vec<PluginIdentity>,
    #[serde(default)]
    pub known_origins: Vec<KnownOrigin>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PluginIdentity {
    pub plugin_id: String,
    pub plugin_version: String,
    pub publisher: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct KnownOrigin {
    pub origin: Origin,
    pub status: OriginStatus,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OriginStatus {
    Available,
    Missing,
    Disabled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Origin {
    User {
        user_id: Option<String>,
    },
    System,
    Plugin {
        plugin_id: String,
        plugin_version: String,
    },
    Import {
        source: String,
    },
    Migration {
        migration_id: String,
    },
}

impl Origin {
    pub fn system() -> Self {
        Self::System
    }

    pub fn local_user() -> Self {
        Self::User {
            user_id: Some("local-user".to_string()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Actor {
    User {
        user_id: String,
    },
    System,
    Plugin {
        plugin_id: String,
        plugin_version: String,
    },
    ExternalInput {
        input_key: String,
    },
    Migration {
        migration_id: String,
    },
    Recovery,
}

impl Actor {
    pub fn local_user() -> Self {
        Self::User {
            user_id: "local-user".to_string(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ShowDefinition {
    pub id: String,
    pub revision: u64,
    pub name: String,
    pub stage: Stage,
    pub output_targets: Vec<OutputTarget>,
    pub assets: Vec<Asset>,
    pub template_definitions: Vec<TemplateDefinition>,
    pub template_instances: Vec<TemplateInstance>,
    pub typed_entities: TypedEntities,
    pub data_tables: Vec<DataTable>,
    pub cue_sheet: CueSheet,
    #[serde(default)]
    pub show_settings: Value,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Stage {
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OutputTarget {
    pub id: String,
    pub name: String,
    pub kind: OutputTargetKind,
    pub stage: Stage,
    pub overlay_url: String,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum OutputTargetKind {
    ObsBrowserSource,
    EmbeddedPreview,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    pub id: String,
    pub kind: AssetKind,
    pub path: String,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum AssetKind {
    Image,
    Video,
    Font,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplateDefinition {
    pub id: String,
    pub name: String,
    pub kind: TemplateKind,
    pub version: u32,
    pub local_slots: Vec<LocalSlot>,
    pub variants: Vec<TemplateVariant>,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TemplateKind {
    LowerThird,
    SessionTitle,
    SpeakerCard,
    SponsorBug,
    BreakScreen,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LocalSlot {
    pub key: String,
    pub kind: SlotKind,
    pub label: String,
    pub required: bool,
    pub max_length: Option<usize>,
    pub fallback: Option<Value>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum SlotKind {
    Text,
    AssetImage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplateVariant {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInstance {
    pub id: String,
    pub name: String,
    pub template_definition_id: String,
    pub template_version_policy: TemplateVersionPolicy,
    pub frame: Rect,
    pub z_index: i32,
    pub variant_id: String,
    #[serde(default)]
    pub slot_bindings: BTreeMap<String, SlotBinding>,
    pub visible_by_default: bool,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum TemplateVersionPolicy {
    LatestCompatible,
    Pinned { version: u32 },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Rect {
    pub x: i32,
    pub y: i32,
    pub width: u32,
    pub height: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct SlotBinding {
    pub entity_type: EntityType,
    pub field: String,
}

#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum EntityType {
    People,
    Sessions,
    Sponsors,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TypedEntities {
    #[serde(default)]
    pub people: Vec<Person>,
    #[serde(default)]
    pub sessions: Vec<Session>,
    #[serde(default)]
    pub sponsors: Vec<Sponsor>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Person {
    pub id: String,
    pub display_name: String,
    pub role: Option<String>,
    pub organization: Option<String>,
    pub photo: Option<String>,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Session {
    pub id: String,
    pub title: String,
    pub track: Option<String>,
    pub start_time: Option<String>,
    #[serde(default)]
    pub speaker_refs: Vec<String>,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Sponsor {
    pub id: String,
    pub name: String,
    pub logo: Option<String>,
    pub tier: Option<String>,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataTable {
    pub id: String,
    pub entity_type: EntityType,
    pub rows: Vec<BTreeMap<String, Value>>,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CueSheet {
    pub cues: Vec<Cue>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct Cue {
    pub id: String,
    pub number: String,
    pub segment: Option<String>,
    pub name: String,
    pub mode: CueMode,
    pub cue_type: TemplateKind,
    #[serde(default)]
    pub entity_refs: BTreeMap<String, EntityRef>,
    #[serde(default)]
    pub slot_overrides: BTreeMap<String, BTreeMap<String, Value>>,
    #[serde(default)]
    pub template_states: Vec<TemplateState>,
    pub transition: Transition,
    pub operator_notes: Option<String>,
    pub origin: Origin,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum CueMode {
    Absolute,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EntityRef {
    pub entity_type: EntityType,
    pub entity_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplateState {
    pub template_instance_id: String,
    pub visible: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum Transition {
    Cut,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunSession {
    pub id: String,
    pub show_definition_id: String,
    pub show_definition_revision: u64,
    pub live_state: LiveState,
    pub preview_snapshot: Option<PreviewSnapshot>,
    pub program_snapshot: Option<ProgramSnapshot>,
    pub retained_program_snapshot: Option<ProgramSnapshot>,
    pub output_connection_state: ConnectionState,
    pub obs_health: HealthStatus,
    pub overlay_health: HealthStatus,
    #[serde(default)]
    pub warning_overrides: BTreeMap<String, String>,
    #[serde(default)]
    pub operation_log: Vec<OperationLogEntry>,
    pub revisions: RunRevisions,
    #[serde(default)]
    pub live_command_results: BTreeMap<String, ProgramSnapshot>,
    #[serde(default)]
    pub run_settings: Value,
    #[serde(default)]
    pub extensions: Extensions,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum LiveState {
    Setup,
    Rehearsal,
    PreflightReady,
    LiveArmed,
    LiveRunning,
    OutputDegraded,
    Recovering,
    Ended,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ConnectionState {
    Disconnected,
    Connected,
    Degraded,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthStatus {
    pub status: HealthLevel,
    #[serde(default)]
    pub messages: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum HealthLevel {
    Unknown,
    Healthy,
    Warning,
    Error,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunRevisions {
    pub project_revision: u64,
    pub show_definition_revision: u64,
    pub data_revision: u64,
    pub preview_revision: u64,
    pub program_revision: u64,
    pub runtime_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OperationLogEntry {
    pub id: String,
    pub at: String,
    pub actor: Actor,
    pub action: String,
    pub message: String,
    pub preview_revision: Option<u64>,
    pub program_revision: Option<u64>,
    pub idempotency_key: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewSnapshot {
    pub id: String,
    pub run_session_id: String,
    pub show_definition_id: String,
    pub show_definition_revision: u64,
    pub source_cue_id: String,
    pub preview_revision: u64,
    pub created_at: String,
    pub origin_chain: Vec<Origin>,
    pub resolved_data: BTreeMap<String, Value>,
    pub overlay_state: OverlayState,
    pub preflight_result: PreflightResult,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProgramSnapshot {
    pub id: String,
    pub run_session_id: String,
    pub show_definition_id: String,
    pub show_definition_revision: u64,
    pub source_preview_snapshot_id: String,
    pub source_cue_id: String,
    pub program_revision: u64,
    pub created_at: String,
    pub resolved_data: BTreeMap<String, Value>,
    pub overlay_state: OverlayState,
    #[serde(default)]
    pub asset_manifest: Vec<String>,
    pub origin_chain: Vec<Origin>,
    pub output_target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreflightResult {
    pub items: Vec<PreflightItem>,
}

impl PreflightResult {
    pub fn passed() -> Self {
        Self { items: vec![] }
    }

    pub fn has_blocking_error(&self) -> bool {
        self.items
            .iter()
            .any(|item| item.severity == PreflightSeverity::Error)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreflightItem {
    pub rule_id: String,
    pub source: Origin,
    pub severity: PreflightSeverity,
    pub scope: PreflightScope,
    pub message: String,
    #[serde(default)]
    pub fix_actions: Vec<FixAction>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum PreflightSeverity {
    Error,
    Warning,
    Info,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PreflightScope {
    Show,
    Cue { cue_id: String },
    TemplateInstance { template_instance_id: String },
    OutputTarget { output_target_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum FixAction {
    OpenAssetResolver,
    RefreshPreview,
    SelectCue { cue_id: String },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlayState {
    pub protocol_version: u32,
    pub stage: Stage,
    pub program_revision: Option<u64>,
    pub items: Vec<OverlayItem>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum OverlayItem {
    Text(OverlayText),
    Image(OverlayImage),
    Clear,
    Blackout,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlayText {
    pub id: String,
    pub template_instance_id: String,
    pub slot_key: String,
    pub text: String,
    pub frame: Rect,
    pub z_index: i32,
    pub style: TextStyle,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct OverlayImage {
    pub id: String,
    pub template_instance_id: String,
    pub slot_key: String,
    pub asset_id: String,
    pub frame: Rect,
    pub z_index: i32,
    pub fit: ImageFit,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TextStyle {
    pub font_family: String,
    pub font_size: u32,
    pub color: String,
    pub weight: FontWeight,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum FontWeight {
    Regular,
    Semibold,
    Bold,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImageFit {
    Cover,
    Contain,
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn extension_metadata_round_trips_without_core_understanding_it() {
        let mut extensions = Extensions::new();
        extensions.insert(
            "com.example.future".to_string(),
            json!({ "pluginSpecific": { "nested": true } }),
        );

        let project = ProjectPackage {
            format_version: 5,
            migrations_applied: vec!["v5-extension-readiness".to_string()],
            metadata: ProjectMetadata {
                id: "project-001".to_string(),
                name: "Round Trip".to_string(),
                created_at: "2026-05-04T00:00:00Z".to_string(),
                updated_at: "2026-05-04T00:00:00Z".to_string(),
            },
            show_definitions: vec![],
            run_sessions: vec![],
            extension_registry: ExtensionRegistry {
                installed_plugins: vec![],
                known_origins: vec![],
            },
            settings: json!({}),
            extensions,
        };

        let encoded = serde_json::to_string_pretty(&project).unwrap();
        let decoded: ProjectPackage = serde_json::from_str(&encoded).unwrap();

        assert_eq!(
            decoded.extensions["com.example.future"]["pluginSpecific"]["nested"],
            json!(true)
        );
    }
}
