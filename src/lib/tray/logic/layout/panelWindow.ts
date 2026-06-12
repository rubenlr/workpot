import { LogicalSize } from "@tauri-apps/api/dpi";
import { getCurrentWindow } from "@tauri-apps/api/window";

const PANEL_WIDTH_PX = 400;

let lastSyncedHeightPx: number | null = null;

function inTauriRuntime(): boolean {
  return Boolean(
    (globalThis as typeof globalThis & { isTauri?: boolean }).isTauri,
  );
}

export async function syncPanelWindowHeight(heightPx: number): Promise<void> {
  if (!inTauriRuntime()) return;
  if (heightPx === lastSyncedHeightPx) return;
  lastSyncedHeightPx = heightPx;

  const window = getCurrentWindow();
  await window.setSize(new LogicalSize(PANEL_WIDTH_PX, heightPx));
}
