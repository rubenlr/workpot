import path from "node:path";
import { fileURLToPath } from "node:url";
import { storybookTest } from "@storybook/addon-vitest/vitest-plugin";
import { storybookSveltekitPlugin } from "@storybook/sveltekit/vite-plugin";
import { svelteTesting } from "@testing-library/svelte/vite";
import { mergeConfig } from "vite";
import { defineConfig } from "vitest/config";
import { storybookTauriAliases } from "./.storybook/storybook-aliases";
import { nonTestableCoverageGlobs } from "./scripts/coverage-exclusions.mjs";
import viteConfig from "./vite.config";

const dirname = path.dirname(fileURLToPath(import.meta.url));

export default mergeConfig(
  viteConfig,
  defineConfig({
    plugins: [svelteTesting()],
    resolve: { conditions: ["browser"] },
    test: {
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
      projects: [
        {
          extends: true,
          test: {
            name: "unit",
            environment: "jsdom",
          },
        },
        {
          extends: true,
          plugins: [
            storybookTest({
              configDir: path.join(dirname, ".storybook"),
              storybookScript: "pnpm storybook --no-open",
            }),
            storybookSveltekitPlugin(),
          ],
          resolve: {
            alias: storybookTauriAliases,
          },
          test: {
            name: "storybook",
            browser: {
              enabled: true,
              provider: "playwright",
              headless: true,
              instances: [{ browser: "chromium" }],
            },
          },
        },
      ],
    },
  }),
);
