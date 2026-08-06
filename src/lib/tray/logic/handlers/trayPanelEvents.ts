import { listen } from "@tauri-apps/api/event";
import type { UnlistenFn } from "@tauri-apps/api/event";
import type { SyncSummary, RepoConvertEvent, RepoSyncEvent } from "$lib/types";
import { trayTrace } from "./trayTrace";

export type ListenFn = <T>(
  event: string,
  handler: (event: { payload: T }) => void,
) => Promise<UnlistenFn>;

export interface TrayPanelEventHandlers {
  onPanelOpened: () => void;
  onPanelClosed: () => void;
  onSyncStarted: () => void;
  onSyncComplete: (summary: SyncSummary) => void;
  onSyncFailed: (message: string) => void;
  onRepoSyncStarted: (payload: RepoSyncEvent) => void;
  onRepoSyncComplete: (payload: RepoSyncEvent) => void;
  onRepoSyncFailed: (payload: RepoSyncEvent) => void;
  onRepoConvertStarted: (payload: RepoConvertEvent) => void;
  onRepoConvertComplete: (payload: RepoConvertEvent) => void;
  onRepoConvertFailed: (payload: RepoConvertEvent) => void;
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
    listenFn("sync-started", () => handlers.onSyncStarted()),
    listenFn<SyncSummary>("sync-complete", (event) =>
      handlers.onSyncComplete(event.payload),
    ),
    listenFn<string>("sync-failed", (event) =>
      handlers.onSyncFailed(event.payload),
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
    listenFn<RepoConvertEvent>("repo-convert-started", (event) =>
      handlers.onRepoConvertStarted(event.payload),
    ),
    listenFn<RepoConvertEvent>("repo-convert-complete", (event) =>
      handlers.onRepoConvertComplete(event.payload),
    ),
    listenFn<RepoConvertEvent>("repo-convert-failed", (event) =>
      handlers.onRepoConvertFailed(event.payload),
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
