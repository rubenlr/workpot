import { afterEach, describe, expect, it, vi } from "vitest";
import type { GitRefreshSummary } from "$lib/types";
import { clearGitRefreshWatchdog } from "./gitRefreshWatchdog";
import {
  onGitRefreshComplete,
  onGitRefreshFailed,
  onPanelOpened,
  type GitRefreshHandlerDeps,
} from "./trayGitRefreshHandlers";

afterEach(() => {
  clearGitRefreshWatchdog();
});

function deps(
  overrides: Partial<GitRefreshHandlerDeps> = {},
): GitRefreshHandlerDeps {
  return {
    setSelectedIndex: vi.fn(),
    refresh: vi.fn().mockResolvedValue(undefined),
    setError: vi.fn(),
    focusFilter: vi.fn(),
    ...overrides,
  };
}

describe("trayGitRefreshHandlers", () => {
  it("onPanelOpened loads cached list and focuses filter", () => {
    const d = deps();
    onPanelOpened(d);
    expect(d.refresh).toHaveBeenCalledWith(true);
    expect(d.focusFilter).toHaveBeenCalledOnce();
  });

  it("onGitRefreshComplete resets selection and refreshes with clear flag", async () => {
    const d = deps();
    const summary: GitRefreshSummary = {
      refreshed: 2,
      errors: 0,
      any_dirty: false,
    };
    onGitRefreshComplete(summary, d);
    expect(d.setSelectedIndex).toHaveBeenCalledWith(0);
    expect(d.refresh).toHaveBeenCalledWith(true);
    await vi.waitFor(() => expect(d.setError).toHaveBeenCalledWith(null));
  });

  it("onGitRefreshComplete clears list error on partial failure", async () => {
    const d = deps();
    const summary: GitRefreshSummary = {
      refreshed: 1,
      errors: 2,
      any_dirty: true,
    };
    onGitRefreshComplete(summary, d);
    expect(d.refresh).toHaveBeenCalledWith(true);
    await vi.waitFor(() => expect(d.setError).toHaveBeenCalledWith(null));
  });

  it("onGitRefreshComplete sets total failure message when all failed", async () => {
    const d = deps();
    const summary: GitRefreshSummary = {
      refreshed: 0,
      errors: 3,
      any_dirty: false,
    };
    onGitRefreshComplete(summary, d);
    await vi.waitFor(() =>
      expect(d.setError).toHaveBeenCalledWith(
        "Git refresh failed for all repositories.",
      ),
    );
  });

  it("onGitRefreshFailed sets error", () => {
    const d = deps();
    onGitRefreshFailed("boom", d);
    expect(d.setError).toHaveBeenCalledWith("boom");
  });
});
