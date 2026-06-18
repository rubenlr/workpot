import path from "node:path";
import { fileURLToPath } from "node:url";
import type { AliasOptions } from "vite";

const dirname = path.dirname(fileURLToPath(import.meta.url));

function storybookAlias(relativePath: string): string {
  return path.join(dirname, relativePath);
}

export const storybookTauriAliases = {
  "@tauri-apps/api/core": storybookAlias(
    "../src/lib/tray/storybook/tauriCoreMock.ts",
  ),
  "@tauri-apps/api/event": storybookAlias(
    "../src/lib/tray/storybook/tauriEventMock.ts",
  ),
  "@tauri-apps/api/window": storybookAlias(
    "../src/lib/tray/storybook/tauriWindowMock.ts",
  ),
} as const;

export function applyStorybookTauriAliases(alias: AliasOptions): AliasOptions {
  const additions = Object.entries(storybookTauriAliases).map(
    ([find, replacement]) => ({ find, replacement }),
  );
  if (Array.isArray(alias)) {
    return [...alias, ...additions];
  }
  return { ...alias, ...storybookTauriAliases };
}
