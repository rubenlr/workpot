/// <reference types="vitest/config" />
import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { svelteTesting } from "@testing-library/svelte/vite";
import { defineConfig } from "vite";
import { nonTestableCoverageGlobs } from "./scripts/coverage-exclusions.mjs";

const host = process.env.TAURI_DEV_HOST;
const isCi = process.env.CI === "true" || process.env.CI === "1";
const isVitest = Boolean(process.env.VITEST);

export default defineConfig({
  plugins: [tailwindcss(), sveltekit(), ...(isVitest ? [svelteTesting()] : [])],
  resolve: isVitest ? { conditions: ["browser"] } : undefined,
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
  test: {
    environment: "jsdom",
    coverage: {
      provider: "v8",
      reporter: ["lcov"],
      reportsDirectory: "coverage",
      include: ["src/**/*.{ts,svelte}"],
      exclude: [
        "**/*.test.ts",
        "**/*.svelte.test.ts",
        ...nonTestableCoverageGlobs,
      ],
    },
  },
});
