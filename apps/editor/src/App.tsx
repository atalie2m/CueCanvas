import { useCallback, useEffect, useMemo, useState } from "react";
import type { CSSProperties, ReactNode } from "react";
import {
  Activity,
  AlertTriangle,
  Ban,
  CheckCircle2,
  Database,
  Eye,
  Layers,
  ListChecks,
  MonitorUp,
  Play,
  Plus,
  RotateCcw,
  Save,
  ShieldCheck,
  Square,
  Trash2,
  Undo2,
  Upload,
  Wand2,
} from "lucide-react";
import { snapshotDiff } from "./diff";
import {
  buildImportPreview,
  inferColumnMappings,
  sampleContent,
} from "./importMapping";
import type {
  ColumnMapping,
  Cue,
  DataImportReport,
  DelimitedFormat,
  EntityType,
  FixAction,
  HealthStatus,
  ImportMode,
  LiveState,
  LocalSlot,
  Origin,
  PreflightItem,
  PreflightState,
  Person,
  PreviewSnapshot,
  ProgramSnapshot,
  ProjectPackage,
  RuntimeCommandResponse,
  RuntimeEvent,
  Session,
  ShowDefinition,
  Sponsor,
  TemplateDefinition,
  TemplateInstance,
  TemplateKind,
  TypedEntity,
} from "./types";

const runtimeBase = "";
const runtimeTokens = readRuntimeTokens();
const editorAuthHeaders: Record<string, string> = runtimeTokens.editorToken
  ? { "X-CueCanvas-Editor-Token": runtimeTokens.editorToken }
  : {};
const overlaySnapshotPath = runtimeTokens.overlayToken
  ? `/api/overlay/program/snapshot?token=${encodeURIComponent(runtimeTokens.overlayToken)}`
  : "/api/overlay/program/snapshot";

type EntityDraft = {
  id: string;
  displayName: string;
  role: string;
  organization: string;
  photo: string;
  title: string;
  track: string;
  startTime: string;
  speakerRefs: string;
  name: string;
  logo: string;
  tier: string;
};

type CueDraft = {
  id: string;
  number: string;
  name: string;
  segment: string;
  entityType: EntityType;
  entityId: string;
  templateInstanceId: string;
  operatorNotes: string;
};

