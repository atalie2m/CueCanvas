/// <reference types="vitest" />
import { defineConfig } from "vitest/config";

export default defineConfig({
  test: {
    environment: "happy-dom",
  },
  server: {
    proxy: {
      "/api": "http://127.0.0.1:4317",
      "/ws": {
        target: "ws://127.0.0.1:4317",
        ws: true,
      },
    },
  },
});
