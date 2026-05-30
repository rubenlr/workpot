import { describe, expect, it } from "vitest";
import { filterAndSortRepos } from "./trayList";
import type { RepoDto } from "./types";

function repo(partial: Partial<RepoDto> & Pick<RepoDto, "name">): RepoDto {
  return {
    path: partial.path ?? `/tmp/${partial.name}`,
    name: partial.name,
    branch: partial.branch ?? null,
    is_dirty: partial.is_dirty ?? null,
    parent_dir: "",
    last_opened_at: partial.last_opened_at ?? null,
    git_state_error: null,
  };
}

describe("filterAndSortRepos", () => {
  const repos = [
    repo({ name: "clean-old", is_dirty: false, last_opened_at: 10 }),
    repo({ name: "dirty", is_dirty: true, last_opened_at: 1 }),
    repo({ name: "workpot", is_dirty: false, last_opened_at: 200 }),
  ];

  it("sorts dirty first with empty query", () => {
    const out = filterAndSortRepos(repos, "");
    expect(out.map((r) => r.name)).toEqual(["dirty", "workpot", "clean-old"]);
  });

  it("filters by fuzzy query", () => {
    const out = filterAndSortRepos(repos, "wp");
    expect(out.map((r) => r.name)).toEqual(["workpot"]);
  });

  it("returns empty when nothing matches", () => {
    expect(filterAndSortRepos(repos, "zzz")).toEqual([]);
  });
});
