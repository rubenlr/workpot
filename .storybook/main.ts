import type { StorybookConfig } from "@storybook/sveltekit";
import { applyEsbuildTarget } from "../vite-esbuild-target.ts";
import { applyStorybookTauriAliases } from "./storybook-aliases.ts";

const config: StorybookConfig = {
  stories: ["../src/**/*.stories.svelte"],
  addons: [
    "@storybook/addon-svelte-csf",
    "@storybook/addon-a11y",
    "@storybook/addon-docs",
    "@storybook/addon-vitest",
    "@chromatic-com/storybook",
  ],
  framework: "@storybook/sveltekit",
  async viteFinal(viteConfig) {
    applyEsbuildTarget(viteConfig);
    viteConfig.resolve ??= {};
    viteConfig.resolve.alias = applyStorybookTauriAliases(
      viteConfig.resolve.alias ?? {},
    );
    return viteConfig;
  },
};

export default config;
