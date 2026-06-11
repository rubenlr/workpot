import type { IndexSummary } from "$lib/types";
import { trayTrace } from "./trayTrace";

export interface IndexHandlerDeps {
  setSelectedIndex: (index: number) => void;
  refresh: (clearError: boolean) => Promise<void>;
  resyncDetail: () => void;
  setError: (message: string | null) => void;
}

export function indexGitErrorMessage(summary: IndexSummary): string | null {
  if (summary.git_errors === 0) {
    return null;
  }
  if (summary.git_refreshed === 0) {
    return "Index refresh failed to update git state for all repositories.";
  }
  return null;
}

export function onIndexComplete(
  summary: IndexSummary,
  deps: IndexHandlerDeps,
): void {
  trayTrace("index-complete", summary);
  deps.setSelectedIndex(0);
  void deps.refresh(true).then(() => {
    deps.resyncDetail();
    deps.setError(indexGitErrorMessage(summary));
  });
}

export function onIndexFailed(
  message: string,
  deps: Pick<IndexHandlerDeps, "setError">,
): void {
  trayTrace("index-failed", message);
  deps.setError(message);
}
