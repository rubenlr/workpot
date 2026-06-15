import { getCurrentWindow } from "@tauri-apps/api/window";

function inTauriRuntime(): boolean {
  return Boolean(
    (globalThis as typeof globalThis & { isTauri?: boolean }).isTauri,
  );
}

export function applyDocumentTheme(theme: "light" | "dark" | null): void {
  const root = document.documentElement;
  if (theme) {
    root.dataset.theme = theme;
  } else {
    delete root.dataset.theme;
  }
}

export async function initSystemThemeSync(): Promise<() => void> {
  if (!inTauriRuntime()) {
    return () => {};
  }

  const window = getCurrentWindow();
  applyDocumentTheme(await window.theme());
  return window.onThemeChanged(({ payload }) => {
    applyDocumentTheme(payload);
  });
}
