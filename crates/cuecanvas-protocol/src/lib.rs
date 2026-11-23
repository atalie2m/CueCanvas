use cuecanvas_model::{
    Actor, Cue, EntityType, FixAction, LiveState, Origin, OutputTarget, OverlayState, Person,
    PreflightScope, PreflightSeverity, PreviewSnapshot, ProgramSnapshot, ProjectPackage, Rect,
    RunSession, Session, Sponsor, Stage, TemplateInstance,
};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CommandEnvelope<T> {
    pub command: String,
    pub actor: Actor,
    pub expected_revision: Option<u64>,
    pub idempotency_key: Option<String>,
    pub payload: T,
}

pub type RuntimeCommandEnvelope = CommandEnvelope<Value>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeCommandResponse {
    pub command: String,
    pub revision: u64,
    pub result: Value,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CreateShowPayload {
    pub name: String,
    pub stage: Stage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateStagePayload {
    pub show_definition_id: Option<String>,
    pub stage: Stage,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct UpdateOutputTargetPayload {
    pub show_definition_id: Option<String>,
    pub output_target: OutputTarget,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct RunSessionPayload {
    pub run_session: Option<RunSession>,
    pub run_session_id: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreviewCuePayload {
    pub run_session_id: Option<String>,
    pub cue_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TakePayload {
    pub run_session_id: Option<String>,
    pub output_target_id: String,
    pub preview_snapshot_id: String,
    pub preview_revision: u64,
    pub expected_program_revision: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct EmergencyLivePayload {
    pub run_session_id: Option<String>,
    pub output_target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TestPatternPayload {
    pub run_session_id: Option<String>,
    pub output_target_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CuePayload {
    pub show_definition_id: Option<String>,
    pub cue: Cue,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteCuePayload {
    pub show_definition_id: Option<String>,
    pub cue_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ReorderCuesPayload {
    pub show_definition_id: Option<String>,
    pub cue_ids: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct BulkGenerateCuesPayload {
    pub show_definition_id: Option<String>,
    pub entity_type: EntityType,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CueStillRequestPayload {
    pub show_definition_id: Option<String>,
    pub cue_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct TemplateInstancePayload {
    pub show_definition_id: Option<String>,
    pub template_instance: TemplateInstance,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTemplateInstancePayload {
    pub show_definition_id: Option<String>,
    pub template_instance_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "entityType", rename_all = "camelCase")]
pub enum TypedEntityPayload {
    People {
        show_definition_id: Option<String>,
        entity: Person,
    },
    Sessions {
        show_definition_id: Option<String>,
        entity: Session,
    },
    Sponsors {
        show_definition_id: Option<String>,
        entity: Sponsor,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DeleteTypedEntityPayload {
    pub show_definition_id: Option<String>,
    pub entity_type: EntityType,
    pub entity_id: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum DelimitedFormat {
    Csv,
    Tsv,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ColumnMapping {
    pub source: String,
    pub target: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub enum ImportMode {
    Append,
    Replace,
    Merge,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct DataImportPayload {
    pub entity_type: EntityType,
    pub source_name: String,
    pub format: DelimitedFormat,
    pub content: String,
    pub mappings: Vec<ColumnMapping>,
    #[serde(default)]
    pub replace_existing: bool,
    pub mode: Option<ImportMode>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct WarningOverridePayload {
    pub run_session_id: Option<String>,
    pub warning_key: String,
    pub value_hash: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct LiveStateTransitionPayload {
    pub run_session_id: Option<String>,
    pub live_state: LiveState,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct CueStillPayload {
    pub cue_id: String,
    pub frame: Rect,
    pub data_fingerprint: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreflightState {
    pub show_definition_id: String,
    pub run_session_id: String,
    pub runtime_revision: u64,
    pub groups: PreflightGroups,
    pub totals: PreflightTotals,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreflightGroups {
    pub errors: Vec<PreflightDisplayItem>,
    pub warnings: Vec<PreflightDisplayItem>,
    pub infos: Vec<PreflightDisplayItem>,
}

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreflightTotals {
    pub errors: usize,
    pub warnings: usize,
    pub infos: usize,
    pub active_warnings: usize,
    pub overridden_warnings: usize,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct PreflightDisplayItem {
    pub rule_id: String,
    pub severity: PreflightSeverity,
    pub scope: PreflightScope,
    pub scope_label: String,
    pub source: Origin,
    pub source_label: String,
    pub message: String,
    pub fix_actions: Vec<FixAction>,
    pub warning_key: Option<String>,
    pub value_hash: Option<String>,
    pub overridden: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(tag = "event", rename_all = "camelCase")]
pub enum RuntimeEvent {
    PreviewChanged {
        source: Actor,
        revision: u64,
        payload: PreviewSnapshot,
    },
    ProgramChanged {
        source: Actor,
        revision: u64,
        payload: ProgramSnapshot,
    },
    OverlaySnapshot {
        sequence: u64,
        payload: OverlayState,
    },
    OperationLogged {
        source: Actor,
        revision: u64,
        payload: Value,
    },
    PreflightChanged {
        source: Actor,
        revision: u64,
        payload: Value,
    },
    OverlayConnectionChanged {
        source: Actor,
        revision: u64,
        payload: Value,
    },
    OverlayRendered {
        source: Actor,
        revision: u64,
        payload: Value,
    },
    AutosaveChanged {
        source: Actor,
        revision: u64,
        payload: Value,
    },
    RuntimeError {
        source: Actor,
        revision: u64,
        message: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct HealthResponse {
    pub ok: bool,
    pub service: String,
    pub version: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPersistenceRequest {
    pub package_dir: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
pub struct ProjectPersistenceResponse {
    pub package_dir: String,
    pub project: ProjectPackage,
}
