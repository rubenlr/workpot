import path from "node:path";
import { fileURLToPath } from "node:url";

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

export function applyStorybookTauriAliases(
  alias:
    | Record<string, string>
    | Array<{ find: string | RegExp; replacement: string }>,
): void {
  if (Array.isArray(alias)) {
    for (const [find, replacement] of Object.entries(storybookTauriAliases)) {
      alias.push({ find, replacement });
    }
  } else {
    Object.assign(alias, storybookTauriAliases);
  }
}
