import { describe, expect, it } from "vitest";
import { renderOverlay } from "./render";

describe("renderOverlay", () => {
  it("renders text items and ignores clear items", () => {
    const root = document.createElement("div");
    renderOverlay(root, {
      protocolVersion: 1,
      stage: { width: 1920, height: 1080 },
      items: [
        { kind: "clear" },
        {
          kind: "text",
          id: "item-1",
          text: "Cat",
          frame: { x: 10, y: 20, width: 300, height: 50 },
          zIndex: 1,
          style: {
            fontFamily: "system-ui",
            fontSize: 48,
            color: "#fff",
            weight: "bold",
          },
        },
      ],
    });

    expect(root.querySelector("[data-item-id='item-1']")?.textContent).toBe(
      "Cat",
    );
    expect(root.children).toHaveLength(1);
  });

  it("renders image items through the runtime asset route", () => {
    const root = document.createElement("div");
    renderOverlay(root, {
      protocolVersion: 1,
      stage: { width: 1920, height: 1080 },
      items: [
        {
          kind: "image",
          id: "image-1",
          assetId: "asset-cat-photo",
          frame: { x: 10, y: 20, width: 128, height: 128 },
          zIndex: 1,
          fit: "cover",
        },
      ],
    });

    const image = root.querySelector("img[data-item-id='image-1']");
    expect(image?.getAttribute("src")).toBe("/assets/asset-cat-photo");
    expect(image?.getAttribute("alt")).toBe("");
  });
});
