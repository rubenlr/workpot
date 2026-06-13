import { describe, expect, it, vi } from "vitest";
import {
  onRepoSyncComplete,
  onRepoSyncFailed,
  onRepoSyncStarted,
  restoreRepoSyncStatus,
  syncRepoBranch,
} from "./trayRepoSync";

describe("trayRepoSync", () => {
  it("syncRepoBranch invokes sync_repo_branch", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    const setActiveSync = vi.fn();
    await syncRepoBranch("/tmp/repo", "main", "push", {
      invoke,
      refresh: vi.fn(),
      onError: vi.fn(),
      setActiveSync,
    });
    expect(invoke).toHaveBeenCalledWith("sync_repo_branch", {
      repoPath: "/tmp/repo",
      branch: "main",
      direction: "push",
    });
  });

  it("syncRepoBranch does not double-surface git sync failures", async () => {
    const onError = vi.fn();
    const invoke = vi
      .fn()
      .mockRejectedValue(new Error("pre-push hook failed: cargo-fmt-check"));
    await syncRepoBranch("/tmp/repo", "main", "push", {
      invoke,
      refresh: vi.fn(),
      onError,
      setActiveSync: vi.fn(),
    });
    expect(onError).not.toHaveBeenCalled();
  });

  it("syncRepoBranch surfaces pre-start invoke errors", async () => {
    const onError = vi.fn();
    const invoke = vi
      .fn()
      .mockRejectedValue(new Error("a repo sync is already in progress"));
    await syncRepoBranch("/tmp/repo", "main", "push", {
      invoke,
      refresh: vi.fn(),
      onError,
      setActiveSync: vi.fn(),
    });
    expect(onError).toHaveBeenCalled();
  });

  it("restoreRepoSyncStatus hydrates activeSync from backend", async () => {
    const setActiveSync = vi.fn();
    const invoke = vi.fn().mockResolvedValue({
      repo_path: "/tmp/r",
      branch: "main",
      direction: "push",
    });
    await restoreRepoSyncStatus(invoke, setActiveSync);
    expect(invoke).toHaveBeenCalledWith("get_repo_sync_status");
    expect(setActiveSync).toHaveBeenCalledWith({
      repoPath: "/tmp/r",
      branch: "main",
      direction: "push",
    });
  });

  it("restoreRepoSyncStatus ignores empty status", async () => {
    const setActiveSync = vi.fn();
    await restoreRepoSyncStatus(vi.fn().mockResolvedValue(null), setActiveSync);
    expect(setActiveSync).not.toHaveBeenCalled();
  });

  it("onRepoSyncStarted sets activeSync", () => {
    const setActiveSync = vi.fn();
    onRepoSyncStarted(
      { repo_path: "/tmp/r", branch: "main", direction: "pull" },
      setActiveSync,
    );
    expect(setActiveSync).toHaveBeenCalledWith({
      repoPath: "/tmp/r",
      branch: "main",
      direction: "pull",
    });
  });

  it("onRepoSyncComplete clears activeSync and refreshes", async () => {
    const setActiveSync = vi.fn();
    const refresh = vi.fn().mockResolvedValue(undefined);
    const bumpBranchRevision = vi.fn();
    await onRepoSyncComplete(
      { repo_path: "/tmp/r", branch: "main", direction: "push" },
      { setActiveSync, refresh, bumpBranchRevision },
    );
    expect(setActiveSync).toHaveBeenCalledWith(null);
    expect(refresh).toHaveBeenCalled();
    expect(bumpBranchRevision).toHaveBeenCalled();
  });

  it("onRepoSyncFailed clears activeSync and surfaces error", () => {
    const setActiveSync = vi.fn();
    const onError = vi.fn();
    onRepoSyncFailed(
      {
        repo_path: "/tmp/r",
        branch: "main",
        direction: "pull",
        error: "rejected",
      },
      { setActiveSync, onError },
    );
    expect(setActiveSync).toHaveBeenCalledWith(null);
    expect(onError).toHaveBeenCalledWith("rejected");
  });
});
