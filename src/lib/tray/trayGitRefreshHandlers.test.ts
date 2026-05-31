import { describe, expect, it, vi } from "vitest";
import type { GitRefreshSummary } from "$lib/types";
import {
  onGitRefreshComplete,
  onGitRefreshFailed,
  onPanelOpened,
  type GitRefreshHandlerDeps,
} from "./trayGitRefreshHandlers";

function deps(
  overrides: Partial<GitRefreshHandlerDeps> = {},
): GitRefreshHandlerDeps {
  return {
    setRefreshing: vi.fn(),
    setSelectedIndex: vi.fn(),
    refresh: vi.fn().mockResolvedValue(undefined),
    setError: vi.fn(),
    focusFilter: vi.fn(),
    ...overrides,
  };
}

describe("trayGitRefreshHandlers", () => {
  it("onPanelOpened refreshes, sets refreshing, and focuses filter", () => {
    const d = deps();
    onPanelOpened(d);
    expect(d.refresh).toHaveBeenCalledWith(true);
    expect(d.setRefreshing).toHaveBeenCalledWith(true);
    expect(d.focusFilter).toHaveBeenCalledOnce();
  });

  it("onGitRefreshComplete clears refreshing, resets selection, refreshes with clear flag", async () => {
    const d = deps();
    const summary: GitRefreshSummary = {
      refreshed: 2,
      errors: 0,
      any_dirty: false,
    };
    onGitRefreshComplete(summary, d);
    expect(d.setRefreshing).toHaveBeenCalledWith(false);
    expect(d.setSelectedIndex).toHaveBeenCalledWith(0);
    expect(d.refresh).toHaveBeenCalledWith(true);
    await vi.waitFor(() => expect(d.setError).toHaveBeenCalledWith(null));
  });

  it("onGitRefreshComplete sets partial error message when errors > 0", async () => {
    const d = deps();
    const summary: GitRefreshSummary = {
      refreshed: 1,
      errors: 2,
      any_dirty: true,
    };
    onGitRefreshComplete(summary, d);
    expect(d.refresh).toHaveBeenCalledWith(false);
    await vi.waitFor(() =>
      expect(d.setError).toHaveBeenCalledWith(
        "Git refresh completed with 2 error(s).",
      ),
    );
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

  it("onGitRefreshFailed clears refreshing and sets error", () => {
    const d = deps();
    onGitRefreshFailed("boom", d);
    expect(d.setRefreshing).toHaveBeenCalledWith(false);
    expect(d.setError).toHaveBeenCalledWith("boom");
  });
});
