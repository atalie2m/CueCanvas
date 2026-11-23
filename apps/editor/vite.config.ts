import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";

export default defineConfig({
  base: "/editor/",
  plugins: [react()],
  server: {
    proxy: {
      "/api": "http://127.0.0.1:4317",
      "/overlay": "http://127.0.0.1:4317",
      "/ws": {
        target: "ws://127.0.0.1:4317",
        ws: true,
      },
    },
  },
});
