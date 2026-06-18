import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig } from "vite";
import { applyEsbuildTarget } from "./vite-esbuild-target";

const host = process.env.TAURI_DEV_HOST;
const isCi = process.env.CI === "true" || process.env.CI === "1";

export default defineConfig(
  applyEsbuildTarget({
    plugins: [tailwindcss(), sveltekit()],
    // Tauri WKWebView on macOS 12+ — see vite-esbuild-target.ts.
    clearScreen: false,
    logLevel: isCi ? "warn" : "info",
    server: {
      port: 1420,
      strictPort: true,
      host: host || false,
      hmr: host
        ? {
            protocol: "ws",
            host,
            port: 1421,
          }
        : undefined,
      watch: {
        ignored: ["**/src-tauri/**"],
      },
    },
  }),
);
