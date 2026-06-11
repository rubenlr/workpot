import { describe, expect, it, vi } from "vitest";
import { subscribeTrayPanelEvents, type ListenFn } from "./trayPanelEvents";
import type {
  GitRefreshSummary,
  IndexSummary,
  RepoSyncEvent,
} from "$lib/types";

function mockListen(): {
  listen: ListenFn;
  unsubs: ReturnType<typeof vi.fn>[];
  handlers: Map<string, (event: { payload: unknown }) => void>;
} {
  const unsubs: ReturnType<typeof vi.fn>[] = [];
  const handlers = new Map<string, (event: { payload: unknown }) => void>();

  const listen: ListenFn = vi.fn(async (event, handler) => {
    handlers.set(event, handler as (event: { payload: unknown }) => void);
    const unsub = vi.fn();
    unsubs.push(unsub);
    return unsub;
  });

  return { listen, unsubs, handlers };
}

describe("subscribeTrayPanelEvents", () => {
  it("registers listeners and unsubscribes all", async () => {
    const { listen, unsubs, handlers } = mockListen();
    const onPanelOpened = vi.fn();
    const onPanelClosed = vi.fn();
    const onGitRefreshStarted = vi.fn();
    const onGitRefreshComplete = vi.fn();
    const onGitRefreshFailed = vi.fn();
    const onIndexStarted = vi.fn();
    const onIndexComplete = vi.fn();
    const onIndexFailed = vi.fn();
    const onRepoSyncStarted = vi.fn();
    const onRepoSyncComplete = vi.fn();
    const onRepoSyncFailed = vi.fn();
    const onRepoContextAction = vi.fn();

    const unsubscribe = await subscribeTrayPanelEvents(
      {
        onPanelOpened,
        onPanelClosed,
        onGitRefreshStarted,
        onGitRefreshComplete,
        onGitRefreshFailed,
        onIndexStarted,
        onIndexComplete,
        onIndexFailed,
        onRepoSyncStarted,
        onRepoSyncComplete,
        onRepoSyncFailed,
        onRepoContextAction,
      },
      listen,
    );

    expect(listen).toHaveBeenCalledTimes(12);

    handlers.get("panel-opened")!({ payload: undefined });
    expect(onPanelOpened).toHaveBeenCalledOnce();

    handlers.get("panel-closed")!({ payload: undefined });
    expect(onPanelClosed).toHaveBeenCalledOnce();

    handlers.get("git-refresh-started")!({ payload: undefined });
    expect(onGitRefreshStarted).toHaveBeenCalledOnce();

    const summary: GitRefreshSummary = {
      refreshed: 1,
      errors: 0,
      any_dirty: false,
    };
    handlers.get("git-refresh-complete")!({ payload: summary });
    expect(onGitRefreshComplete).toHaveBeenCalledWith(summary);

    handlers.get("git-refresh-failed")!({ payload: "boom" });
    expect(onGitRefreshFailed).toHaveBeenCalledWith("boom");

    handlers.get("index-started")!({ payload: undefined });
    expect(onIndexStarted).toHaveBeenCalledOnce();

    const indexSummary: IndexSummary = {
      added: 1,
      removed: 0,
      skipped: 0,
      git_refreshed: 2,
      git_errors: 0,
    };
    handlers.get("index-complete")!({ payload: indexSummary });
    expect(onIndexComplete).toHaveBeenCalledWith(indexSummary);

    handlers.get("index-failed")!({ payload: "index boom" });
    expect(onIndexFailed).toHaveBeenCalledWith("index boom");

    const syncEvent: RepoSyncEvent = {
      repo_path: "/tmp/x",
      branch: "main",
      direction: "push",
    };
    handlers.get("repo-sync-started")!({ payload: syncEvent });
    expect(onRepoSyncStarted).toHaveBeenCalledWith(syncEvent);

    handlers.get("repo-sync-complete")!({ payload: syncEvent });
    expect(onRepoSyncComplete).toHaveBeenCalledWith(syncEvent);

    handlers.get("repo-sync-failed")!({
      payload: { ...syncEvent, error: "failed" },
    });
    expect(onRepoSyncFailed).toHaveBeenCalled();

    const ctx = { action: "pin", repo_path: "/tmp/x" };
    handlers.get("repo-context-action")!({ payload: ctx });
    expect(onRepoContextAction).toHaveBeenCalledWith(ctx);

    unsubscribe();
    expect(unsubs.every((u) => u.mock.calls.length === 1)).toBe(true);
  });
});
