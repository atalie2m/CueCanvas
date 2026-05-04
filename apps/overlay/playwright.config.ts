import { defineConfig } from "@playwright/test";

export default defineConfig({
  testDir: ".",
  testMatch: "visual.pw.ts",
  outputDir: "../../test-results/overlay-visual",
  use: {
    baseURL: "http://127.0.0.1:5174",
    viewport: { width: 640, height: 360 },
  },
  webServer: {
    command:
      "pnpm --filter @cuecanvas/overlay exec vite --host 127.0.0.1 --port 5174 --strictPort",
    reuseExistingServer: !process.env.CI,
    timeout: 120_000,
    url: "http://127.0.0.1:5174",
  },
});
