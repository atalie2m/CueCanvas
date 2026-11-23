import type { OverlayState } from "./render";

const stage = { width: 640, height: 360 };

export type OverlayFixtureName =
  | "transparent"
  | "cjk-wrapping"
  | "font-fallback"
  | "text-overflow"
  | "image-fit"
  | "clear"
  | "blackout"
  | "reconnect-recovery";

export function overlayFixture(name: string): OverlayState | null {
  switch (name as OverlayFixtureName) {
    case "transparent":
      return {
        protocolVersion: 1,
        stage,
        items: [],
      };
    case "cjk-wrapping":
      return {
        protocolVersion: 1,
        stage,
        items: [
          {
            kind: "text",
            id: "cjk-copy",
            text: "基調講演の開始まであと少しです。東京、서울、上海の参加者を歓迎します。",
            frame: { x: 32, y: 42, width: 250, height: 172 },
            zIndex: 1,
            style: {
              fontFamily:
                '"Hiragino Sans", "Noto Sans CJK JP", system-ui, sans-serif',
              fontSize: 26,
              color: "#ffffff",
              weight: "semibold",
            },
          },
        ],
      };
    case "font-fallback":
      return {
        protocolVersion: 1,
        stage,
        items: [
          {
            kind: "text",
            id: "fallback-copy",
            text: "Fallback font stack",
            frame: { x: 42, y: 58, width: 360, height: 76 },
            zIndex: 1,
            style: {
              fontFamily:
                '"Definitely Missing CueCanvas Font", Inter, system-ui, sans-serif',
              fontSize: 32,
              color: "#f8fafc",
              weight: "bold",
            },
          },
        ],
      };
    case "text-overflow":
      return {
        protocolVersion: 1,
        stage,
        items: [
          {
            kind: "rect",
            id: "overflow-frame",
            frame: { x: 30, y: 48, width: 180, height: 72 },
            zIndex: 0,
            style: { fill: "#1d4ed8", opacity: 100 },
          },
          {
            kind: "text",
            id: "overflow-copy",
            text: "Supercalifragilisticexpialidocious-without-breaking-layout",
            frame: { x: 30, y: 48, width: 180, height: 72 },
            zIndex: 1,
            style: {
              fontFamily: "Inter, system-ui, sans-serif",
              fontSize: 24,
              color: "#ffffff",
              weight: "bold",
            },
          },
        ],
      };
    case "image-fit":
      return {
        protocolVersion: 1,
        stage,
        items: [
          {
            kind: "image",
            id: "fit-image",
            assetId: "fixture-photo",
            frame: { x: 42, y: 38, width: 240, height: 160 },
            zIndex: 1,
            fit: "contain",
          },
        ],
      };
    case "clear":
      return {
        protocolVersion: 1,
        stage,
        items: [{ kind: "clear" }],
      };
    case "blackout":
      return {
        protocolVersion: 1,
        stage,
        items: [{ kind: "blackout" }],
      };
    case "reconnect-recovery":
      return {
        protocolVersion: 1,
        stage,
        programRevision: 3,
        items: [
          {
            kind: "rect",
            id: "reconnect-ready",
            frame: { x: 0, y: 0, width: stage.width, height: stage.height },
            zIndex: 0,
            style: { fill: "#16a34a", opacity: 100 },
          },
          {
            kind: "text",
            id: "reconnect-copy",
            text: "Recovered Program",
            frame: { x: 42, y: 128, width: 420, height: 88 },
            zIndex: 1,
            style: {
              fontFamily: "Inter, system-ui, sans-serif",
              fontSize: 38,
              color: "#ffffff",
              weight: "bold",
            },
          },
        ],
      };
    default:
      return null;
  }
}
