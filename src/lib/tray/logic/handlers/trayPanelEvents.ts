import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type {
  GitRefreshSummary,
  IndexSummary,
  RepoSyncEvent,
} from "$lib/types";
import { trayTrace } from "./trayTrace";

export type ListenFn = <T>(
  event: string,
  handler: (event: { payload: T }) => void,
) => Promise<UnlistenFn>;

export interface TrayPanelEventHandlers {
  onPanelOpened: () => void;
  onPanelClosed: () => void;
  onGitRefreshStarted: () => void;
  onGitRefreshComplete: (summary: GitRefreshSummary) => void;
  onGitRefreshFailed: (message: string) => void;
  onIndexStarted: () => void;
  onIndexComplete: (summary: IndexSummary) => void;
  onIndexFailed: (message: string) => void;
  onRepoSyncStarted: (payload: RepoSyncEvent) => void;
  onRepoSyncComplete: (payload: RepoSyncEvent) => void;
  onRepoSyncFailed: (payload: RepoSyncEvent) => void;
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
    listenFn("panel-closed", () => handlers.onPanelClosed()),
    listenFn("git-refresh-started", () => handlers.onGitRefreshStarted()),
    listenFn<GitRefreshSummary>("git-refresh-complete", (event) =>
      handlers.onGitRefreshComplete(event.payload),
    ),
    listenFn<string>("git-refresh-failed", (event) =>
      handlers.onGitRefreshFailed(event.payload),
    ),
    listenFn("index-started", () => handlers.onIndexStarted()),
    listenFn<IndexSummary>("index-complete", (event) =>
      handlers.onIndexComplete(event.payload),
    ),
    listenFn<string>("index-failed", (event) =>
      handlers.onIndexFailed(event.payload),
    ),
    listenFn<RepoSyncEvent>("repo-sync-started", (event) =>
      handlers.onRepoSyncStarted(event.payload),
    ),
    listenFn<RepoSyncEvent>("repo-sync-complete", (event) =>
      handlers.onRepoSyncComplete(event.payload),
    ),
    listenFn<RepoSyncEvent>("repo-sync-failed", (event) =>
      handlers.onRepoSyncFailed(event.payload),
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
