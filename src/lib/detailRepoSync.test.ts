import { describe, expect, it } from "vitest";
import { resyncDetailRepo } from "./detailRepoSync";
import type { RepoDto } from "./types";

function repo(name: string, tags: string[] = []): RepoDto {
  return {
    name,
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
    expect(resyncDetailRepo(after, before.path)).toEqual(after[0]);
    expect(resyncDetailRepo(after, before.path)?.tags).toEqual(["new", "extra"]);
  });

  it("returns null when repo was removed", () => {
    expect(resyncDetailRepo([], "/tmp/alpha")).toBeNull();
  });

  it("returns null when no path is open", () => {
    expect(resyncDetailRepo([repo("alpha")], null)).toBeNull();
    expect(resyncDetailRepo([repo("alpha")], undefined)).toBeNull();
  });
});
