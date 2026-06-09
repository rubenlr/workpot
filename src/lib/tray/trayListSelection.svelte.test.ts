import { describe, expect, it } from "vitest";
import { DEFAULT_SECTION_CFG } from "$lib/openSelection";
import type { RepoDto } from "$lib/types";
import { createTrayListSelection } from "./trayListSelection.svelte";

function repo(name: string, overrides: Partial<RepoDto> = {}): RepoDto {
  const path = `/tmp/${name}`;
  return {
    path,
    name,
    alias: null,
    branch: null,
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

function createSelection(repos: RepoDto[], error: string | null = null) {
  return createTrayListSelection({
    getRepos: () => repos,
    getSectionCfg: () => DEFAULT_SECTION_CFG,
    getError: () => error,
  });
}

describe("createTrayListSelection", () => {
  it("filterQuery change resets selectedIndex to 0", () => {
    const list = createSelection([repo("a"), repo("b")]);
    list.selectedIndex = 1;
    list.filterQuery = "a";
    expect(list.selectedIndex).toBe(0);
  });

  it("moveSelection wraps around list bounds", () => {
    const list = createSelection([repo("a"), repo("b"), repo("c")]);
    expect(list.selectedIndex).toBe(0);
    list.moveSelection(1);
    expect(list.selectedIndex).toBe(1);
    list.moveSelection(2);
    expect(list.selectedIndex).toBe(0);
    list.moveSelection(-1);
    expect(list.selectedIndex).toBe(2);
  });

  it("getSelectedRepo returns repo at clamped index", () => {
    const a = repo("alpha");
    const b = repo("beta");
    const list = createSelection([a, b]);
    list.selectedIndex = 1;
    expect(list.getSelectedRepo()).toEqual(b);
  });

  it("appendTagFilter and onTagAutocompleteSelect update filterQuery", () => {
    const list = createSelection([repo("a", { tags: ["work"] })]);
    list.filterQuery = "alpha #w";
    list.onTagAutocompleteSelect("work");
    expect(list.filterQuery).toBe("alpha #work ");

    list.filterQuery = "";
    list.appendTagFilter("personal");
    expect(list.filterQuery).toBe("#personal");
  });

  it("listView reflects error, empty, no-match, and list states", () => {
    const empty = createSelection([]);
    expect(empty.listView).toEqual({ kind: "empty-list" });

    const err = createSelection([], "load failed");
    expect(err.listView).toEqual({ kind: "error", message: "load failed" });

    const noMatch = createSelection([repo("a")]);
    noMatch.filterQuery = "zzz";
    expect(noMatch.listView).toEqual({ kind: "no-match" });

    const visible = createSelection([repo("a")]);
    expect(visible.listView).toEqual({ kind: "list" });
  });

  it("flatIndexByPath maps repo paths to flat indices", () => {
    const a = repo("a");
    const b = repo("b");
    const list = createSelection([a, b]);
    expect(list.flatIndexByPath.get(a.path)).toBe(0);
    expect(list.flatIndexByPath.get(b.path)).toBe(1);
  });
});
