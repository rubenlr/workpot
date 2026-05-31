import path from "node:path";
import { fileURLToPath } from "node:url";
import type { StorybookConfig } from "@storybook/sveltekit";

const dirname = path.dirname(fileURLToPath(import.meta.url));

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.svelte"],
  addons: [
    "@storybook/addon-svelte-csf",
    "@storybook/addon-a11y",
    "@storybook/addon-docs",
  ],
  framework: "@storybook/sveltekit",
  async viteFinal(viteConfig) {
    viteConfig.resolve ??= {};
    viteConfig.resolve.alias ??= {};
    const alias = viteConfig.resolve.alias;
    if (Array.isArray(alias)) {
      alias.push({
        find: "@tauri-apps/api/core",
        replacement: path.join(dirname, "../src/lib/storybook/tauriCoreMock.ts"),
      });
    } else {
      alias["@tauri-apps/api/core"] = path.join(
        dirname,
        "../src/lib/storybook/tauriCoreMock.ts",
      );
    }
    return viteConfig;
  },
};

export default config;
