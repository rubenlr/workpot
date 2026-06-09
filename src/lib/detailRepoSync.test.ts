import { describe, expect, it } from "vitest";
import { resyncDetailIfOpen, resyncDetailRepo } from "./detailRepoSync";
import type { RepoDto } from "./types";

function repo(name: string, tags: string[] = []): RepoDto {
  return {
    name,
    alias: null,
    path: `/tmp/${name}`,
    branch: null,
    is_dirty: null,
    parent_dir: "",
    last_opened_at: null,
    git_state_error: null,
    pinned: false,
    pin_order: null,
    notes: null,
    tags,
    branches: [],
  };
}

describe("resyncDetailRepo", () => {
  it("returns updated row when tags change after reload", () => {
    const before = repo("alpha", ["old"]);
    const after = [repo("alpha", ["new", "extra"])];
    const synced = resyncDetailRepo(after, before.path);
    expect(synced?.path).toBe(before.path);
    expect(synced?.tags).toEqual(["new", "extra"]);
  });

  it("returns null when repo was removed", () => {
    expect(resyncDetailRepo([], "/tmp/alpha")).toBeNull();
  });

  it("returns null when no path is open", () => {
    expect(resyncDetailRepo([repo("alpha")], null)).toBeNull();
    expect(resyncDetailRepo([repo("alpha")], undefined)).toBeNull();
  });

  it("returns null when path no longer exists in repos", () => {
    expect(resyncDetailRepo([repo("other")], "/tmp/alpha")).toBeNull();
  });
});

describe("resyncDetailIfOpen", () => {
  it("returns null when detail pane was closed before resync (WR-01)", () => {
    const repos = [repo("alpha", ["fresh"])];
    expect(resyncDetailIfOpen(repos, null)).toBeNull();
  });

  it("resyncs when detail pane is still open", () => {
    const open = repo("alpha", ["old"]);
    const repos = [repo("alpha", ["new"])];
    const synced = resyncDetailIfOpen(repos, open);
    expect(synced?.path).toBe(open.path);
    expect(synced?.tags).toEqual(["new"]);
  });
});
