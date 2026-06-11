import { describe, expect, it, vi } from "vitest";
import { subscribeTrayPanelEvents, type ListenFn } from "./trayPanelEvents";
import type { GitRefreshSummary, RepoSyncEvent } from "$lib/types";

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
  it("registers eight listeners and unsubscribes all", async () => {
    const { listen, unsubs, handlers } = mockListen();
    const onPanelOpened = vi.fn();
    const onGitRefreshStarted = vi.fn();
    const onGitRefreshComplete = vi.fn();
    const onGitRefreshFailed = vi.fn();
    const onRepoSyncStarted = vi.fn();
    const onRepoSyncComplete = vi.fn();
    const onRepoSyncFailed = vi.fn();
    const onRepoContextAction = vi.fn();

    const unsubscribe = await subscribeTrayPanelEvents(
      {
        onPanelOpened,
        onGitRefreshStarted,
        onGitRefreshComplete,
        onGitRefreshFailed,
        onRepoSyncStarted,
        onRepoSyncComplete,
        onRepoSyncFailed,
        onRepoContextAction,
      },
      listen,
    );

    expect(listen).toHaveBeenCalledTimes(8);

    handlers.get("panel-opened")!({ payload: undefined });
    expect(onPanelOpened).toHaveBeenCalledOnce();

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
