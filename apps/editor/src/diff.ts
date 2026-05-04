import type { ProgramSnapshot, PreviewSnapshot } from "./types";

export type DiffRow = {
  category: string;
  key: string;
  program: string;
  preview: string;
};

export function snapshotDiff(
  program: ProgramSnapshot | null,
  preview: PreviewSnapshot | null,
): DiffRow[] {
  if (!preview) return [];
  return [
    ...resolvedDataDiff(program, preview),
    ...revisionDiff(program, preview),
    ...overlayDiff(program, preview),
    ...originDiff(program, preview),
    ...warningDiff(preview),
  ];
}

function resolvedDataDiff(
  program: ProgramSnapshot | null,
  preview: PreviewSnapshot,
): DiffRow[] {
  const keys = new Set([
    ...Object.keys(program?.resolvedData ?? {}),
    ...Object.keys(preview.resolvedData),
  ]);
  return [...keys].sort().flatMap((key) => {
    const programValue = formatValue(program?.resolvedData[key]);
    const previewValue = formatValue(preview.resolvedData[key]);
    return programValue === previewValue
      ? []
      : [
          {
            category: "Resolved data",
            key,
            program: programValue,
            preview: previewValue,
          },
        ];
  });
}

function revisionDiff(
  program: ProgramSnapshot | null,
  preview: PreviewSnapshot,
): DiffRow[] {
  const rows: DiffRow[] = [];
  if (
    formatValue(program?.showDefinitionRevision) !==
    formatValue(preview.showDefinitionRevision)
  ) {
    rows.push({
      category: "Revision",
      key: "Show definition",
      program: formatValue(program?.showDefinitionRevision),
      preview: formatValue(preview.showDefinitionRevision),
    });
  }
  if ((program?.sourceCueId ?? "not set") !== preview.sourceCueId) {
    rows.push({
      category: "Output state",
      key: "Source cue",
      program: formatValue(program?.sourceCueId),
      preview: preview.sourceCueId,
    });
  }
  return rows;
}

function overlayDiff(
  program: ProgramSnapshot | null,
  preview: PreviewSnapshot,
): DiffRow[] {
  const rows: DiffRow[] = [];
  const programState = program?.overlayState;
  const previewState = preview.overlayState;
  if (!previewState) return rows;
  if (formatValue(program?.outputTargetId) !== "not set") {
    rows.push({
      category: "Output state",
      key: "Output target",
      program: formatValue(program?.outputTargetId),
      preview: "Preview",
    });
  }
  const programItems = overlayItemsByKey(programState);
  const previewItems = overlayItemsByKey(previewState);
  const keys = new Set([...programItems.keys(), ...previewItems.keys()]);
  for (const key of [...keys].sort()) {
    const programValue = programItems.get(key) ?? "hidden";
    const previewValue = previewItems.get(key) ?? "hidden";
    if (programValue !== previewValue) {
      rows.push({
        category: key.includes(".asset") ? "Assets" : "Visibility/style",
        key,
        program: programValue,
        preview: previewValue,
      });
    }
  }
  return rows;
}

function overlayItemsByKey(state: ProgramSnapshot["overlayState"]) {
  const map = new Map<string, string>();
  for (const item of state?.items ?? []) {
    if (item.kind === "clear" || item.kind === "blackout") {
      map.set("program.state", item.kind);
      continue;
    }
    if (item.kind === "text") {
      map.set(
        `${item.templateInstanceId}.${item.slotKey}.text`,
        JSON.stringify({
          text: item.text,
          frame: item.frame,
          zIndex: item.zIndex,
          style: item.style,
        }),
      );
    } else if (item.kind === "image") {
      map.set(
        `${item.templateInstanceId}.${item.slotKey}.asset`,
        JSON.stringify({
          assetId: item.assetId,
          frame: item.frame,
          zIndex: item.zIndex,
          fit: item.fit,
        }),
      );
    }
  }
  return map;
}

function originDiff(
  program: ProgramSnapshot | null,
  preview: PreviewSnapshot,
): DiffRow[] {
  const programOrigins = formatValue(program?.originChain ?? []);
  const previewOrigins = formatValue(preview.originChain ?? []);
  return programOrigins === previewOrigins
    ? []
    : [
        {
          category: "Origin",
          key: "Origin chain",
          program: programOrigins,
          preview: previewOrigins,
        },
      ];
}

function warningDiff(preview: PreviewSnapshot): DiffRow[] {
  return preview.preflightResult.items
    .filter((item) => item.severity === "warning")
    .map((item) => ({
      category: "Warnings",
      key: item.ruleId,
      program: "not active",
      preview: item.message,
    }));
}

function formatValue(value: unknown): string {
  if (value === undefined || value === null) return "not set";
  if (typeof value === "string") return value;
  return JSON.stringify(value);
}
