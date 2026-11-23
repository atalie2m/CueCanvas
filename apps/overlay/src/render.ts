export type OverlayState = {
  protocolVersion: number;
  stage: { width: number; height: number };
  programRevision?: number;
  items: OverlayItem[];
};

export type OverlayItem =
  | {
      kind: "text";
      id: string;
      text: string;
      frame: Frame;
      zIndex: number;
      style: {
        fontFamily: string;
        fontSize: number;
        color: string;
        weight: "regular" | "semibold" | "bold";
      };
    }
  | {
      kind: "image";
      id: string;
      assetId: string;
      frame: Frame;
      zIndex: number;
      fit: "cover" | "contain";
    }
  | {
      kind: "rect";
      id: string;
      frame: Frame;
      zIndex: number;
      style: { fill: string; opacity: number };
    }
  | { kind: "clear" }
  | { kind: "blackout" };

type Frame = { x: number; y: number; width: number; height: number };

const overlayToken =
  typeof window === "undefined"
    ? ""
    : (new URLSearchParams(window.location.search).get("token") ?? "");

export function renderOverlay(root: HTMLElement, state: OverlayState | null) {
  root.replaceChildren();
  if (!state) return;

  root.style.width = `${state.stage.width}px`;
  root.style.height = `${state.stage.height}px`;

  for (const item of state.items) {
    if (item.kind === "clear") continue;
    if (item.kind === "blackout") {
      const blackout = document.createElement("div");
      blackout.className = "blackout";
      root.replaceChildren(blackout);
      return;
    }

    const element = document.createElement(
      item.kind === "image" ? "img" : "div",
    );
    element.className = `overlay-item ${item.kind}`;
    element.dataset.itemId = item.id;
    element.style.left = `${item.frame.x}px`;
    element.style.top = `${item.frame.y}px`;
    element.style.width = `${item.frame.width}px`;
    element.style.height = `${item.frame.height}px`;
    element.style.zIndex = String(item.zIndex);

    if (item.kind === "text") {
      element.textContent = item.text;
      element.style.fontFamily = item.style.fontFamily;
      element.style.fontSize = `${item.style.fontSize}px`;
      element.style.color = item.style.color;
      element.style.fontWeight =
        item.style.weight === "bold"
          ? "800"
          : item.style.weight === "semibold"
            ? "650"
            : "450";
    } else if (item.kind === "image") {
      element.setAttribute("src", assetUrl(item.assetId));
      element.setAttribute("alt", "");
      element.style.objectFit = item.fit;
    } else {
      element.style.background = item.style.fill;
      element.style.opacity = String(
        Math.max(0, Math.min(100, item.style.opacity)) / 100,
      );
    }
    root.appendChild(element);
  }
}

function assetUrl(assetId: string) {
  const tokenQuery = overlayToken
    ? `?token=${encodeURIComponent(overlayToken)}`
    : "";
  return `/assets/${encodeURIComponent(assetId)}${tokenQuery}`;
}
