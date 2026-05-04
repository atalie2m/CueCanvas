export type ProjectPackage = {
  metadata: { name: string; updatedAt?: string };
  showDefinitions: ShowDefinition[];
  runSessions: RunSession[];
};

export type Origin =
  | { kind: "user"; userId?: string; user_id?: string }
  | { kind: "system" }
  | {
      kind: "plugin";
      pluginId?: string;
      pluginVersion?: string;
      plugin_id?: string;
      plugin_version?: string;
    }
  | { kind: "import"; source: string }
  | { kind: "migration"; migrationId?: string; migration_id?: string };

export type EntityType = "people" | "sessions" | "sponsors";

export type DelimitedFormat = "csv" | "tsv";

export type ImportMode = "append" | "replace" | "merge";

export type ColumnMapping = {
  source: string;
  target: string;
};

export type DataImportReport = {
  entityType: EntityType;
  sourceName: string;
  importedRows: number;
  rejectedRows: number;
  duplicateRows: number;
  conflictedRows: number;
  warnings: string[];
  issues: { rowIndex: number; field: string; message: string }[];
};

export type Stage = { width: number; height: number };

export type Rect = { x: number; y: number; width: number; height: number };

export type ShowDefinition = {
  id: string;
  revision: number;
  name: string;
  stage: Stage;
  outputTargets: OutputTarget[];
  assets: Asset[];
  templateDefinitions: TemplateDefinition[];
  templateInstances: TemplateInstance[];
  cueSheet: { cues: Cue[] };
  typedEntities: {
    people: Person[];
    sessions: Session[];
    sponsors: Sponsor[];
  };
  origin: Origin;
};

export type OutputTarget = {
  id: string;
  name: string;
  kind: "obsBrowserSource" | "embeddedPreview";
  stage: Stage;
  overlayUrl: string;
  origin: Origin;
};

export type Asset = {
  id: string;
  kind: "image" | "video" | "font";
  path: string;
  origin: Origin;
};

export type TemplateKind =
  | "lowerThird"
  | "sessionTitle"
  | "speakerCard"
  | "sponsorBug"
  | "breakScreen";

export type SlotKind = "text" | "assetImage";

export type LocalSlot = {
  key: string;
  kind: SlotKind;
  label: string;
  required: boolean;
  maxLength?: number;
  fallback?: unknown;
};

export type TemplateVariant = { id: string; name: string };

export type TemplateDefinition = {
  id: string;
  name: string;
  kind: TemplateKind;
  version: number;
  localSlots: LocalSlot[];
  variants: TemplateVariant[];
  origin: Origin;
};

export type SlotBinding = { entityType: EntityType; field: string };

export type TemplateInstance = {
  id: string;
  name: string;
  templateDefinitionId: string;
  templateVersionPolicy: "latestCompatible" | { pinned: { version: number } };
  frame: Rect;
  zIndex: number;
  variantId: string;
  slotBindings: Record<string, SlotBinding>;
  visibleByDefault: boolean;
  origin: Origin;
};

export type Cue = {
  id: string;
  number: string;
  name: string;
  segment?: string;
  mode: "absolute";
  cueType: TemplateKind;
  entityRefs: Record<string, EntityRef>;
  slotOverrides: Record<string, Record<string, unknown>>;
  templateStates: TemplateState[];
  transition: { kind: "cut" };
  operatorNotes?: string;
  origin: Origin;
};

export type EntityRef = {
  entityType: EntityType;
  entityId: string;
};

export type TemplateState = {
  templateInstanceId: string;
  visible: boolean;
};

export type Person = {
  id: string;
  displayName: string;
  role?: string;
  organization?: string;
  photo?: string;
  origin: Origin;
};

export type Session = {
  id: string;
  title: string;
  track?: string;
  startTime?: string;
  speakerRefs: string[];
  origin: Origin;
};

export type Sponsor = {
  id: string;
  name: string;
  logo?: string;
  tier?: string;
  origin: Origin;
};

export type TypedEntity = Person | Session | Sponsor;

export type RunSession = {
  id: string;
  liveState: LiveState;
  outputConnectionState?: "disconnected" | "connected" | "degraded";
  obsHealth?: HealthStatus;
  overlayHealth?: HealthStatus;
  warningOverrides?: Record<string, string>;
  revisions: {
    previewRevision: number;
    programRevision: number;
    runtimeRevision: number;
    showDefinitionRevision?: number;
    dataRevision?: number;
  };
  operationLog: OperationLogEntry[];
};

export type LiveState =
  | "setup"
  | "rehearsal"
  | "preflightReady"
  | "liveArmed"
  | "liveRunning"
  | "outputDegraded"
  | "recovering"
  | "ended";

