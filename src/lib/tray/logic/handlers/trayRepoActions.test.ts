import { describe, expect, it, vi } from "vitest";
import type { RepoDto } from "$lib/types";
import {
  executeContextCommand,
  handleRepoContextAction,
  removeTag,
  setPinOrder,
  type TrayRepoActionsDeps,
} from "./trayRepoActions";

function repo(overrides: Partial<RepoDto> = {}): RepoDto {
  return {
    path: "/tmp/foo",
    name: "foo",
    alias: null,
    branch: null,
    ahead: null,
    behind: null,
    is_dirty: null,
    parent_dir: "",
    last_opened_at: null,
    git_state_error: null,
    pinned: false,
    pin_order: null,
    notes: null,
    tags: [],
    branches: [],
    ...overrides,
  };
}

function deps(
  overrides: Partial<TrayRepoActionsDeps> = {},
): TrayRepoActionsDeps {
  return {
    invoke: vi.fn().mockResolvedValue(undefined),
    refresh: vi.fn().mockResolvedValue(undefined),
    onError: vi.fn(),
    openDetailWithTagFocus: vi.fn(),
    ...overrides,
  };
}

describe("trayRepoActions", () => {
  it("mutateThenRefresh invokes, refreshes, and skips onError on success", async () => {
    const d = deps();
    await removeTag("/tmp/foo", "work", d);
    expect(d.invoke).toHaveBeenCalledWith("remove_tag", {
      repoPath: "/tmp/foo",
      tag: "work",
    });
    expect(d.refresh).toHaveBeenCalledOnce();
    expect(d.onError).not.toHaveBeenCalled();
  });

  it("mutateThenRefresh calls onError without refresh when invoke fails", async () => {
    const d = deps({
      invoke: vi.fn().mockRejectedValue(new Error("fail")),
    });
    await removeTag("/tmp/foo", "work", d);
    expect(d.onError).toHaveBeenCalledOnce();
    expect(d.refresh).not.toHaveBeenCalled();
  });

  it("executeContextCommand toggles pin via mutateThenRefresh", async () => {
    const d = deps();
    await executeContextCommand(
      { kind: "toggle_pin", repoPath: "/tmp/foo", pinned: true },
      d,
    );
    expect(d.invoke).toHaveBeenCalledWith("set_pin", {
      repoPath: "/tmp/foo",
      pinned: true,
    });
    expect(d.refresh).toHaveBeenCalledOnce();
  });

  it("executeContextCommand opens detail with tag focus without invoke", async () => {
    const r = repo();
    const d = deps();
    await executeContextCommand({ kind: "open_detail_tag_focus", repo: r }, d);
    expect(d.openDetailWithTagFocus).toHaveBeenCalledWith(r);
    expect(d.invoke).not.toHaveBeenCalled();
  });

  it("executeContextCommand noop does nothing", async () => {
    const d = deps();
    await executeContextCommand({ kind: "noop" }, d);
    expect(d.invoke).not.toHaveBeenCalled();
    expect(d.refresh).not.toHaveBeenCalled();
  });

  it("setPinOrder invokes set_pin_order", async () => {
    const d = deps();
    const items = [{ path: "/a", order: 0 }];
    await setPinOrder(items, d);
    expect(d.invoke).toHaveBeenCalledWith("set_pin_order", { items });
  });

  it("handleRepoContextAction resolves pin from repos", async () => {
    const d = deps();
    const repos = [repo({ path: "/tmp/foo", pinned: false })];
    await handleRepoContextAction(
      { action: "pin", repo_path: "/tmp/foo" },
      repos,
      d,
    );
    expect(d.invoke).toHaveBeenCalledWith("set_pin", {
      repoPath: "/tmp/foo",
      pinned: true,
    });
  });
});
