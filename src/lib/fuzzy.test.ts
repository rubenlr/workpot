import { describe, expect, it } from "vitest";
import { fuzzyMatch, fuzzyScore } from "./fuzzy";
import type { RepoDto } from "./types";

function repo(partial: Partial<RepoDto> & Pick<RepoDto, "name">): RepoDto {
  return {
    path: partial.path ?? `/Users/me/c/${partial.name}`,
    name: partial.name,
    alias: partial.alias ?? null,
    branch: partial.branch ?? "main",
    ahead: null,
    behind: null,
    is_dirty: partial.is_dirty ?? null,
    parent_dir: "",
    last_opened_at: partial.last_opened_at ?? null,
    git_state_error: partial.git_state_error ?? null,
    pinned: partial.pinned ?? false,
    pin_order: partial.pin_order ?? null,
    notes: partial.notes ?? null,
    tags: partial.tags ?? [],
    branches: partial.branches ?? [],
    is_bare: partial.is_bare ?? false,
    convert_to: partial.convert_to ?? null,
  };
}

describe("fuzzyMatch", () => {
  it('matches "wp" against workpot name', () => {
    const r = repo({ name: "workpot" });
    expect(fuzzyMatch("wp", r)).toBe(true);
    expect(fuzzyScore("wp", r)).toBeGreaterThan(0);
  });

  it('matches branch "main"', () => {
    const r = repo({ name: "other", branch: "main" });
    expect(fuzzyMatch("main", r)).toBe(true);
  });

  it("returns all repos for empty query via score", () => {
    const r = repo({ name: "x" });
    expect(fuzzyMatch("", r)).toBe(true);
  });

  it("rejects query over 256 chars", () => {
    const r = repo({ name: "workpot" });
    expect(fuzzyMatch("x".repeat(257), r)).toBe(false);
  });

  it("matches path segment", () => {
    const r = repo({
      name: "other",
      path: "/Users/me/c/workpot",
    });
    expect(fuzzyMatch("workpot", r)).toBe(true);
  });

  it("trims query whitespace", () => {
    const r = repo({ name: "workpot" });
    expect(fuzzyMatch("  wp  ", r)).toBe(true);
  });

  it("returns false when no field matches", () => {
    const r = repo({ name: "alpha", branch: "main" });
    expect(fuzzyMatch("zzz", r)).toBe(false);
  });

  it("scores name prefix higher than path-only subsequence", () => {
    const byName = repo({ name: "workpot", path: "/tmp/x" });
    const byPath = repo({
      name: "x",
      path: "/tmp/workpot-extra",
    });
    expect(fuzzyScore("work", byName)).toBeGreaterThan(
      fuzzyScore("work", byPath),
    );
  });

  it("matches notes text", () => {
    const r = repo({
      name: "x",
      notes: "deployment pipeline",
    });
    expect(fuzzyMatch("pipeline", r)).toBe(true);
  });

  it("matches tag text", () => {
    const r = repo({ name: "x", tags: ["backend"] });
    expect(fuzzyMatch("backend", r)).toBe(true);
  });

  it("matches alias when name does not", () => {
    const r = repo({ name: "workpot-core", alias: "wp" });
    expect(fuzzyMatch("wp", r)).toBe(true);
    expect(fuzzyMatch("wp", repo({ name: "alpha", alias: null }))).toBe(false);
  });

  it("scores alias prefix above path-only match", () => {
    const byAlias = repo({ name: "x", alias: "workpot", path: "/tmp/a" });
    const byPath = repo({
      name: "y",
      alias: null,
      path: "/tmp/workpot-extra",
    });
    expect(fuzzyScore("work", byAlias)).toBeGreaterThan(
      fuzzyScore("work", byPath),
    );
  });

  it("does not match unrelated query on note-only repo", () => {
    const r = repo({
      name: "x",
      branch: null,
      notes: "deployment pipeline",
    });
    expect(fuzzyMatch("zzz", r)).toBe(false);
    expect(fuzzyScore("zzz", r)).toBe(0);
  });
});
