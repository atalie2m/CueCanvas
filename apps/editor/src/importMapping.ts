import type { ColumnMapping, DelimitedFormat, EntityType } from "./types";

type Preset = {
  required: string;
  aliases: Record<string, string[]>;
};

const PRESETS: Record<EntityType, Preset> = {
  people: {
    required: "displayName",
    aliases: {
      id: ["id", "person id", "speaker id"],
      displayName: ["name", "display name", "displayname", "speaker", "person"],
      role: ["role", "title", "job title"],
      organization: ["organization", "org", "company", "affiliation"],
      photo: ["photo", "image", "headshot"],
    },
  },
  sessions: {
    required: "title",
    aliases: {
      id: ["id", "session id"],
      title: ["title", "session", "session title", "name"],
      track: ["track", "room"],
      startTime: ["start", "start time", "time"],
      speakerRefs: ["speakers", "speaker refs", "speaker ids"],
    },
  },
  sponsors: {
    required: "name",
    aliases: {
      id: ["id", "sponsor id"],
      name: ["name", "sponsor", "sponsor name"],
      logo: ["logo", "image"],
      tier: ["tier", "level"],
    },
  },
};

export type ImportPreview = {
  headers: string[];
  rows: string[][];
  mappings: ColumnMapping[];
  requiredTarget: string;
  hasRequiredMapping: boolean;
};

export function buildImportPreview(
  content: string,
  format: DelimitedFormat,
  entityType: EntityType,
): ImportPreview {
  const rows = parseDelimited(content, format).filter((row) =>
    row.some((value) => value.trim().length > 0),
  );
  const headers = rows[0] ?? [];
  const mappings = inferColumnMappings(headers, entityType);
  const requiredTarget = PRESETS[entityType].required;

  return {
    headers,
    rows: rows.slice(1, 4),
    mappings,
    requiredTarget,
    hasRequiredMapping: mappings.some(
      (mapping) => mapping.target === requiredTarget,
    ),
  };
}

export function parseDelimited(
  content: string,
  format: DelimitedFormat,
): string[][] {
  const separator = format === "csv" ? "," : "\t";
  const rows: string[][] = [];
  let row: string[] = [];
  let cell = "";
  let quoted = false;

  for (let index = 0; index < content.length; index += 1) {
    const char = content[index];

    if (quoted) {
      if (char === '"' && content[index + 1] === '"') {
        cell += '"';
        index += 1;
      } else if (char === '"') {
        quoted = false;
      } else {
        cell += char;
      }
      continue;
    }

    if (char === '"' && cell.length === 0) {
      quoted = true;
    } else if (char === separator) {
      row.push(cell.trim());
      cell = "";
    } else if (char === "\n" || char === "\r") {
      if (char === "\r" && content[index + 1] === "\n") index += 1;
      row.push(cell.trim());
      rows.push(row);
      row = [];
      cell = "";
    } else {
      cell += char;
    }
  }

  if (cell.length > 0 || row.length > 0) {
    row.push(cell.trim());
    rows.push(row);
  }

  return rows;
}

export function inferColumnMappings(
  headers: string[],
  entityType: EntityType,
): ColumnMapping[] {
  const preset = PRESETS[entityType];
  const claimedTargets = new Set<string>();

  return headers.flatMap((header) => {
    const normalized = normalizeHeader(header);
    const target = Object.entries(preset.aliases).find(([candidate, aliases]) =>
      [candidate, ...aliases].some(
        (alias) => normalizeHeader(alias) === normalized,
      ),
    )?.[0];

    if (!target || claimedTargets.has(target)) return [];
    claimedTargets.add(target);
    return [{ source: header, target }];
  });
}

export function sampleContent(entityType: EntityType, format: DelimitedFormat) {
  if (entityType === "sessions") {
    return joinRows(
      [
        ["title", "track", "startTime"],
        ["Opening Keynote", "Main", "09:00"],
      ],
      format,
    );
  }

  if (entityType === "sponsors") {
    return joinRows(
      [
        ["name", "tier", "logo"],
        ["ACME", "Gold", "asset-acme"],
      ],
      format,
    );
  }

  return joinRows(
    [
      ["name", "role", "organization"],
      ["Imported Speaker", "Host", "CueCanvas"],
    ],
    format,
  );
}

function joinRows(rows: string[][], format: DelimitedFormat) {
  const separator = format === "csv" ? "," : "\t";
  return rows.map((row) => row.join(separator)).join("\n");
}

function normalizeHeader(header: string) {
  return header.trim().toLowerCase().replace(/[_-]+/g, " ");
}
