// For more info, see https://github.com/storybookjs/eslint-plugin-storybook#configuration-flat-config-format
import storybook from "eslint-plugin-storybook";

import js from "@eslint/js";
import svelte from "eslint-plugin-svelte";
import unusedImports from "eslint-plugin-unused-imports";
import globals from "globals";
import ts from "typescript-eslint";
import svelteConfig from "./svelte.config.js";

export default ts.config(
  js.configs.recommended,
  ...ts.configs.recommended,
  ...svelte.configs["flat/recommended"],
  {
    languageOptions: {
      globals: {
        ...globals.browser,
        ...globals.node,
      },
    },
  },
  {
    files: ["**/*.svelte", "**/*.svelte.ts", "**/*.svelte.js"],
    languageOptions: {
      parserOptions: {
        parser: ts.parser,
        extraFileExtensions: [".svelte", ".svelte.ts", ".svelte.js"],
        svelteConfig,
      },
    },
    rules: {
      "@typescript-eslint/no-unused-expressions": "off",
    },
  },
  {
    plugins: {
      "unused-imports": unusedImports,
    },
    rules: {
      "@typescript-eslint/no-unused-vars": "off",
      "unused-imports/no-unused-imports": "error",
      "unused-imports/no-unused-vars": [
        "error",
        {
          vars: "all",
          varsIgnorePattern: "^_",
          args: "after-used",
          argsIgnorePattern: "^_",
        },
      ],
      "prefer-const": "off",
      "svelte/prefer-const": [
        "error",
        {
          destructuring: "all",
          excludedRunes: ["$props", "$derived", "$state"],
        },
      ],
    },
  },
  {
    ignores: [
      ".svelte-kit/**",
      "build/**",
      "src-tauri/gen/**",
      "node_modules/**",
      "target/**",
      "storybook-static/**",
      ".planning/**",
    ],
  },
  storybook.configs["flat/recommended"],
);
