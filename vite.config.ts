import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig, type Plugin } from "vite";

const host = process.env.TAURI_DEV_HOST;
const isCi = process.env.CI === "true" || process.env.CI === "1";

/** Avoid libuv kqueue assert on macOS when Node tears down after adapter-static. */
function exitAfterProductionBuild(): Plugin {
  let isSsrBuild = false;
  return {
    name: "workpot:exit-after-build",
    apply: "build",
    configResolved(config) {
      isSsrBuild = !!config.build.ssr;
    },
    closeBundle: {
      sequential: true,
      async handler() {
        // Adapter runs in SvelteKit's sequential SSR closeBundle; wait for it.
        if (!isSsrBuild) return;
        process.exit(0);
      },
    },
  };
}

export default defineConfig({
  // Registered after sveltekit so sequential closeBundle runs after adapter-static.
  plugins: [tailwindcss(), sveltekit(), exitAfterProductionBuild()],
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
    coverage: {
      provider: "v8",
      reporter: ["lcov"],
      reportsDirectory: "coverage",
      include: ["src/**/*.{ts,svelte}"],
      exclude: ["**/*.test.ts", "**/+layout.ts"],
    },
  },
});
