import { describe, expect, it, vi } from "vitest";
import {
  convertRepo,
  onRepoConvertComplete,
  onRepoConvertFailed,
  onRepoConvertStarted,
  restoreRepoConvertStatus,
} from "./trayRepoConvert";

describe("trayRepoConvert", () => {
  it("convertRepo invokes convert_repo", async () => {
    const invoke = vi.fn().mockResolvedValue(undefined);
    await convertRepo("/tmp/repo", "bare", {
      invoke,
      refresh: vi.fn(),
      onError: vi.fn(),
      setActiveConvert: vi.fn(),
    });
    expect(invoke).toHaveBeenCalledWith("convert_repo", {
      repoPath: "/tmp/repo",
      target: "bare",
    });
  });

  it("convertRepo does not double-surface backend convert failures", async () => {
    const onError = vi.fn();
    const invoke = vi
      .fn()
      .mockRejectedValue(new Error("git worktree add failed: path exists"));
    await convertRepo("/tmp/repo", "local", {
      invoke,
      refresh: vi.fn(),
      onError,
      setActiveConvert: vi.fn(),
    });
    expect(onError).not.toHaveBeenCalled();
  });

  it("convertRepo surfaces pre-start invoke errors", async () => {
    const onError = vi.fn();
    const invoke = vi
      .fn()
      .mockRejectedValue(new Error("a repo convert is already in progress"));
    await convertRepo("/tmp/repo", "bare", {
      invoke,
      refresh: vi.fn(),
      onError,
      setActiveConvert: vi.fn(),
    });
    expect(onError).toHaveBeenCalled();
  });

  it("convertRepo surfaces invalid target pre-start errors", async () => {
    const onError = vi.fn();
    const invoke = vi
      .fn()
      .mockRejectedValue(new Error("invalid convert target"));
    await convertRepo("/tmp/repo", "local", {
      invoke,
      refresh: vi.fn(),
      onError,
      setActiveConvert: vi.fn(),
    });
    expect(onError).toHaveBeenCalled();
  });

  it("restoreRepoConvertStatus hydrates activeConvert from backend", async () => {
    const setActiveConvert = vi.fn();
    const invoke = vi.fn().mockResolvedValue({
      repo_path: "/tmp/r",
    });
    await restoreRepoConvertStatus(invoke, setActiveConvert);
    expect(invoke).toHaveBeenCalledWith("get_repo_convert_status");
    expect(setActiveConvert).toHaveBeenCalledWith({ repoPath: "/tmp/r" });
  });

  it("restoreRepoConvertStatus ignores empty status", async () => {
    const setActiveConvert = vi.fn();
    await restoreRepoConvertStatus(
      vi.fn().mockResolvedValue(null),
      setActiveConvert,
    );
    expect(setActiveConvert).not.toHaveBeenCalled();
  });

  it("onRepoConvertStarted sets activeConvert", () => {
    const setActiveConvert = vi.fn();
    onRepoConvertStarted({ repo_path: "/tmp/r" }, setActiveConvert);
    expect(setActiveConvert).toHaveBeenCalledWith({ repoPath: "/tmp/r" });
  });

  it("onRepoConvertComplete clears activeConvert and refreshes", async () => {
    const setActiveConvert = vi.fn();
    const refresh = vi.fn().mockResolvedValue(undefined);
    await onRepoConvertComplete(
      { repo_path: "/tmp/r" },
      { setActiveConvert, refresh },
    );
    expect(setActiveConvert).toHaveBeenCalledWith(null);
    expect(refresh).toHaveBeenCalled();
  });

  it("onRepoConvertFailed clears activeConvert and surfaces error", () => {
    const setActiveConvert = vi.fn();
    const onError = vi.fn();
    onRepoConvertFailed(
      { repo_path: "/tmp/r", error: "preflight failed" },
      { setActiveConvert, onError },
    );
    expect(setActiveConvert).toHaveBeenCalledWith(null);
    expect(onError).toHaveBeenCalledWith("preflight failed");
  });
});
