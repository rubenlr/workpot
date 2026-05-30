import { sveltekit } from "@sveltejs/kit/vite";
import tailwindcss from "@tailwindcss/vite";
import { defineConfig, type Plugin } from "vite";

const host = process.env.TAURI_DEV_HOST;
const isCi = process.env.CI === "true" || process.env.CI === "1";

/** Avoid libuv kqueue assert on macOS when Node tears down after adapter-static. */
function exitAfterProductionBuild(): Plugin {
  return {
    name: "workpot:exit-after-build",
    apply: "build",
    closeBundle() {
      process.exit(0);
    },
  };
}

export default defineConfig({
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
});
