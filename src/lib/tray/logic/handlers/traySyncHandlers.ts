import type { SyncSummary } from "$lib/types";
import { trayTrace } from "./trayTrace";

export interface SyncHandlerDeps {
  setSelectedIndex: (index: number) => void;
  refresh: (clearError: boolean) => Promise<void>;
  resyncDetail: () => void;
  setError: (message: string | null) => void;
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

export function onSyncComplete(
  summary: SyncSummary,
  deps: SyncHandlerDeps,
): void {
  trayTrace("sync-complete", summary);
  deps.setSelectedIndex(0);
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
  deps.setError(message);
}
