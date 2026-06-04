import {
  gitRefreshErrorMessage,
  shouldClearListErrorOnRefreshLoad,
} from "$lib/gitRefresh";
import type { GitRefreshSummary } from "$lib/types";
import {
  armGitRefreshWatchdog,
  clearGitRefreshWatchdog,
} from "./gitRefreshWatchdog";
import { trayTrace } from "./trayTrace";

export interface GitRefreshHandlerDeps {
  setSelectedIndex: (index: number) => void;
  refresh: (clearError: boolean) => Promise<void>;
  setError: (message: string | null) => void;
  focusFilter: () => void;
}

export function onPanelOpened(deps: GitRefreshHandlerDeps): void {
  trayTrace("panel-opened");
  void deps.refresh(true);
  armGitRefreshWatchdog(() => {
    trayTrace("git refresh watchdog fired (no git-refresh-complete)");
    deps.setError(
      "Git refresh timed out waiting for git-refresh-complete. Check the terminal (RUST_LOG=debug just launch) and the tray webview console (right-click → Inspect).",
    );
  });
  deps.focusFilter();
}

export function onGitRefreshComplete(
  summary: GitRefreshSummary,
  deps: GitRefreshHandlerDeps,
): void {
  trayTrace("git-refresh-complete", summary);
  clearGitRefreshWatchdog();
  deps.setSelectedIndex(0);
  void deps.refresh(shouldClearListErrorOnRefreshLoad()).then(() => {
    deps.setError(gitRefreshErrorMessage(summary));
  });
}

export function onGitRefreshFailed(
  message: string,
  deps: Pick<GitRefreshHandlerDeps, "setError">,
): void {
  trayTrace("git-refresh-failed", message);
  clearGitRefreshWatchdog();
  deps.setError(message);
}
