import type { StorybookConfig } from "@storybook/sveltekit";
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
    viteConfig.resolve ??= {};
    viteConfig.resolve.alias ??= {};
    applyStorybookTauriAliases(viteConfig.resolve.alias);
    return viteConfig;
  },
};

export default config;
