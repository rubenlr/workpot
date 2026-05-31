import { describe, expect, it, vi } from "vitest";
import { subscribeTrayPanelEvents, type ListenFn } from "./trayPanelEvents";
import type { GitRefreshSummary } from "$lib/types";

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
  it("registers four listeners and unsubscribes all", async () => {
    const { listen, unsubs, handlers } = mockListen();
    const onPanelOpened = vi.fn();
    const onGitRefreshComplete = vi.fn();
    const onGitRefreshFailed = vi.fn();
    const onRepoContextAction = vi.fn();

    const unsubscribe = subscribeTrayPanelEvents(
      {
        onPanelOpened,
        onGitRefreshComplete,
        onGitRefreshFailed,
        onRepoContextAction,
      },
      listen,
    );

    await vi.waitFor(() => expect(listen).toHaveBeenCalledTimes(4));

    handlers.get("panel-opened")!({ payload: undefined });
    expect(onPanelOpened).toHaveBeenCalledOnce();

    const summary: GitRefreshSummary = {
      refreshed: 1,
      errors: 0,
      any_dirty: false,
    };
    handlers.get("git-refresh-complete")!({ payload: summary });
    expect(onGitRefreshComplete).toHaveBeenCalledWith(summary);

    handlers.get("git-refresh-failed")!({ payload: "boom" });
    expect(onGitRefreshFailed).toHaveBeenCalledWith("boom");

    const ctx = { action: "pin", repo_path: "/tmp/x" };
    handlers.get("repo-context-action")!({ payload: ctx });
    expect(onRepoContextAction).toHaveBeenCalledWith(ctx);

    unsubscribe();
    await vi.waitFor(() => expect(unsubs.every((u) => u.mock.calls.length === 1)).toBe(true));
  });
});
