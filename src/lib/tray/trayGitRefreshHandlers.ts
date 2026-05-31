import {
  gitRefreshErrorMessage,
  shouldClearListErrorOnRefreshLoad,
} from "$lib/gitRefresh";
import type { GitRefreshSummary } from "$lib/types";

export interface GitRefreshHandlerDeps {
  setRefreshing: (value: boolean) => void;
  setSelectedIndex: (index: number) => void;
  refresh: (clearError: boolean) => Promise<void>;
  setError: (message: string | null) => void;
  focusFilter: () => void;
}

export function onPanelOpened(deps: GitRefreshHandlerDeps): void {
  void deps.refresh(true);
  deps.setRefreshing(true);
  deps.focusFilter();
}

export function onGitRefreshComplete(
  summary: GitRefreshSummary,
  deps: GitRefreshHandlerDeps,
): void {
  deps.setRefreshing(false);
  deps.setSelectedIndex(0);
  void deps.refresh(shouldClearListErrorOnRefreshLoad(summary)).then(() => {
    deps.setError(gitRefreshErrorMessage(summary));
  });
}

export function onGitRefreshFailed(
  message: string,
  deps: Pick<GitRefreshHandlerDeps, "setRefreshing" | "setError">,
): void {
  deps.setRefreshing(false);
  deps.setError(message);
}
