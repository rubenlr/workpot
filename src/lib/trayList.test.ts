import { describe, expect, it } from "vitest";
import {
  filterAndSectionRepos,
  filterAndSortRepos,
  flatSectioned,
} from "./trayList";
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
    pinned: partial.pinned ?? false,
    pin_order: partial.pin_order ?? null,
    notes: partial.notes ?? null,
    tags: partial.tags ?? [],
    branches: partial.branches ?? [],
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

describe("flatSectioned", () => {
  it("flattens pinned → dirty → recent → rest for keyboard index", () => {
    const sections = {
      pinned: [repo({ name: "pin" })],
      dirty: [repo({ name: "dirty" })],
      recent: [repo({ name: "recent" })],
      rest: [repo({ name: "rest" })],
    };
    expect(flatSectioned(sections).map((r) => r.name)).toEqual([
      "pin",
      "dirty",
      "recent",
      "rest",
    ]);
  });
});

describe("filterAndSectionRepos", () => {
  const config = { maxRecentDays: 14, minRecentCount: 0 };
  const now = 1_000_000;

  const sample = [
    repo({ name: "alpha", is_dirty: false, last_opened_at: now - 1 }),
    repo({
      name: "beta",
      is_dirty: false,
      last_opened_at: now - 2,
      tags: ["backend"],
    }),
    repo({ name: "gamma", pinned: true, pin_order: 0, tags: ["frontend"] }),
  ];

  it("sections all repos with empty query", () => {
    const sections = filterAndSectionRepos(sample, "", config, now);
    const names = [
      ...sections.pinned,
      ...sections.dirty,
      ...sections.recent,
      ...sections.rest,
    ].map((r) => r.name);
    expect(names.sort()).toEqual(["alpha", "beta", "gamma"]);
  });

  it("filters by #backend tag", () => {
    const sections = filterAndSectionRepos(sample, "#backend", config, now);
    const names = [
      ...sections.pinned,
      ...sections.dirty,
      ...sections.recent,
      ...sections.rest,
    ].map((r) => r.name);
    expect(names).toEqual(["beta"]);
  });

  it("requires all active tags (AND)", () => {
    const multi = [
      repo({
        name: "both",
        is_dirty: false,
        last_opened_at: now - 1,
        tags: ["backend", "infra"],
      }),
      repo({
        name: "backend-only",
        is_dirty: false,
        last_opened_at: now - 2,
        tags: ["backend"],
      }),
    ];
    const sections = filterAndSectionRepos(
      multi,
      "#backend #infra",
      config,
      now,
    );
    const names = [
      ...sections.pinned,
      ...sections.dirty,
      ...sections.recent,
      ...sections.rest,
    ].map((r) => r.name);
    expect(names).toEqual(["both"]);
  });

  it("hides pinned repo when tag filter excludes it", () => {
    const sections = filterAndSectionRepos(sample, "#backend", config, now);
    expect(sections.pinned).toHaveLength(0);
  });

  it("combines fuzzy text and tag filter", () => {
    const sections = filterAndSectionRepos(
      sample,
      "alp #backend",
      config,
      now,
    );
    const names = [
      ...sections.pinned,
      ...sections.dirty,
      ...sections.recent,
      ...sections.rest,
    ].map((r) => r.name);
    expect(names).toEqual([]);
  });

  it("uses nowSeconds for recency boundary", () => {
    const now = 10_000_000;
    const inWindow = repo({
      name: "fresh",
      is_dirty: false,
      last_opened_at: now - 86_400,
    });
    const sections = filterAndSectionRepos([inWindow], "", config, now);
    expect(sections.recent.map((r) => r.name)).toEqual(["fresh"]);

    const stale = repo({
      name: "stale",
      is_dirty: false,
      last_opened_at: 0,
    });
    const sectionsLater = filterAndSectionRepos([stale], "", config, now);
    expect(sectionsLater.recent).toHaveLength(0);
    expect(sectionsLater.rest.map((r) => r.name)).toEqual(["stale"]);
  });
});
