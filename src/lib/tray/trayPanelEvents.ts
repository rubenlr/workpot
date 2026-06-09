import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { GitRefreshSummary } from "$lib/types";
import { trayTrace } from "./trayTrace";

export type ListenFn = <T>(
  event: string,
  handler: (event: { payload: T }) => void,
) => Promise<UnlistenFn>;

export interface TrayPanelEventHandlers {
  onPanelOpened: () => void;
  onGitRefreshComplete: (summary: GitRefreshSummary) => void;
  onGitRefreshFailed: (message: string) => void;
  onRepoContextAction: (payload: { action: string; repo_path: string }) => void;
}

/** Subscribe to tray Tauri events; returned fn unsubscribes all listeners. */
export async function subscribeTrayPanelEvents(
  handlers: TrayPanelEventHandlers,
  listenFn: ListenFn = listen,
): Promise<() => void> {
  trayTrace("registering tray event listeners");
  const unsubs = await Promise.all([
    listenFn("panel-opened", () => handlers.onPanelOpened()),
    listenFn<GitRefreshSummary>("git-refresh-complete", (event) =>
      handlers.onGitRefreshComplete(event.payload),
    ),
    listenFn<string>("git-refresh-failed", (event) =>
      handlers.onGitRefreshFailed(event.payload),
    ),
    listenFn<{ action: string; repo_path: string }>(
      "repo-context-action",
      (event) => handlers.onRepoContextAction(event.payload),
    ),
  ]);
  trayTrace("tray event listeners ready");
  return () => {
    for (const fn of unsubs) {
      fn();
    }
  };
}
