import { describe, expect, it } from "vitest";
import {
  buildImportPreview,
  inferColumnMappings,
  parseDelimited,
  sampleContent,
} from "./importMapping";

describe("import mapping helpers", () => {
  it("infers people mappings from common CSV headers", () => {
    expect(
      inferColumnMappings(["name", "role", "org", "photo"], "people"),
    ).toEqual([
      { source: "name", target: "displayName" },
      { source: "role", target: "role" },
      { source: "org", target: "organization" },
      { source: "photo", target: "photo" },
    ]);
  });

  it("parses quoted CSV rows for mapping preview", () => {
    expect(
      parseDelimited('name,role\n"Ada, Lovelace",Keynote\n', "csv"),
    ).toEqual([
      ["name", "role"],
      ["Ada, Lovelace", "Keynote"],
    ]);
  });

  it("reports whether the required target can be mapped", () => {
    const preview = buildImportPreview("role\nHost\n", "csv", "people");

    expect(preview.requiredTarget).toBe("displayName");
    expect(preview.hasRequiredMapping).toBe(false);
  });

  it("generates TSV samples when TSV mode is selected", () => {
    expect(sampleContent("sponsors", "tsv")).toContain("name\ttier\tlogo");
  });
});
