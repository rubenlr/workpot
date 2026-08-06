import type { SyncSummary } from "$lib/types";
import { armSyncWatchdog, clearSyncWatchdog } from "./syncWatchdog";
import { trayTrace } from "./trayTrace";

export interface SyncHandlerDeps {
  setSelectedIndex: (index: number) => void;
  refresh: (clearError: boolean) => Promise<void>;
  resyncDetail: () => void;
  setError: (message: string | null) => void;
  bumpBranchRevision?: () => void;
  focusFilter?: () => void;
}

export function syncGitErrorMessage(summary: SyncSummary): string | null {
  if (summary.git_errors === 0) {
    return null;
  }
  if (summary.git_refreshed === 0) {
    return "Sync failed to update git state for all repositories.";
  }
  return null;
}

/**
 * Panel open: focus filter only. Rust spawns catalog sync; list refresh runs
 * once on sync-complete (avoids double list_repos with panel-opened).
 */
export function onPanelOpened(deps: SyncHandlerDeps): void {
  trayTrace("panel-opened");
  deps.focusFilter?.();
}

export function onSyncStarted(deps: Pick<SyncHandlerDeps, "setError">): void {
  trayTrace("sync-started");
  armSyncWatchdog(() => {
    trayTrace("sync watchdog fired (no sync-complete)");
    deps.setError(
      "Sync timed out waiting for sync-complete. Check the terminal (RUST_LOG=debug just launch) and the tray webview console (right-click → Inspect).",
    );
  });
}

export function onSyncComplete(
  summary: SyncSummary,
  deps: SyncHandlerDeps,
): void {
  trayTrace("sync-complete", summary);
  clearSyncWatchdog();
  deps.setSelectedIndex(0);
  deps.bumpBranchRevision?.();
  void deps.refresh(true).then(() => {
    deps.resyncDetail();
    deps.setError(syncGitErrorMessage(summary));
  });
}

export function onSyncFailed(
  message: string,
  deps: Pick<SyncHandlerDeps, "setError">,
): void {
  trayTrace("sync-failed", message);
  clearSyncWatchdog();
  deps.setError(message);
}