export function App() {
  const [project, setProject] = useState<ProjectPackage | null>(null);
  const [selectedCueId, setSelectedCueId] = useState<string>("cue-001");
  const [selectedTemplateDefinitionId, setSelectedTemplateDefinitionId] =
    useState<string>("");
  const [selectedTemplateInstanceId, setSelectedTemplateInstanceId] =
    useState<string>("");
  const [instanceDraft, setInstanceDraft] = useState<TemplateInstance | null>(
    null,
  );
  const [preview, setPreview] = useState<PreviewSnapshot | null>(null);
  const [program, setProgram] = useState<ProgramSnapshot | null>(null);
  const [preflightState, setPreflightState] = useState<PreflightState | null>(
    null,
  );
  const [status, setStatus] = useState("Runtime not checked");
  const [entityType, setEntityType] = useState<EntityType>("people");
  const [entityDraft, setEntityDraft] = useState<EntityDraft>(emptyEntityDraft);
  const [cueDraft, setCueDraft] = useState<CueDraft>(emptyCueDraft);
  const [importEntityType, setImportEntityType] =
    useState<EntityType>("people");
  const [importFormat, setImportFormat] = useState<DelimitedFormat>("csv");
  const [importMode, setImportMode] = useState<ImportMode>("replace");
  const [importSourceName, setImportSourceName] = useState("people.csv");
  const [importContent, setImportContent] = useState(() =>
    sampleContent("people", "csv"),
  );
  const [importMappings, setImportMappings] = useState<ColumnMapping[]>([]);
  const [importReport, setImportReport] = useState<DataImportReport | null>(
    null,
  );
  const [isImporting, setIsImporting] = useState(false);

  const show = project?.showDefinitions[0] ?? null;
  const run = project?.runSessions[0] ?? null;
  const cues = show?.cueSheet.cues ?? [];
  const selectedCue =
    cues.find((cue) => cue.id === selectedCueId) ?? cues[0] ?? null;
  const templateDefinitions = show?.templateDefinitions ?? [];
  const templateInstances = show?.templateInstances ?? [];
  const selectedTemplateDefinition =
    templateDefinitions.find(
      (definition) => definition.id === selectedTemplateDefinitionId,
    ) ??
    templateDefinitions[0] ??
    null;
  const selectedTemplateInstance =
    templateInstances.find(
      (instance) => instance.id === selectedTemplateInstanceId,
    ) ??
    templateInstances[0] ??
    null;
  const diffRows = useMemo(
    () => snapshotDiff(program, preview),
    [program, preview],
  );
  const effectivePreflight = preflightState ?? preflightFromPreview(preview);
  const isLiveRunning = run?.liveState === "liveRunning";
  const canTake =
    Boolean(preview) && (effectivePreflight?.totals.errors ?? 0) === 0;
  const importPreview = useMemo(
    () => buildImportPreview(importContent, importFormat, importEntityType),
    [importContent, importEntityType, importFormat],
  );
  const importRequiredTarget = importPreview.requiredTarget;
  const canImport =
    importContent.trim().length > 0 &&
    importSourceName.trim().length > 0 &&
    importMappings.some((mapping) => mapping.target === importRequiredTarget) &&
    !isImporting;
  const stalePreview = Boolean(
    preview &&
      show &&
      preview.showDefinitionRevision !== undefined &&
      preview.showDefinitionRevision !== show.revision,
  );

  const entities = useMemo(
    () => entityList(show, entityType),
    [entityType, show],
  );
  const cueEntityOptions = useMemo(
    () => entityList(show, cueDraft.entityType),
    [cueDraft.entityType, show],
  );
  const templateById = useMemo(
    () =>
      new Map(
        templateDefinitions.map((definition) => [definition.id, definition]),
      ),
    [templateDefinitions],
  );

  const loadProject = useCallback(async () => {
    try {
      const [projectResponse, snapshotResponse, preflightResponse] =
        await Promise.all([
          fetch(`${runtimeBase}/api/project`, { headers: editorAuthHeaders }),
          fetch(`${runtimeBase}${overlaySnapshotPath}`),
          fetch(`${runtimeBase}/api/preflight`, { headers: editorAuthHeaders }),
        ]);
      if (!projectResponse.ok)
        throw new Error("Runtime project API is unavailable");
      setProject(await projectResponse.json());
      if (snapshotResponse.ok) {
        setProgram(await snapshotResponse.json());
      }
      if (preflightResponse.ok) {
        setPreflightState(await preflightResponse.json());
      }
      setStatus("Runtime connected");
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Runtime unavailable");
    }
  }, []);

  const sendCommand = useCallback(
    async <T,>(
      command: string,
      payload: unknown,
      options: {
        expectedRevision?: number | null;
        idempotencyKey?: string;
      } = {},
    ) => {
      const response = await fetch(`${runtimeBase}/api/commands`, {
        method: "POST",
        headers: { ...editorAuthHeaders, "Content-Type": "application/json" },
        body: JSON.stringify({
          command,
          actor: { kind: "user", user_id: "local-user" },
          expectedRevision: options.expectedRevision ?? null,
          idempotencyKey:
            options.idempotencyKey ??
            `${command}-${Date.now()}-${Math.random().toString(16).slice(2)}`,
          payload,
        }),
      });
      if (!response.ok) throw new Error(await readError(response));
      const body = (await response.json()) as RuntimeCommandResponse;
      await loadProject();
      return body.result as T;
    },
    [loadProject],
  );

  useEffect(() => {
    void loadProject();
  }, [loadProject]);

  useEffect(() => {
    setImportMappings(
      inferColumnMappings(importPreview.headers, importEntityType),
    );
  }, [importEntityType, importPreview.headers]);

  useEffect(() => {
    if (!runtimeTokens.editorToken) return;

    let closed = false;
    let retry: number | undefined;
    let socket: WebSocket | undefined;
    const protocol = window.location.protocol === "https:" ? "wss" : "ws";
    const query = `?editorToken=${encodeURIComponent(runtimeTokens.editorToken)}`;

    const connect = () => {
      socket = new WebSocket(
        `${protocol}://${window.location.host}/ws/editor${query}`,
      );
      socket.onmessage = (event) => {
        const message = JSON.parse(event.data) as RuntimeEvent;
        if (message.event === "previewChanged") {
          setPreview(message.payload as PreviewSnapshot);
        } else if (message.event === "programChanged") {
          setProgram(message.payload as ProgramSnapshot);
        } else if (message.event === "preflightChanged") {
          setPreflightState(message.payload as PreflightState);
        } else if (message.event === "runtimeError" && message.message) {
          setStatus(message.message);
        }
        void loadProject();
      };
      socket.onclose = () => {
        if (!closed) retry = window.setTimeout(connect, 1000);
      };
    };

    connect();
    return () => {
      closed = true;
      if (retry) window.clearTimeout(retry);
      socket?.close();
    };
  }, [loadProject]);

  useEffect(() => {
    if (!show) return;
    if (!cues.some((cue) => cue.id === selectedCueId)) {
      setSelectedCueId(cues[0]?.id ?? "");
    }
    if (
      !templateDefinitions.some(
        (definition) => definition.id === selectedTemplateDefinitionId,
      )
    ) {
      setSelectedTemplateDefinitionId(templateDefinitions[0]?.id ?? "");
    }
    if (
      !templateInstances.some(
        (instance) => instance.id === selectedTemplateInstanceId,
      )
    ) {
      setSelectedTemplateInstanceId(templateInstances[0]?.id ?? "");
    }
  }, [
    cues,
    selectedCueId,
    selectedTemplateDefinitionId,
    selectedTemplateInstanceId,
    show,
    templateDefinitions,
    templateInstances,
  ]);

  useEffect(() => {
    setInstanceDraft(selectedTemplateInstance ?? null);
  }, [selectedTemplateInstance]);

  useEffect(() => {
    if (!selectedCue) return;
    const firstRef = Object.values(selectedCue.entityRefs)[0];
    setCueDraft({
      id: selectedCue.id,
      number: selectedCue.number,
      name: selectedCue.name,
      segment: selectedCue.segment ?? "",
      entityType: firstRef?.entityType ?? "people",
      entityId: firstRef?.entityId ?? "",
      templateInstanceId:
        selectedCue.templateStates[0]?.templateInstanceId ??
        templateInstances[0]?.id ??
        "",
      operatorNotes: selectedCue.operatorNotes ?? "",
    });
  }, [selectedCue, templateInstances]);

  async function previewCue(cue: Cue) {
    try {
      setPreview(
        await sendCommand<PreviewSnapshot>(
          "live.previewCue",
          { cueId: cue.id },
          { idempotencyKey: `editor-preview-${cue.id}-${Date.now()}` },
        ),
      );
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Preview failed");
    }
  }

  async function takePreview() {
    if (!preview) return;
    try {
      setProgram(
        await sendCommand<ProgramSnapshot>(
          "live.take",
          {
            outputTargetId: "output-program-obs",
            previewSnapshotId: preview.id,
            previewRevision: preview.previewRevision,
            expectedProgramRevision: program?.programRevision ?? 0,
          },
          { idempotencyKey: `editor-take-${preview.id}` },
        ),
      );
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Take failed");
    }
  }

  async function emergencyCommand(kind: "clear" | "blackout" | "restore") {
    try {
      setProgram(
        await sendCommand<ProgramSnapshot>(
          `live.${kind}`,
          { outputTargetId: "output-program-obs" },
          { idempotencyKey: `editor-${kind}-${Date.now()}` },
        ),
      );
    } catch (error) {
      setStatus(error instanceof Error ? error.message : `${kind} failed`);
    }
  }

  async function changeLiveState(liveState: LiveState) {
    if (!run) return;
    try {
      await sendCommand(
        "live.transition",
        { runSessionId: run.id, liveState },
        { expectedRevision: run.revisions.runtimeRevision },
      );
    } catch (error) {
      setStatus(
        error instanceof Error ? error.message : "Live state change failed",
      );
    }
  }

  async function overrideWarning(item: PreflightItem) {
    if (!item.warningKey || !item.valueHash || !run) return;
    try {
      await sendCommand(
        "warningOverride.upsert",
        {
          runSessionId: run.id,
          warningKey: item.warningKey,
          valueHash: item.valueHash,
        },
        { expectedRevision: run.revisions.runtimeRevision },
      );
    } catch (error) {
      setStatus(
        error instanceof Error ? error.message : "Warning override failed",
      );
    }
  }

  function allowEditInCurrentLiveState() {
    return (
      !isLiveRunning ||
      window.confirm("Live is running. Confirm this edit before applying it.")
    );
  }

  async function importData() {
    if (!canImport) return;
    if (!allowEditInCurrentLiveState()) return;
    setIsImporting(true);
    setImportReport(null);
    try {
      const report = await sendCommand<DataImportReport>(
        "data.import",
        {
          entityType: importEntityType,
          sourceName: importSourceName.trim(),
          format: importFormat,
          content: importContent,
          mappings: importMappings.filter((mapping) => mapping.target),
          mode: importMode,
          replaceExisting: importMode === "replace",
        },
        { expectedRevision: show?.revision ?? null },
      );
      setImportReport(report);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Import failed");
    } finally {
      setIsImporting(false);
    }
  }

  async function saveEntity() {
    if (!show) return;
    if (!allowEditInCurrentLiveState()) return;
    const entity = buildEntity(entityType, entityDraft);
    try {
      await sendCommand(
        "entity.upsert",
        { entityType, showDefinitionId: show.id, entity },
        { expectedRevision: show.revision },
      );
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Entity save failed");
    }
  }

  async function deleteEntity(entityId: string) {
    if (!show) return;
    if (!allowEditInCurrentLiveState()) return;
    try {
      await sendCommand(
        "entity.delete",
        { showDefinitionId: show.id, entityType, entityId },
        { expectedRevision: show.revision },
      );
      setEntityDraft(emptyEntityDraft());
    } catch (error) {
      setStatus(
        error instanceof Error ? error.message : "Entity delete failed",
      );
    }
  }

  async function addTemplateInstance(definition: TemplateDefinition) {
    if (!show) return;
    if (!allowEditInCurrentLiveState()) return;
    const instance = defaultTemplateInstance(definition);
    try {
      const saved = await sendCommand<TemplateInstance>(
        "templateInstance.upsert",
        { showDefinitionId: show.id, templateInstance: instance },
        { expectedRevision: show.revision },
      );
      setSelectedTemplateInstanceId(saved.id);
    } catch (error) {
      setStatus(
        error instanceof Error
          ? error.message
          : "Template instance save failed",
      );
    }
  }

  async function saveTemplateInstance() {
    if (!show || !instanceDraft) return;
    if (!allowEditInCurrentLiveState()) return;
    try {
      await sendCommand(
        "templateInstance.upsert",
        { showDefinitionId: show.id, templateInstance: instanceDraft },
        { expectedRevision: show.revision },
      );
    } catch (error) {
      setStatus(
        error instanceof Error
          ? error.message
          : "Template instance save failed",
      );
    }
  }

  async function deleteTemplateInstance(instanceId: string) {
    if (!show) return;
    if (!allowEditInCurrentLiveState()) return;
    try {
      await sendCommand(
        "templateInstance.delete",
        { showDefinitionId: show.id, templateInstanceId: instanceId },
        { expectedRevision: show.revision },
      );
    } catch (error) {
      setStatus(
        error instanceof Error
          ? error.message
          : "Template instance delete failed",
      );
    }
  }

  async function saveCue() {
    if (!show || !cueDraft.templateInstanceId) return;
    if (!allowEditInCurrentLiveState()) return;
    const instance = templateInstances.find(
      (candidate) => candidate.id === cueDraft.templateInstanceId,
    );
    const definition = instance
      ? templateById.get(instance.templateDefinitionId)
      : null;
    const cue = buildCue(cueDraft, definition?.kind ?? "lowerThird");
    try {
      const saved = await sendCommand<Cue>(
        "cue.upsert",
        { showDefinitionId: show.id, cue },
        { expectedRevision: show.revision },
      );
      setSelectedCueId(saved.id);
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Cue save failed");
    }
  }

  async function deleteCue(cueId: string) {
    if (!show) return;
    if (!allowEditInCurrentLiveState()) return;
    try {
      await sendCommand(
        "cue.delete",
        { showDefinitionId: show.id, cueId },
        { expectedRevision: show.revision },
      );
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Cue delete failed");
    }
  }

  async function moveCue(direction: -1 | 1) {
    if (!show || !selectedCue) return;
    if (!allowEditInCurrentLiveState()) return;
    const index = cues.findIndex((cue) => cue.id === selectedCue.id);
    const next = index + direction;
    if (index < 0 || next < 0 || next >= cues.length) return;
    const cueIds = cues.map((cue) => cue.id);
    [cueIds[index], cueIds[next]] = [cueIds[next], cueIds[index]];
    try {
      await sendCommand(
        "cue.reorder",
        { showDefinitionId: show.id, cueIds },
        { expectedRevision: show.revision },
      );
    } catch (error) {
      setStatus(error instanceof Error ? error.message : "Cue reorder failed");
    }
  }

  async function bulkGenerateCues() {
    if (!show) return;
    if (!allowEditInCurrentLiveState()) return;
    try {
      await sendCommand(
        "cue.bulkGenerate",
        { showDefinitionId: show.id, entityType },
        { expectedRevision: show.revision },
      );
    } catch (error) {
      setStatus(
        error instanceof Error ? error.message : "Bulk cue generation failed",
      );
    }
  }

  function changeImportEntity(nextEntityType: EntityType) {
    setImportEntityType(nextEntityType);
    setImportSourceName(`${nextEntityType}.${importFormat}`);
    setImportContent(sampleContent(nextEntityType, importFormat));
    setImportReport(null);
  }

  function changeImportFormat(format: DelimitedFormat) {
    setImportFormat(format);
    setImportSourceName(`${importEntityType}.${format}`);
    setImportContent(sampleContent(importEntityType, format));
    setImportReport(null);
  }

  function selectEntity(entity: TypedEntity) {
    setEntityDraft(draftFromEntity(entityType, entity));
  }

  return (
    <main className="app-shell">
      <aside className="sidebar" aria-label="Cue sheet">
        <div className="brand">
          <MonitorUp size={22} />
          <div>
            <strong>CueCanvas</strong>
            <span>{show?.name ?? "No show loaded"}</span>
          </div>
        </div>

        <div className="section-title">
          <ListChecks size={16} />
          Cue Sheet
        </div>
        <div className="cue-list">
          {cues.map((cue) => (
            <button
              className={
                cue.id === selectedCue?.id ? "cue-row selected" : "cue-row"
              }
              key={cue.id}
              type="button"
              onClick={() => setSelectedCueId(cue.id)}
            >
              <span>{cue.number}</span>
              <strong>{cue.name}</strong>
            </button>
          ))}
        </div>

        <div className="section-title">
          <Database size={16} />
          Data
        </div>
        <dl className="data-summary">
          <div>
            <dt>People</dt>
            <dd>{show?.typedEntities.people.length ?? 0}</dd>
          </div>
          <div>
            <dt>Sessions</dt>
            <dd>{show?.typedEntities.sessions.length ?? 0}</dd>
          </div>
          <div>
            <dt>Sponsors</dt>
            <dd>{show?.typedEntities.sponsors.length ?? 0}</dd>
          </div>
        </dl>
      </aside>

      <section className="workbench">
        <header className="topbar">
          <div>
            <p className="eyeline">Live run</p>
            <h1>{selectedCue?.name ?? "Select a cue"}</h1>
          </div>
          <div className="status">
            <Activity size={16} />
            {status}
          </div>
        </header>

        <LiveStateControls
          liveState={run?.liveState ?? "setup"}
          preflight={effectivePreflight}
          obsHealth={run?.obsHealth}
          overlayHealth={run?.overlayHealth}
          outputConnectionState={run?.outputConnectionState}
          onChange={(liveState) => void changeLiveState(liveState)}
        />

        {isLiveRunning ? (
          <div className="live-lock-banner">
            <ShieldCheck size={16} />
            Live Mode is running. Setup, data, cue, and template edits require
            confirmation before Runtime accepts them.
          </div>
        ) : null}

        {stalePreview ? (
          <div className="stale-banner">
            <AlertTriangle size={16} />
            Preview is stale. Take will use the existing PreviewSnapshot until
            you refresh Preview.
          </div>
        ) : null}

        <section className="monitors" aria-label="Preview and program monitors">
          <Monitor
            title="Preview"
            snapshotId={preview?.id}
            revision={preview?.previewRevision}
          >
            {preview ? (
              <ResolvedData data={preview.resolvedData} />
            ) : (
              <EmptyMonitor text="No preview snapshot" />
            )}
          </Monitor>
          <Monitor
            title="Program"
            snapshotId={program?.id}
            revision={program?.programRevision}
          >
            {program ? (
              <ResolvedData data={program.resolvedData} />
            ) : (
              <EmptyMonitor text="Program is clear" />
            )}
          </Monitor>
        </section>

        <section className="command-bar" aria-label="Live commands">
          <button
            type="button"
            onClick={() => selectedCue && void previewCue(selectedCue)}
          >
            <Eye size={16} />
            Preview
          </button>
          <button
            className="take"
            type="button"
            disabled={!canTake}
            onClick={() => void takePreview()}
          >
            <Play size={16} />
            Take
          </button>
          <button type="button" onClick={() => void loadProject()}>
            <RotateCcw size={16} />
            Refresh
          </button>
          <button type="button" onClick={() => void emergencyCommand("clear")}>
            <Square size={16} />
            Clear
          </button>
          <button
            type="button"
            onClick={() => void emergencyCommand("blackout")}
          >
            <Ban size={16} />
            Blackout
          </button>
          <button
            type="button"
            onClick={() => void emergencyCommand("restore")}
          >
            <Undo2 size={16} />
            Restore
          </button>
        </section>

        <section className="product-grid">
          <Panel title="Template Library">
            <TemplateLibrary
              definitions={templateDefinitions}
              selectedId={selectedTemplateDefinition?.id ?? ""}
              disabled={isLiveRunning}
              onSelect={setSelectedTemplateDefinitionId}
              onAdd={(definition) => void addTemplateInstance(definition)}
            />
          </Panel>

          <Panel title="Stage / Template Mode">
            <StageTemplatePanel
              show={show}
              definitions={templateDefinitions}
              instances={templateInstances}
              selectedInstanceId={selectedTemplateInstance?.id ?? ""}
              draft={instanceDraft}
              disabled={isLiveRunning}
              onSelect={setSelectedTemplateInstanceId}
              onDraftChange={setInstanceDraft}
              onSave={() => void saveTemplateInstance()}
              onDelete={(id) => void deleteTemplateInstance(id)}
            />
          </Panel>

          <Panel title="Data Editors">
            <EntityEditor
              entityType={entityType}
              entities={entities}
              draft={entityDraft}
              onEntityTypeChange={(next) => {
                setEntityType(next);
                setEntityDraft(emptyEntityDraft());
              }}
              onDraftChange={setEntityDraft}
              onSelect={selectEntity}
              onNew={() => setEntityDraft(emptyEntityDraft())}
              disabled={isLiveRunning}
              onSave={() => void saveEntity()}
              onDelete={(id) => void deleteEntity(id)}
            />
          </Panel>

          <Panel title="Import Review">
            <DataImportPanel
              entityType={importEntityType}
              format={importFormat}
              mode={importMode}
              sourceName={importSourceName}
              content={importContent}
              preview={importPreview}
              mappings={importMappings}
              report={importReport}
              canImport={canImport && !isLiveRunning}
              isImporting={isImporting}
              onEntityTypeChange={changeImportEntity}
              onFormatChange={changeImportFormat}
              onModeChange={setImportMode}
              onSourceNameChange={(value) => {
                setImportSourceName(value);
                setImportReport(null);
              }}
              onContentChange={(value) => {
                setImportContent(value);
                setImportReport(null);
              }}
              onMappingChange={(source, target) => {
                setImportMappings((current) => {
                  const next = current.filter(
                    (mapping) => mapping.source !== source,
                  );
                  if (target) next.push({ source, target });
                  return next;
                });
                setImportReport(null);
              }}
              onImport={() => void importData()}
            />
          </Panel>

          <Panel title="Cue Editor">
            <CueEditor
              draft={cueDraft}
              cues={cues}
              entities={cueEntityOptions}
              templateInstances={templateInstances}
              templateById={templateById}
              disabled={isLiveRunning}
              onDraftChange={setCueDraft}
              onSave={() => void saveCue()}
              onDelete={() => selectedCue && void deleteCue(selectedCue.id)}
              onMoveUp={() => void moveCue(-1)}
              onMoveDown={() => void moveCue(1)}
              onNew={() => {
                setSelectedCueId("");
                setCueDraft({
                  ...emptyCueDraft(),
                  number: String(cues.length + 1).padStart(3, "0"),
                  templateInstanceId: templateInstances[0]?.id ?? "",
                });
              }}
              onBulkGenerate={() => void bulkGenerateCues()}
            />
          </Panel>

          <Panel title="Diff Preview">
            {diffRows.length === 0 ? (
              <p className="muted">No pending differences.</p>
            ) : (
              <table>
                <thead>
                  <tr>
                    <th>Field</th>
                    <th>Category</th>
                    <th>Program</th>
                    <th>Preview</th>
                  </tr>
                </thead>
                <tbody>
                  {diffRows.map((row) => (
                    <tr key={`${row.category}-${row.key}`}>
                      <td>{row.key}</td>
                      <td>{row.category}</td>
                      <td>{row.program}</td>
                      <td>{row.preview}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </Panel>

          <Panel title="Preflight">
            <PreflightPanel
              state={effectivePreflight}
              onOverride={(item) => void overrideWarning(item)}
              onRefreshPreview={() =>
                selectedCue && void previewCue(selectedCue)
              }
              onSelectCue={setSelectedCueId}
            />
          </Panel>

          <Panel title="Operation Log">
            <ol className="operation-log">
              {(run?.operationLog ?? []).slice(-6).map((entry) => (
                <li key={entry.id}>
                  <span>{entry.action}</span>
                  {entry.message}
                </li>
              ))}
            </ol>
          </Panel>
        </section>
      </section>
    </main>
  );
}

async function readError(response: Response) {
  try {
    const body = (await response.json()) as { error?: string };
    return body.error ?? `Request failed with ${response.status}`;
  } catch {
    return `Request failed with ${response.status}`;
  }
}

function readRuntimeTokens() {
  const search = typeof window === "undefined" ? "" : window.location.search;
  const params = new URLSearchParams(search);
  const env = (
    import.meta as ImportMeta & {
      env?: Record<string, string | undefined>;
    }
  ).env;

  return {
    editorToken:
      params.get("editorToken") ?? env?.VITE_CUECANVAS_EDITOR_TOKEN ?? "",
    overlayToken:
      params.get("overlayToken") ??
      params.get("token") ??
      env?.VITE_CUECANVAS_OVERLAY_TOKEN ??
      "",
  };
}

function Monitor({
  title,
  snapshotId,
  revision,
  children,
}: {
  title: string;
  snapshotId?: string;
  revision?: number;
  children: ReactNode;
}) {
  return (
    <article className="monitor">
      <header>
        <strong>{title}</strong>
        <span>{revision ? `rev ${revision}` : "not ready"}</span>
      </header>
      <div className="monitor-body">{children}</div>
      <footer>{snapshotId ?? "no snapshot"}</footer>
    </article>
  );
}

function EmptyMonitor({ text }: { text: string }) {
  return <div className="empty-monitor">{text}</div>;
}

function Panel({ title, children }: { title: string; children: ReactNode }) {
  return (
    <section className="panel">
      <h2>{title}</h2>
      {children}
    </section>
  );
}

function LiveStateControls({
  liveState,
  preflight,
  obsHealth,
  overlayHealth,
  outputConnectionState,
  onChange,
}: {
  liveState: LiveState;
  preflight: PreflightState | null;
  obsHealth?: HealthStatus;
  overlayHealth?: HealthStatus;
  outputConnectionState?: string;
  onChange: (liveState: LiveState) => void;
}) {
  const states: LiveState[] = [
    "setup",
    "rehearsal",
    "preflightReady",
    "liveArmed",
    "liveRunning",
    "ended",
  ];
  return (
    <section className="live-status-strip" aria-label="Live status">
      <div className="segmented" role="group" aria-label="Live state">
        {states.map((state) => (
          <button
            className={liveState === state ? "selected" : ""}
            key={state}
            type="button"
            onClick={() => onChange(state)}
          >
            {liveStateLabel(state)}
          </button>
        ))}
      </div>
      <div className="health-grid">
        <HealthPill
          label="Preflight"
          status={preflightStatus(preflight)}
          detail={`${preflight?.totals.errors ?? 0} errors / ${
            preflight?.totals.activeWarnings ?? 0
          } warnings`}
        />
        <HealthPill
          label="Output"
          status={
            outputConnectionState === "connected"
              ? "healthy"
              : outputConnectionState
                ? "error"
                : "unknown"
          }
          detail={outputConnectionState ?? "unknown"}
        />
        <HealthPill
          label="OBS"
          status={obsHealth?.status ?? "unknown"}
          detail={obsHealth?.messages[0] ?? "not checked"}
        />
        <HealthPill
          label="Overlay"
          status={overlayHealth?.status ?? "unknown"}
          detail={overlayHealth?.messages[0] ?? "not checked"}
        />
      </div>
    </section>
  );
}

function HealthPill({
  label,
  status,
  detail,
}: {
  label: string;
  status: HealthStatus["status"];
  detail: string;
}) {
  return (
    <div className={`health-pill ${status}`}>
      {status === "healthy" ? (
        <CheckCircle2 size={15} />
      ) : (
        <AlertTriangle size={15} />
      )}
      <span>{label}</span>
      <small>{detail}</small>
    </div>
  );
}

function PreflightPanel({
  state,
  onOverride,
  onRefreshPreview,
  onSelectCue,
}: {
  state: PreflightState | null;
  onOverride: (item: PreflightItem) => void;
  onRefreshPreview: () => void;
  onSelectCue: (cueId: string) => void;
}) {
  if (!state) return <p className="muted">Preflight has not run.</p>;
  const groups = [
    ["Errors", state.groups.errors],
    ["Warnings", state.groups.warnings],
    ["Info", state.groups.infos],
  ] as const;
  if (
    state.groups.errors.length === 0 &&
    state.groups.warnings.length === 0 &&
    state.groups.infos.length === 0
  ) {
    return <p className="muted">No preflight items.</p>;
  }
  return (
    <div className="preflight-panel">
      <div className="preflight-summary">
        <span>{state.totals.errors} errors</span>
        <span>{state.totals.activeWarnings} active warnings</span>
        <span>{state.totals.overriddenWarnings} overridden</span>
      </div>
      {groups.map(([label, items]) =>
        items.length ? (
          <section className="preflight-group" key={label}>
            <h3>{label}</h3>
            <ul className="preflight-list">
              {items.map((item) => (
                <li
                  className={item.overridden ? "overridden" : ""}
                  key={`${item.ruleId}-${item.scopeLabel}-${item.message}`}
                >
                  <AlertTriangle size={15} />
                  <div>
                    <strong>{item.message}</strong>
                    <span>
                      {item.scopeLabel ?? "Scope"} ·{" "}
                      {item.sourceLabel ?? originLabel(item.source)} ·{" "}
                      {item.ruleId}
                    </span>
                    <div className="preflight-actions">
                      {item.fixActions?.map((action) => (
                        <button
                          key={fixActionKey(action)}
                          type="button"
                          onClick={() =>
                            handleFixAction(
                              action,
                              onRefreshPreview,
                              onSelectCue,
                            )
                          }
                        >
                          {fixActionLabel(action)}
                        </button>
                      ))}
                      {item.severity === "warning" &&
                      item.warningKey &&
                      item.valueHash &&
                      !item.overridden ? (
                        <button type="button" onClick={() => onOverride(item)}>
                          Override
                        </button>
                      ) : null}
                      {item.overridden ? <span>overridden</span> : null}
                    </div>
                  </div>
                </li>
              ))}
            </ul>
          </section>
        ) : null,
      )}
    </div>
  );
}

function ResolvedData({ data }: { data: Record<string, unknown> }) {
  return (
    <dl className="resolved-data">
      {Object.entries(data).map(([key, value]) => (
        <div key={key}>
          <dt>{key.split(".").at(-1)}</dt>
          <dd>{String(value)}</dd>
        </div>
      ))}
    </dl>
  );
}

function TemplateLibrary({
  definitions,
  selectedId,
  disabled,
  onSelect,
  onAdd,
}: {
  definitions: TemplateDefinition[];
  selectedId: string;
  disabled: boolean;
  onSelect: (id: string) => void;
  onAdd: (definition: TemplateDefinition) => void;
}) {
  return (
    <div className="template-library">
      {definitions.map((definition) => (
        <div
          className={
            definition.id === selectedId
              ? "template-row selected"
              : "template-row"
          }
          key={definition.id}
        >
          <button
            className="template-main"
            type="button"
            onClick={() => onSelect(definition.id)}
          >
            <Layers size={16} />
            <span>
              <strong>{definition.name}</strong>
              <small>
                {templateKindLabel(definition.kind)} · v{definition.version} ·{" "}
                {originLabel(definition.origin)}
              </small>
            </span>
          </button>
          <button
            aria-label={`Add ${definition.name}`}
            disabled={disabled}
            type="button"
            onClick={() => onAdd(definition)}
          >
            <Plus size={15} />
          </button>
        </div>
      ))}
    </div>
  );
}

function StageTemplatePanel({
  show,
  definitions,
  instances,
  selectedInstanceId,
  draft,
  disabled,
  onSelect,
  onDraftChange,
  onSave,
  onDelete,
}: {
  show: ShowDefinition | null;
  definitions: TemplateDefinition[];
  instances: TemplateInstance[];
  selectedInstanceId: string;
  draft: TemplateInstance | null;
  disabled: boolean;
  onSelect: (id: string) => void;
  onDraftChange: (draft: TemplateInstance) => void;
  onSave: () => void;
  onDelete: (id: string) => void;
}) {
  const definition = draft
    ? definitions.find((item) => item.id === draft.templateDefinitionId)
    : null;

  return (
    <div className="stage-template-panel">
      <div className="stage-preview">
        <div className="stage-box">
          {instances.map((instance) => (
            <button
              className={
                instance.id === selectedInstanceId
                  ? "stage-instance selected"
                  : "stage-instance"
              }
              key={instance.id}
              style={stageStyle(instance, show)}
              type="button"
              onClick={() => onSelect(instance.id)}
            >
              {instance.name}
            </button>
          ))}
        </div>
      </div>

      {draft && definition ? (
        <div className="inspector">
          <label>
            Instance
            <input
              value={draft.name}
              onChange={(event) =>
                onDraftChange({ ...draft, name: event.currentTarget.value })
              }
            />
          </label>
          <label>
            Variant
            <select
              value={draft.variantId}
              onChange={(event) =>
                onDraftChange({
                  ...draft,
                  variantId: event.currentTarget.value,
                })
              }
            >
              {definition.variants.map((variant) => (
                <option key={variant.id} value={variant.id}>
                  {variant.name}
                </option>
              ))}
            </select>
          </label>
          <div className="numeric-grid">
            {(["x", "y", "width", "height"] as const).map((field) => (
              <label key={field}>
                {field}
                <input
                  min={field === "width" || field === "height" ? 1 : 0}
                  type="number"
                  value={draft.frame[field]}
                  onChange={(event) =>
                    onDraftChange({
                      ...draft,
                      frame: {
                        ...draft.frame,
                        [field]: Number(event.currentTarget.value),
                      },
                    })
                  }
                />
              </label>
            ))}
            <label>
              z
              <input
                type="number"
                value={draft.zIndex}
                onChange={(event) =>
                  onDraftChange({
                    ...draft,
                    zIndex: Number(event.currentTarget.value),
                  })
                }
              />
            </label>
          </div>
          <div className="slot-bindings">
            <strong>Slot bindings</strong>
            {definition.localSlots.map((slot) => (
              <SlotBindingEditor
                draft={draft}
                key={slot.key}
                slot={slot}
                onDraftChange={onDraftChange}
              />
            ))}
          </div>
          <div className="panel-actions">
            <button type="button" disabled={disabled} onClick={onSave}>
              <Save size={16} />
              Save
            </button>
            <button
              type="button"
              disabled={disabled}
              onClick={() => onDelete(draft.id)}
            >
              <Trash2 size={16} />
              Delete
            </button>
          </div>
        </div>
      ) : (
        <p className="muted">Add or select a template instance.</p>
      )}
    </div>
  );
}

function SlotBindingEditor({
  draft,
  slot,
  onDraftChange,
}: {
  draft: TemplateInstance;
  slot: LocalSlot;
  onDraftChange: (draft: TemplateInstance) => void;
}) {
  const binding = draft.slotBindings[slot.key] ?? {
    entityType: "people" as EntityType,
    field: "displayName",
  };
  return (
    <div className="slot-row">
      <span>
        {slot.label}
        {slot.required ? " *" : ""}
      </span>
      <select
        value={binding.entityType}
        onChange={(event) =>
          onDraftChange({
            ...draft,
            slotBindings: {
              ...draft.slotBindings,
              [slot.key]: {
                entityType: event.currentTarget.value as EntityType,
                field: firstFieldFor(event.currentTarget.value as EntityType),
              },
            },
          })
        }
      >
        <option value="people">People</option>
        <option value="sessions">Sessions</option>
        <option value="sponsors">Sponsors</option>
      </select>
      <select
        value={binding.field}
        onChange={(event) =>
          onDraftChange({
            ...draft,
            slotBindings: {
              ...draft.slotBindings,
              [slot.key]: { ...binding, field: event.currentTarget.value },
            },
          })
        }
      >
        {fieldsFor(binding.entityType).map((field) => (
          <option key={field} value={field}>
            {field}
          </option>
        ))}
      </select>
    </div>
  );
}

function EntityEditor({
  entityType,
  entities,
  draft,
  onEntityTypeChange,
  onDraftChange,
  onSelect,
  onNew,
  disabled,
  onSave,
  onDelete,
}: {
  entityType: EntityType;
  entities: TypedEntity[];
  draft: EntityDraft;
  onEntityTypeChange: (entityType: EntityType) => void;
  onDraftChange: (draft: EntityDraft) => void;
  onSelect: (entity: TypedEntity) => void;
  onNew: () => void;
  disabled: boolean;
  onSave: () => void;
  onDelete: (id: string) => void;
}) {
  return (
    <div className="entity-editor">
      <div className="segmented" role="group" aria-label="Entity type">
        {(["people", "sessions", "sponsors"] as const).map((type) => (
          <button
            className={entityType === type ? "selected" : ""}
            key={type}
            type="button"
            onClick={() => onEntityTypeChange(type)}
          >
            {entityLabel(type)}
          </button>
        ))}
      </div>
      <div className="entity-list">
        {entities.map((entity) => (
          <button
            key={entity.id}
            type="button"
            onClick={() => onSelect(entity)}
          >
            <strong>{entityTitle(entityType, entity)}</strong>
            <span>{originLabel(entity.origin)}</span>
          </button>
        ))}
      </div>
      <div className="entity-form">
        <label>
          ID
          <input
            value={draft.id}
            onChange={(event) =>
              onDraftChange({ ...draft, id: event.currentTarget.value })
            }
          />
        </label>
        {entityType === "people" ? (
          <>
            <TextInput
              draft={draft}
              field="displayName"
              label="Display name"
              onDraftChange={onDraftChange}
            />
            <TextInput
              draft={draft}
              field="role"
              label="Role"
              onDraftChange={onDraftChange}
            />
            <TextInput
              draft={draft}
              field="organization"
              label="Organization"
              onDraftChange={onDraftChange}
            />
            <TextInput
              draft={draft}
              field="photo"
              label="Photo asset"
              onDraftChange={onDraftChange}
            />
          </>
        ) : null}
        {entityType === "sessions" ? (
          <>
            <TextInput
              draft={draft}
              field="title"
              label="Title"
              onDraftChange={onDraftChange}
            />
            <TextInput
              draft={draft}
              field="track"
              label="Track"
              onDraftChange={onDraftChange}
            />
            <TextInput
              draft={draft}
              field="startTime"
              label="Start"
              onDraftChange={onDraftChange}
            />
            <TextInput
              draft={draft}
              field="speakerRefs"
              label="Speaker IDs"
              onDraftChange={onDraftChange}
            />
          </>
        ) : null}
        {entityType === "sponsors" ? (
          <>
            <TextInput
              draft={draft}
              field="name"
              label="Name"
              onDraftChange={onDraftChange}
            />
            <TextInput
              draft={draft}
              field="tier"
              label="Tier"
              onDraftChange={onDraftChange}
            />
            <TextInput
              draft={draft}
              field="logo"
              label="Logo asset"
              onDraftChange={onDraftChange}
            />
          </>
        ) : null}
      </div>
      <div className="panel-actions">
        <button type="button" onClick={onNew}>
          <Plus size={16} />
          New
        </button>
        <button type="button" disabled={disabled} onClick={onSave}>
          <Save size={16} />
          Save
        </button>
        <button
          type="button"
          disabled={disabled || !draft.id}
          onClick={() => onDelete(draft.id)}
        >
          <Trash2 size={16} />
          Delete
        </button>
      </div>
    </div>
  );
}

function TextInput({
  draft,
  field,
  label,
  onDraftChange,
}: {
  draft: EntityDraft;
  field: keyof EntityDraft;
  label: string;
  onDraftChange: (draft: EntityDraft) => void;
}) {
  return (
    <label>
      {label}
      <input
        value={draft[field]}
        onChange={(event) =>
          onDraftChange({ ...draft, [field]: event.currentTarget.value })
        }
      />
    </label>
  );
}

function DataImportPanel({
  entityType,
  format,
  mode,
  sourceName,
  content,
  preview,
  mappings,
  report,
  canImport,
  isImporting,
  onEntityTypeChange,
  onFormatChange,
  onModeChange,
  onSourceNameChange,
  onContentChange,
  onMappingChange,
  onImport,
}: {
  entityType: EntityType;
  format: DelimitedFormat;
  mode: ImportMode;
  sourceName: string;
  content: string;
  preview: ReturnType<typeof buildImportPreview>;
  mappings: ColumnMapping[];
  report: DataImportReport | null;
  canImport: boolean;
  isImporting: boolean;
  onEntityTypeChange: (entityType: EntityType) => void;
  onFormatChange: (format: DelimitedFormat) => void;
  onModeChange: (mode: ImportMode) => void;
  onSourceNameChange: (sourceName: string) => void;
  onContentChange: (content: string) => void;
  onMappingChange: (source: string, target: string) => void;
  onImport: () => void;
}) {
  return (
    <div className="import-panel">
      <div className="field-grid">
        <label>
          Entity
          <select
            value={entityType}
            onChange={(event) =>
              onEntityTypeChange(event.currentTarget.value as EntityType)
            }
          >
            <option value="people">People</option>
            <option value="sessions">Sessions</option>
            <option value="sponsors">Sponsors</option>
          </select>
        </label>
        <label>
          Source
          <input
            value={sourceName}
            onChange={(event) => onSourceNameChange(event.currentTarget.value)}
          />
        </label>
      </div>

      <div className="segmented" role="group" aria-label="Import format">
        <button
          className={format === "csv" ? "selected" : ""}
          type="button"
          onClick={() => onFormatChange("csv")}
        >
          CSV
        </button>
        <button
          className={format === "tsv" ? "selected" : ""}
          type="button"
          onClick={() => onFormatChange("tsv")}
        >
          TSV
        </button>
      </div>
      <div className="segmented" role="group" aria-label="Import mode">
        {(["replace", "append", "merge"] as const).map((option) => (
          <button
            className={mode === option ? "selected" : ""}
            key={option}
            type="button"
            onClick={() => onModeChange(option)}
          >
            {option}
          </button>
        ))}
      </div>

      <label className="content-field">
        Content
        <textarea
          value={content}
          spellCheck={false}
          onChange={(event) => onContentChange(event.currentTarget.value)}
        />
      </label>

      <div className="mapping-preview">
        <strong>Mappings</strong>
        {preview.headers.length === 0 ? (
          <p className="muted">No header row.</p>
        ) : (
          <div className="mapping-grid">
            {preview.headers.map((header) => (
              <label key={header}>
                {header}
                <select
                  value={
                    mappings.find((mapping) => mapping.source === header)
                      ?.target ?? ""
                  }
                  onChange={(event) =>
                    onMappingChange(header, event.currentTarget.value)
                  }
                >
                  <option value="">Ignore</option>
                  {fieldsFor(entityType).map((field) => (
                    <option key={field} value={field}>
                      {field}
                    </option>
                  ))}
                </select>
              </label>
            ))}
          </div>
        )}
        {!mappings.some(
          (mapping) => mapping.target === preview.requiredTarget,
        ) ? (
          <p className="warning">
            Missing required field: {preview.requiredTarget}
          </p>
        ) : null}
      </div>

      {preview.rows.length ? (
        <table className="sample-table">
          <thead>
            <tr>
              {preview.headers.map((header) => (
                <th key={header}>{header}</th>
              ))}
            </tr>
          </thead>
          <tbody>
            {preview.rows.map((row, rowIndex) => (
              <tr key={`${row.join("-")}-${rowIndex}`}>
                {preview.headers.map((header, index) => (
                  <td key={`${header}-${index}`}>{row[index] ?? ""}</td>
                ))}
              </tr>
            ))}
          </tbody>
        </table>
      ) : null}

      {report ? (
        <div className="import-report">
          <span>{report.importedRows} imported</span>
          <span>{report.rejectedRows} rejected</span>
          <span>{report.duplicateRows} duplicates</span>
          <span>{report.conflictedRows} conflicts</span>
          {report.issues.map((issue) => (
            <p key={`${issue.rowIndex}-${issue.field}-${issue.message}`}>
              Row {issue.rowIndex || "?"}: {issue.field} - {issue.message}
            </p>
          ))}
        </div>
      ) : null}

      <div className="import-actions">
        <button type="button" disabled={!canImport} onClick={onImport}>
          <Upload size={16} />
          {isImporting ? "Importing" : "Apply Import"}
        </button>
      </div>
    </div>
  );
}

function CueEditor({
  draft,
  cues,
  entities,
  templateInstances,
  templateById,
  disabled,
  onDraftChange,
  onSave,
  onDelete,
  onMoveUp,
  onMoveDown,
  onNew,
  onBulkGenerate,
}: {
  draft: CueDraft;
  cues: Cue[];
  entities: TypedEntity[];
  templateInstances: TemplateInstance[];
  templateById: Map<string, TemplateDefinition>;
  disabled: boolean;
  onDraftChange: (draft: CueDraft) => void;
  onSave: () => void;
  onDelete: () => void;
  onMoveUp: () => void;
  onMoveDown: () => void;
  onNew: () => void;
  onBulkGenerate: () => void;
}) {
  return (
    <div className="cue-editor">
      <div className="field-grid">
        <label>
          Number
          <input
            value={draft.number}
            onChange={(event) =>
              onDraftChange({ ...draft, number: event.currentTarget.value })
            }
          />
        </label>
        <label>
          Segment
          <input
            value={draft.segment}
            onChange={(event) =>
              onDraftChange({ ...draft, segment: event.currentTarget.value })
            }
          />
        </label>
      </div>
      <label>
        Name
        <input
          value={draft.name}
          onChange={(event) =>
            onDraftChange({ ...draft, name: event.currentTarget.value })
          }
        />
      </label>
      <div className="field-grid">
        <label>
          Entity type
          <select
            value={draft.entityType}
            onChange={(event) =>
              onDraftChange({
                ...draft,
                entityType: event.currentTarget.value as EntityType,
                entityId: "",
              })
            }
          >
            <option value="people">People</option>
            <option value="sessions">Sessions</option>
            <option value="sponsors">Sponsors</option>
          </select>
        </label>
        <label>
          Entity
          <select
            value={draft.entityId}
            onChange={(event) =>
              onDraftChange({ ...draft, entityId: event.currentTarget.value })
            }
          >
            <option value="">None</option>
            {entities.map((entity) => (
              <option key={entity.id} value={entity.id}>
                {entityTitle(draft.entityType, entity)}
              </option>
            ))}
          </select>
        </label>
      </div>
      <label>
        Template instance
        <select
          value={draft.templateInstanceId}
          onChange={(event) =>
            onDraftChange({
              ...draft,
              templateInstanceId: event.currentTarget.value,
            })
          }
        >
          {templateInstances.map((instance) => {
            const definition = templateById.get(instance.templateDefinitionId);
            return (
              <option key={instance.id} value={instance.id}>
                {instance.name} ({definition?.name ?? "missing definition"})
              </option>
            );
          })}
        </select>
      </label>
      <label>
        Operator notes
        <textarea
          value={draft.operatorNotes}
          onChange={(event) =>
            onDraftChange({
              ...draft,
              operatorNotes: event.currentTarget.value,
            })
          }
        />
      </label>
      <div className="panel-actions">
        <button type="button" onClick={onNew}>
          <Plus size={16} />
          New
        </button>
        <button type="button" disabled={disabled} onClick={onSave}>
          <Save size={16} />
          Save
        </button>
        <button
          type="button"
          disabled={disabled || !draft.id}
          onClick={onDelete}
        >
          <Trash2 size={16} />
          Delete
        </button>
        <button
          type="button"
          disabled={disabled || cues.length < 2}
          onClick={onMoveUp}
        >
          Up
        </button>
        <button
          type="button"
          disabled={disabled || cues.length < 2}
          onClick={onMoveDown}
        >
          Down
        </button>
        <button type="button" disabled={disabled} onClick={onBulkGenerate}>
          <Wand2 size={16} />
          Bulk
        </button>
      </div>
    </div>
  );
}

function emptyEntityDraft(): EntityDraft {
  return {
    id: "",
    displayName: "",
    role: "",
    organization: "",
    photo: "",
    title: "",
    track: "",
    startTime: "",
    speakerRefs: "",
    name: "",
    logo: "",
    tier: "",
  };
}

function emptyCueDraft(): CueDraft {
  return {
    id: "",
    number: "001",
    name: "",
    segment: "",
    entityType: "people",
    entityId: "",
    templateInstanceId: "",
    operatorNotes: "",
  };
}

function localOrigin(): Origin {
  return { kind: "user", user_id: "local-user" };
}

function entityList(show: ShowDefinition | null, entityType: EntityType) {
  if (!show) return [];
  if (entityType === "people") return show.typedEntities.people;
  if (entityType === "sessions") return show.typedEntities.sessions;
  return show.typedEntities.sponsors;
}

function draftFromEntity(
  entityType: EntityType,
  entity: TypedEntity,
): EntityDraft {
  const draft = emptyEntityDraft();
  draft.id = entity.id;
  if (entityType === "people") {
    const person = entity as Person;
    draft.displayName = person.displayName;
    draft.role = person.role ?? "";
    draft.organization = person.organization ?? "";
    draft.photo = person.photo ?? "";
  } else if (entityType === "sessions") {
    const session = entity as Session;
    draft.title = session.title;
    draft.track = session.track ?? "";
    draft.startTime = session.startTime ?? "";
    draft.speakerRefs = session.speakerRefs?.join(", ") ?? "";
  } else {
    const sponsor = entity as Sponsor;
    draft.name = sponsor.name;
    draft.logo = sponsor.logo ?? "";
    draft.tier = sponsor.tier ?? "";
  }
  return draft;
}

function buildEntity(entityType: EntityType, draft: EntityDraft): TypedEntity {
  const id = draft.id.trim() || slug(entityTitleFromDraft(entityType, draft));
  if (entityType === "people") {
    return {
      id,
      displayName: draft.displayName.trim(),
      role: optional(draft.role),
      organization: optional(draft.organization),
      photo: optional(draft.photo),
      origin: localOrigin(),
    };
  }
  if (entityType === "sessions") {
    return {
      id,
      title: draft.title.trim(),
      track: optional(draft.track),
      startTime: optional(draft.startTime),
      speakerRefs: draft.speakerRefs
        .split(",")
        .map((value) => value.trim())
        .filter(Boolean),
      origin: localOrigin(),
    };
  }
  return {
    id,
    name: draft.name.trim(),
    logo: optional(draft.logo),
    tier: optional(draft.tier),
    origin: localOrigin(),
  };
}

function buildCue(draft: CueDraft, cueType: TemplateKind): Cue {
  const cueId = draft.id.trim() || `cue-${Date.now()}`;
  return {
    id: cueId,
    number: draft.number.trim() || "001",
    segment: optional(draft.segment),
    name: draft.name.trim() || `Cue ${draft.number}`,
    mode: "absolute",
    cueType,
    entityRefs: draft.entityId
      ? {
          [entityRefKey(draft.entityType)]: {
            entityType: draft.entityType,
            entityId: draft.entityId,
          },
        }
      : {},
    slotOverrides: {},
    templateStates: draft.templateInstanceId
      ? [{ templateInstanceId: draft.templateInstanceId, visible: true }]
      : [],
    transition: { kind: "cut" },
    operatorNotes: optional(draft.operatorNotes),
    origin: localOrigin(),
  };
}

function defaultTemplateInstance(
  definition: TemplateDefinition,
): TemplateInstance {
  return {
    id: `instance-${slug(definition.name)}-${Date.now()}`,
    name: definition.name,
    templateDefinitionId: definition.id,
    templateVersionPolicy: "latestCompatible",
    frame: defaultFrame(definition.kind),
    zIndex: 10,
    variantId: definition.variants[0]?.id ?? "default",
    slotBindings: Object.fromEntries(
      definition.localSlots.map((slot) => [
        slot.key,
        defaultBinding(definition.kind, slot.key),
      ]),
    ),
    visibleByDefault: false,
    origin: localOrigin(),
  };
}

function defaultFrame(kind: TemplateKind) {
  if (kind === "sponsorBug") return { x: 1450, y: 70, width: 360, height: 180 };
  if (kind === "breakScreen") return { x: 0, y: 0, width: 1920, height: 1080 };
  if (kind === "sessionTitle")
    return { x: 180, y: 360, width: 1560, height: 280 };
  if (kind === "speakerCard")
    return { x: 260, y: 230, width: 1400, height: 560 };
  return { x: 120, y: 760, width: 900, height: 184 };
}

function defaultBinding(kind: TemplateKind, slotKey: string) {
  if (kind === "sessionTitle") {
    return {
      entityType: "sessions" as EntityType,
      field:
        slotKey === "time"
          ? "startTime"
          : slotKey === "subtitle"
            ? "track"
            : "title",
    };
  }
  if (kind === "sponsorBug") {
    return {
      entityType: "sponsors" as EntityType,
      field: slotKey === "logo" ? "logo" : slotKey === "tier" ? "tier" : "name",
    };
  }
  return {
    entityType: "people" as EntityType,
    field:
      slotKey === "portrait" || slotKey === "photo"
        ? "photo"
        : slotKey === "role"
          ? "role"
          : slotKey === "organization"
            ? "organization"
            : "displayName",
  };
}

function stageStyle(
  instance: TemplateInstance,
  show: ShowDefinition | null,
): CSSProperties {
  const width = show?.stage.width ?? 1920;
  const height = show?.stage.height ?? 1080;
  return {
    left: `${(instance.frame.x / width) * 100}%`,
    top: `${(instance.frame.y / height) * 100}%`,
    width: `${(instance.frame.width / width) * 100}%`,
    height: `${(instance.frame.height / height) * 100}%`,
    zIndex: instance.zIndex,
  };
}

function fieldsFor(entityType: EntityType) {
  if (entityType === "people")
    return ["id", "displayName", "role", "organization", "photo"];
  if (entityType === "sessions")
    return ["id", "title", "track", "startTime", "speakerRefs"];
  return ["id", "name", "logo", "tier"];
}

function firstFieldFor(entityType: EntityType) {
  if (entityType === "people") return "displayName";
  if (entityType === "sessions") return "title";
  return "name";
}

function entityLabel(entityType: EntityType) {
  if (entityType === "people") return "People";
  if (entityType === "sessions") return "Sessions";
  return "Sponsors";
}

function entityTitle(entityType: EntityType, entity: TypedEntity) {
  if (entityType === "people") return (entity as Person).displayName;
  if (entityType === "sessions") return (entity as Session).title;
  return (entity as Sponsor).name;
}

function entityTitleFromDraft(entityType: EntityType, draft: EntityDraft) {
  if (entityType === "people") return draft.displayName;
  if (entityType === "sessions") return draft.title;
  return draft.name;
}

function entityRefKey(entityType: EntityType) {
  if (entityType === "people") return "person";
  if (entityType === "sessions") return "session";
  return "sponsor";
}

function templateKindLabel(kind: TemplateKind) {
  return kind.replace(/[A-Z]/g, (match) => ` ${match}`).trim();
}

function liveStateLabel(state: LiveState) {
  return state.replace(/[A-Z]/g, (match) => ` ${match}`).trim();
}

function preflightStatus(state: PreflightState | null): HealthStatus["status"] {
  if (!state) return "unknown";
  if (state.totals.errors > 0) return "error";
  if (state.totals.activeWarnings > 0) return "warning";
  return "healthy";
}

function preflightFromPreview(
  preview: PreviewSnapshot | null,
): PreflightState | null {
  if (!preview) return null;
  const errors = preview.preflightResult.items.filter(
    (item) => item.severity === "error",
  );
  const warnings = preview.preflightResult.items.filter(
    (item) => item.severity === "warning",
  );
  const infos = preview.preflightResult.items.filter(
    (item) => item.severity === "info",
  );
  return {
    showDefinitionId: "",
    runSessionId: "",
    runtimeRevision: preview.previewRevision,
    groups: { errors, warnings, infos },
    totals: {
      errors: errors.length,
      warnings: warnings.length,
      infos: infos.length,
      activeWarnings: warnings.filter((item) => !item.overridden).length,
      overriddenWarnings: warnings.filter((item) => item.overridden).length,
    },
  };
}

function fixActionKey(action: FixAction) {
  return JSON.stringify(action);
}

function fixActionLabel(action: FixAction) {
  if (action.kind === "refreshPreview") return "Refresh Preview";
  if (action.kind === "selectCue") return "Select Cue";
  return "Resolve Asset";
}

function handleFixAction(
  action: FixAction,
  onRefreshPreview: () => void,
  onSelectCue: (cueId: string) => void,
) {
  if (action.kind === "refreshPreview") onRefreshPreview();
  if (action.kind === "selectCue") onSelectCue(action.cueId);
}

function originLabel(origin?: Origin) {
  if (!origin) return "unknown";
  if (origin.kind === "system") return "system";
  if (origin.kind === "import") return `import:${origin.source}`;
  if (origin.kind === "plugin")
    return `plugin:${origin.pluginId ?? origin.plugin_id ?? "unknown"}`;
  if (origin.kind === "migration")
    return `migration:${origin.migrationId ?? origin.migration_id ?? "unknown"}`;
  return "user";
}

function optional(value: string) {
  const trimmed = value.trim();
  return trimmed.length ? trimmed : undefined;
}

function slug(value: string) {
  return (
    value
      .trim()
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/(^-|-$)/g, "") || "item"
  );
}
