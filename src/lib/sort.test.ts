import { describe, expect, it } from "vitest";
import { traySort } from "./sort";
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
