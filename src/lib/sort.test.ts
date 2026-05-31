import { describe, expect, it } from "vitest";
import { sectionSort, traySort } from "./sort";
import type { RepoDto } from "./types";

function repo(partial: Partial<RepoDto> & Pick<RepoDto, "name">): RepoDto {
  return {
    path: `/tmp/${partial.name}`,
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

describe("traySort", () => {
  it("ranks dirty above clean", () => {
    const dirty = repo({ name: "a", is_dirty: true });
    const clean = repo({ name: "b", is_dirty: false });
    expect(traySort(dirty, clean)).toBeLessThan(0);
  });

  it("ranks higher last_opened_at first within same dirty tier", () => {
    const recent = repo({ name: "a", is_dirty: false, last_opened_at: 200 });
    const old = repo({ name: "b", is_dirty: false, last_opened_at: 100 });
    expect(traySort(recent, old)).toBeLessThan(0);
  });

  it("tie-breaks by name", () => {
    const a = repo({ name: "alpha", is_dirty: null, last_opened_at: null });
    const b = repo({ name: "beta", is_dirty: null, last_opened_at: null });
    expect(traySort(a, b)).toBeLessThan(0);
  });

  it("ranks unknown dirty below clean within non-dirty tier", () => {
    const clean = repo({ name: "a", is_dirty: false });
    const unknown = repo({ name: "b", is_dirty: null });
    expect(traySort(clean, unknown)).toBeLessThan(0);
  });

  it("ranks repos with last_opened_at above never-opened in same tier", () => {
    const opened = repo({ name: "b", is_dirty: null, last_opened_at: 1 });
    const never = repo({ name: "a", is_dirty: null, last_opened_at: null });
    expect(traySort(opened, never)).toBeLessThan(0);
  });
});

describe("sectionSort", () => {
  const config = { maxRecentDays: 14, minRecentCount: 3 };
  const now = 1_000_000;

  it("places pinned repos only in pinned section", () => {
    const pinned = repo({ name: "pin", pinned: true, pin_order: 0 });
    const other = repo({ name: "other", is_dirty: false, last_opened_at: now });
    const sections = sectionSort([pinned, other], config, now);
    expect(sections.pinned.map((r) => r.name)).toEqual(["pin"]);
    expect(sections.dirty).toHaveLength(0);
    expect(sections.recent.map((r) => r.name)).toContain("other");
    expect(sections.rest).toHaveLength(0);
  });

  it("places dirty repos in dirty, not recent", () => {
    const dirty = repo({
      name: "dirty",
      is_dirty: true,
      last_opened_at: now - 10,
    });
    const sections = sectionSort([dirty], config, now);
    expect(sections.dirty.map((r) => r.name)).toEqual(["dirty"]);
    expect(sections.recent).toHaveLength(0);
  });

  it("pads recent to minRecentCount from outside window", () => {
    const inWindow = repo({
      name: "a",
      is_dirty: false,
      last_opened_at: now - 100,
    });
    const inWindow2 = repo({
      name: "b",
      is_dirty: false,
      last_opened_at: now - 200,
    });
    const old = repo({
      name: "c",
      is_dirty: false,
      last_opened_at: now - 999_999,
    });
    const sections = sectionSort([inWindow, inWindow2, old], config, now);
    expect(sections.recent).toHaveLength(3);
    expect(sections.recent.map((r) => r.name).sort()).toEqual(["a", "b", "c"]);
    expect(sections.rest).toHaveLength(0);
  });

  it("sends never-opened repos to rest", () => {
    const never = repo({
      name: "never",
      is_dirty: false,
      last_opened_at: null,
    });
    const sections = sectionSort(
      [never],
      { maxRecentDays: 14, minRecentCount: 0 },
      now,
    );
    expect(sections.rest.map((r) => r.name)).toEqual(["never"]);
    expect(sections.recent).toHaveLength(0);
  });

  it("does not pad recent with never-opened repos (D-21)", () => {
    const never = [
      repo({ name: "a", is_dirty: false, last_opened_at: null }),
      repo({ name: "b", is_dirty: false, last_opened_at: null }),
      repo({ name: "c", is_dirty: false, last_opened_at: null }),
    ];
    const sections = sectionSort(never, config, now);
    expect(sections.recent).toHaveLength(0);
    expect(sections.rest.map((r) => r.name).sort()).toEqual(["a", "b", "c"]);
  });

  it("partitions every repo exactly once", () => {
    const repos = [
      repo({ name: "p", pinned: true, pin_order: 0 }),
      repo({ name: "d", is_dirty: true }),
      repo({ name: "r", is_dirty: false, last_opened_at: now - 1 }),
      repo({ name: "x", is_dirty: false, last_opened_at: null }),
    ];
    const sections = sectionSort(repos, config, now);
    const all = [
      ...sections.pinned,
      ...sections.dirty,
      ...sections.recent,
      ...sections.rest,
    ];
    expect(all).toHaveLength(repos.length);
    expect(new Set(all.map((r) => r.path)).size).toBe(repos.length);
  });

  it("sorts pinned by pin_order", () => {
    const a = repo({ name: "a", pinned: true, pin_order: 2 });
    const b = repo({ name: "b", pinned: true, pin_order: 0 });
    const sections = sectionSort([a, b], config, now);
    expect(sections.pinned.map((r) => r.name)).toEqual(["b", "a"]);
  });
});
