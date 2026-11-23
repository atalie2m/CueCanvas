import { describe, expect, it } from "vitest";
import { snapshotDiff } from "./diff";

describe("snapshotDiff", () => {
  it("shows only changed resolved values", () => {
    const rows = snapshotDiff(
      {
        id: "program-1",
        sourceCueId: "cue-001",
        sourcePreviewSnapshotId: "preview-1",
        programRevision: 1,
        resolvedData: {
          "instance.primaryText": "Cat",
          "instance.secondaryText": "CTO",
        },
      },
      {
        id: "preview-2",
        sourceCueId: "cue-002",
        previewRevision: 2,
        resolvedData: {
          "instance.primaryText": "Dog",
          "instance.secondaryText": "CTO",
        },
        preflightResult: { items: [] },
      },
    );

    expect(rows).toEqual([
      {
        category: "Resolved data",
        key: "instance.primaryText",
        program: "Cat",
        preview: "Dog",
      },
      {
        category: "Output state",
        key: "Source cue",
        program: "cue-001",
        preview: "cue-002",
      },
    ]);
  });
});
