import { describe, expect, it, vi } from "vitest";
import {
  cloneRepo,
  onRepoCloneComplete,
  onRepoCloneFailed,
  onRepoCloneStarted,
  restoreRepoCloneStatus,
} from "./trayRepoClone";

function deps(overrides: Record<string, unknown> = {}) {
  return {
    invoke: vi.fn().mockResolvedValue(undefined),
    refresh: vi.fn().mockResolvedValue(undefined),
    onError: vi.fn(),
    addCloning: vi.fn(),
    removeCloning: vi.fn(),
    onOpened: vi.fn(),
    ...overrides,
  };
}

describe("restoreRepoCloneStatus", () => {
  it("marks every in-flight clone path as cloning", async () => {
    const add = vi.fn();
    const invoke = vi.fn().mockResolvedValue(["/a", "/b"]);
    await restoreRepoCloneStatus(invoke, add);
    expect(invoke).toHaveBeenCalledWith("get_repo_clone_status");
    expect(add.mock.calls).toEqual([["/a"], ["/b"]]);
  });

  it("tolerates a null status", async () => {
    const add = vi.fn();
    await restoreRepoCloneStatus(vi.fn().mockResolvedValue(null), add);
    expect(add).not.toHaveBeenCalled();
  });
});

describe("cloneRepo", () => {
  it("invokes clone_repo with openAfter and shows the spinner", async () => {
    const d = deps();
    await cloneRepo({ path: "/code/acme/api" }, true, d);
    expect(d.addCloning).toHaveBeenCalledWith("/code/acme/api");
    expect(d.invoke).toHaveBeenCalledWith("clone_repo", {
      repoPath: "/code/acme/api",
      openAfter: true,
    });
    expect(d.removeCloning).not.toHaveBeenCalled();
  });

  it("drops the spinner and reports when the clone never started", async () => {
    const d = deps({ invoke: vi.fn().mockRejectedValue("boom") });
    await cloneRepo({ path: "/p" }, false, d);
    expect(d.removeCloning).toHaveBeenCalledWith("/p");
    expect(d.onError).toHaveBeenCalledWith("boom");
  });

  it("keeps the spinner when this repo is already being cloned", async () => {
    const d = deps({
      invoke: vi.fn().mockRejectedValue("this repo is already being cloned"),
    });
    await cloneRepo({ path: "/p" }, false, d);
    expect(d.removeCloning).not.toHaveBeenCalled();
    expect(d.onError).toHaveBeenCalled();
  });
});

describe("clone events", () => {
  it("started marks the repo as cloning", () => {
    const add = vi.fn();
    onRepoCloneStarted({ repo_path: "/p" }, add);
    expect(add).toHaveBeenCalledWith("/p");
  });

  it("complete clears the spinner, refreshes, and reports opened", async () => {
    const d = deps();
    await onRepoCloneComplete(
      { repo_path: "/p", new_path: "/p", opened: true },
      d,
    );
    expect(d.removeCloning).toHaveBeenCalledWith("/p");
    expect(d.refresh).toHaveBeenCalled();
    expect(d.onOpened).toHaveBeenCalled();
    expect(d.onError).not.toHaveBeenCalled();
  });

  it("complete without open does not signal opened", async () => {
    const d = deps();
    await onRepoCloneComplete({ repo_path: "/p", opened: false }, d);
    expect(d.onOpened).not.toHaveBeenCalled();
  });

  it("complete with a launch error surfaces it but still refreshes", async () => {
    const d = deps();
    await onRepoCloneComplete(
      { repo_path: "/p", opened: false, error: "failed to launch cursor" },
      d,
    );
    expect(d.refresh).toHaveBeenCalled();
    expect(d.onError).toHaveBeenCalledWith("failed to launch cursor");
    expect(d.onOpened).not.toHaveBeenCalled();
  });

  it("failed clears the spinner and shows the error", () => {
    const d = deps();
    onRepoCloneFailed({ repo_path: "/p", error: "clone failed: nope" }, d);
    expect(d.removeCloning).toHaveBeenCalledWith("/p");
    expect(d.onError).toHaveBeenCalledWith("clone failed: nope");
  });

  it("failed falls back to a generic message", () => {
    const d = deps();
    onRepoCloneFailed({ repo_path: "/p" }, d);
    expect(d.onError).toHaveBeenCalledWith("clone failed");
  });
});
