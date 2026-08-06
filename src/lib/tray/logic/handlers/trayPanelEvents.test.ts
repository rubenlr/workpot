import { describe, expect, it, vi } from "vitest";
import { subscribeTrayPanelEvents, type ListenFn } from "./trayPanelEvents";
import type { SyncSummary, RepoSyncEvent } from "$lib/types";

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
  it("registers sync listeners without git-refresh events", async () => {
    const { listen, unsubs, handlers } = mockListen();
    const onPanelOpened = vi.fn();
    const onPanelClosed = vi.fn();
    const onSyncStarted = vi.fn();
    const onSyncComplete = vi.fn();
    const onSyncFailed = vi.fn();
    const onRepoSyncStarted = vi.fn();
    const onRepoSyncComplete = vi.fn();
    const onRepoSyncFailed = vi.fn();
    const onRepoConvertStarted = vi.fn();
    const onRepoConvertComplete = vi.fn();
    const onRepoConvertFailed = vi.fn();
    const onRepoContextAction = vi.fn();

    const unsubscribe = await subscribeTrayPanelEvents(
      {
        onPanelOpened,
        onPanelClosed,
        onSyncStarted,
        onSyncComplete,
        onSyncFailed,
        onRepoSyncStarted,
        onRepoSyncComplete,
        onRepoSyncFailed,
        onRepoConvertStarted,
        onRepoConvertComplete,
        onRepoConvertFailed,
        onRepoContextAction,
      },
      listen,
    );

    expect(listen).toHaveBeenCalledTimes(12);
    expect(handlers.has("git-refresh-started")).toBe(false);
    expect(handlers.has("git-refresh-complete")).toBe(false);
    expect(handlers.has("git-refresh-failed")).toBe(false);

    handlers.get("panel-opened")!({ payload: undefined });
    expect(onPanelOpened).toHaveBeenCalledOnce();

    handlers.get("panel-closed")!({ payload: undefined });
    expect(onPanelClosed).toHaveBeenCalledOnce();

    handlers.get("sync-started")!({ payload: undefined });
    expect(onSyncStarted).toHaveBeenCalledOnce();

    const syncSummary: SyncSummary = {
      added: 1,
      removed: 0,
      skipped: 0,
      git_refreshed: 2,
      git_errors: 0,
    };
    handlers.get("sync-complete")!({ payload: syncSummary });
    expect(onSyncComplete).toHaveBeenCalledWith(syncSummary);

    handlers.get("sync-failed")!({ payload: "sync boom" });
    expect(onSyncFailed).toHaveBeenCalledWith("sync boom");

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

    const convertEvent = { repo_path: "/tmp/x" };
    handlers.get("repo-convert-started")!({ payload: convertEvent });
    expect(onRepoConvertStarted).toHaveBeenCalledWith(convertEvent);

    handlers.get("repo-convert-complete")!({
      payload: { ...convertEvent, new_path: "/tmp/x-bare" },
    });
    expect(onRepoConvertComplete).toHaveBeenCalled();

    handlers.get("repo-convert-failed")!({
      payload: { ...convertEvent, error: "failed" },
    });
    expect(onRepoConvertFailed).toHaveBeenCalled();

    const ctx = { action: "pin", repo_path: "/tmp/x" };
    handlers.get("repo-context-action")!({ payload: ctx });
    expect(onRepoContextAction).toHaveBeenCalledWith(ctx);

    unsubscribe();
    expect(unsubs.every((u) => u.mock.calls.length === 1)).toBe(true);
  });
});
