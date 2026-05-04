import { renderToStaticMarkup } from "react-dom/server";
import { describe, expect, it, vi } from "vitest";
import { ObsSetupPanel } from "./App";
import type {
  ObsConnectionRequest,
  ObsDesiredState,
  ObsSetupResponse,
} from "./types";

describe("ObsSetupPanel", () => {
  it("renders setup controls, target state, and verification wording", () => {
    const connection: ObsConnectionRequest = {
      host: "127.0.0.1",
      port: 4455,
      password: "",
      mock: true,
    };
    const desired: ObsDesiredState = {
      sceneName: "CueCanvas Graphics",
      sourceName: "CueCanvas Program",
      url: "http://127.0.0.1:4317/overlay/program?token=test-overlay-token",
      stage: { width: 1920, height: 1080 },
      shutdownWhenNotVisible: false,
      refreshWhenActive: false,
      customCss:
        "html, body { margin: 0; background: transparent; overflow: hidden; }",
    };
    const setup: ObsSetupResponse = {
      connection: {
        connected: true,
        host: "127.0.0.1",
        port: 4455,
        obsVersion: "mock-obs",
        websocketVersion: "mock-websocket",
        message: "Mock OBS connected",
      },
      desired,
      observed: {
        sceneExists: true,
        sourceExists: true,
        url: desired.url,
        width: 1920,
        height: 1080,
        shutdownWhenNotVisible: false,
        refreshWhenActive: false,
        customCss: desired.customCss,
        visibleInScenePath: true,
        overlayConnected: true,
      },
      issues: [],
      actions: ["Created Browser Source 'CueCanvas Program'"],
    };

    const html = renderToStaticMarkup(
      <ObsSetupPanel
        connection={connection}
        desired={desired}
        setup={setup}
        onApply={vi.fn()}
        onConnect={vi.fn()}
        onConnectionChange={vi.fn()}
        onTestPattern={vi.fn()}
        onVerify={vi.fn()}
      />,
    );

    expect(html).toContain("CueCanvas Graphics");
    expect(html).toContain("CueCanvas Program");
    expect(html).toContain("Connect");
    expect(html).toContain("Apply");
    expect(html).toContain("Verify");
    expect(html).toContain("Test Pattern");
    expect(html).toContain(
      "High-confidence preview; OBS verified when connected.",
    );
  });
});