export type HealthStatus = {
  status: "unknown" | "healthy" | "warning" | "error";
  messages: string[];
};

export type ObsConnectionRequest = {
  host: string;
  port: number;
  password?: string;
  mock?: boolean;
};

export type ObsConnectionStatus = {
  connected: boolean;
  host: string;
  port: number;
  obsVersion?: string;
  websocketVersion?: string;
  message: string;
};

export type ObsDesiredState = {
  sceneName: string;
  sourceName: string;
  url: string;
  stage: Stage;
  shutdownWhenNotVisible: boolean;
  refreshWhenActive: boolean;
  customCss: string;
};

export type ObsObservedState = {
  sceneExists: boolean;
  sourceExists: boolean;
  url: string;
  width: number;
  height: number;
  shutdownWhenNotVisible: boolean;
  refreshWhenActive: boolean;
  customCss: string;
  visibleInScenePath: boolean;
  overlayConnected: boolean;
};

export type ObsIssue = {
  ruleId: string;
  severity: "error" | "warning" | "info";
  message: string;
};

export type ObsSetupRequest = {
  connection: ObsConnectionRequest;
  desired: ObsDesiredState;
};

export type ObsSetupResponse = {
  connection: ObsConnectionStatus;
  desired: ObsDesiredState;
  observed: ObsObservedState;
  issues: ObsIssue[];
  actions: string[];
};

export type OperationLogEntry = {
  id: string;
  at: string;
  action: string;
  message: string;
  previewRevision?: number;
  programRevision?: number;
};

export type PreviewSnapshot = {
  id: string;
  sourceCueId: string;
  showDefinitionRevision?: number;
  previewRevision: number;
  resolvedData: Record<string, unknown>;
  overlayState?: OverlayState;
  originChain?: Origin[];
  preflightResult: { items: PreflightItem[] };
};

export type ProgramSnapshot = {
  id: string;
  sourceCueId: string;
  sourcePreviewSnapshotId: string;
  showDefinitionRevision?: number;
  programRevision: number;
  resolvedData: Record<string, unknown>;
  overlayState?: OverlayState;
  originChain?: Origin[];
  outputTargetId?: string;
};

export type OverlayState = {
  protocolVersion: number;
  stage: Stage;
  programRevision?: number;
  items: OverlayItem[];
};

export type OverlayItem =
  | {
      kind: "text";
      id: string;
      templateInstanceId: string;
      slotKey: string;
      text: string;
      frame: Rect;
      zIndex: number;
      style: Record<string, unknown>;
    }
  | {
      kind: "image";
      id: string;
      templateInstanceId: string;
      slotKey: string;
      assetId: string;
      frame: Rect;
      zIndex: number;
      fit: string;
    }
  | {
      kind: "rect";
      id: string;
      frame: Rect;
      zIndex: number;
      style: { fill: string; opacity: number };
    }
  | { kind: "clear" }
  | { kind: "blackout" };

export type PreflightItem = {
  ruleId: string;
  severity: "error" | "warning" | "info";
  scope?: PreflightScope;
  scopeLabel?: string;
  source?: Origin;
  sourceLabel?: string;
  message: string;
  fixActions?: FixAction[];
  warningKey?: string;
  valueHash?: string;
  overridden?: boolean;
};

export type PreflightScope =
  | { kind: "show" }
  | { kind: "cue"; cueId: string }
  | { kind: "templateInstance"; templateInstanceId: string }
  | { kind: "outputTarget"; outputTargetId: string };

export type FixAction =
  | { kind: "openAssetResolver" }
  | { kind: "refreshPreview" }
  | { kind: "selectCue"; cueId: string };

export type PreflightState = {
  showDefinitionId: string;
  runSessionId: string;
  runtimeRevision: number;
  groups: {
    errors: PreflightItem[];
    warnings: PreflightItem[];
    infos: PreflightItem[];
  };
  totals: {
    errors: number;
    warnings: number;
    infos: number;
    activeWarnings: number;
    overriddenWarnings: number;
  };
};

export type RuntimeCommandResponse = {
  command: string;
  revision: number;
  result: unknown;
};

export type RuntimeEvent =
  | {
      event: "previewChanged";
      payload: PreviewSnapshot;
    }
  | {
      event: "programChanged";
      payload: ProgramSnapshot;
    }
  | {
      event: "operationLogged" | "autosaveChanged";
      payload: unknown;
    }
  | {
      event: "preflightChanged";
      payload: PreflightState;
    }
  | {
      event: "overlayConnectionChanged" | "overlayRendered";
      payload: unknown;
    }
  | {
      event: "runtimeError";
      message: string;
    }
  | {
      event: string;
      payload?: unknown;
      message?: string;
    };
