import path from "node:path";
import { fileURLToPath } from "node:url";
import type { StorybookConfig } from "@storybook/sveltekit";

const dirname = path.dirname(fileURLToPath(import.meta.url));

function storybookAlias(relativePath: string): string {
  return path.join(dirname, relativePath);
}

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
    const aliases = {
      "@tauri-apps/api/core": storybookAlias(
        "../src/lib/tray/storybook/tauriCoreMock.ts",
      ),
      "@tauri-apps/api/event": storybookAlias(
        "../src/lib/tray/storybook/tauriEventMock.ts",
      ),
      "@tauri-apps/api/window": storybookAlias(
        "../src/lib/tray/storybook/tauriWindowMock.ts",
      ),
    };
    if (Array.isArray(alias)) {
      for (const [find, replacement] of Object.entries(aliases)) {
        alias.push({ find, replacement });
      }
    } else {
      Object.assign(alias, aliases);
    }
    return viteConfig;
  },
};

export default config;
