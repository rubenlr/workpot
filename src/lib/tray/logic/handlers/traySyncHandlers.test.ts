import { afterEach, describe, expect, it, vi } from "vitest";
import { clearSyncWatchdog } from "./syncWatchdog";
import {
  onPanelOpened,
  onSyncComplete,
  onSyncFailed,
  onSyncStarted,
  syncGitErrorMessage,
  type SyncHandlerDeps,
} from "./traySyncHandlers";

afterEach(() => {
  clearSyncWatchdog();
});

function deps(overrides: Partial<SyncHandlerDeps> = {}): SyncHandlerDeps {
  return {
    setSelectedIndex: vi.fn(),
    refresh: vi.fn().mockResolvedValue(undefined),
    resyncDetail: vi.fn(),
    setError: vi.fn(),
    focusFilter: vi.fn(),
    ...overrides,
  };
}

describe("traySyncHandlers", () => {
  it("syncGitErrorMessage surfaces total git failure", () => {
    expect(
      syncGitErrorMessage({
        added: 0,
        removed: 0,
        skipped: 0,
        git_refreshed: 0,
        git_errors: 2,
      }),
    ).toContain("failed");
    expect(
      syncGitErrorMessage({
        added: 1,
        removed: 0,
        skipped: 0,
        git_refreshed: 2,
        git_errors: 1,
      }),
    ).toBeNull();
  });

  it("onPanelOpened focuses filter without list refresh", () => {
    const d = deps();
    onPanelOpened(d);
    expect(d.refresh).not.toHaveBeenCalled();
    expect(d.focusFilter).toHaveBeenCalledOnce();
  });

  it("onSyncStarted arms watchdog that sets error on timeout", () => {
    vi.useFakeTimers();
    const setError = vi.fn();
    onSyncStarted({ setError });
    vi.advanceTimersByTime(90_000);
    expect(setError).toHaveBeenCalledOnce();
    expect(String(setError.mock.calls[0]?.[0])).toContain("timed out");
    vi.useRealTimers();
  });

  it("onSyncComplete resets selection, bumps branchRevision, and refreshes", async () => {
    const bumpBranchRevision = vi.fn();
    const d = deps({ bumpBranchRevision });

    onSyncComplete(
      {
        added: 1,
        removed: 0,
        skipped: 0,
        git_refreshed: 1,
        git_errors: 0,
      },
      d,
    );

    expect(d.setSelectedIndex).toHaveBeenCalledWith(0);
    expect(bumpBranchRevision).toHaveBeenCalledOnce();
    expect(d.refresh).toHaveBeenCalledWith(true);
    await vi.mocked(d.refresh).mock.results[0]?.value;
    expect(d.resyncDetail).toHaveBeenCalledOnce();
    expect(d.setError).toHaveBeenCalledWith(null);
  });

  it("onSyncFailed clears watchdog and sets error", () => {
    const setError = vi.fn();
    onSyncFailed("boom", { setError });
    expect(setError).toHaveBeenCalledWith("boom");
  });
});
