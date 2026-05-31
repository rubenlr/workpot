import { describe, expect, it } from "vitest";
import { matchesTags, parseTagFilter } from "./tagFilter";
import type { RepoDto } from "./types";

type RepoWithTags = RepoDto & { tags?: string[] };

function repo(
  partial: Partial<RepoWithTags> & Pick<RepoDto, "name">,
): RepoWithTags {
  return {
    path: partial.path ?? `/Users/me/c/${partial.name}`,
    name: partial.name,
    branch: partial.branch ?? "main",
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

describe("parseTagFilter", () => {
  it("splits base query and hash tags", () => {
    expect(parseTagFilter("hello #backend #infra")).toEqual({
      baseQuery: "hello",
      activeTags: ["backend", "infra"],
    });
  });

  it("handles tag-only query", () => {
    expect(parseTagFilter("#backend")).toEqual({
      baseQuery: "",
      activeTags: ["backend"],
    });
  });

  it("returns empty tags for plain text", () => {
    expect(parseTagFilter("hello world")).toEqual({
      baseQuery: "hello world",
      activeTags: [],
    });
  });

  it("handles empty string", () => {
    expect(parseTagFilter("")).toEqual({
      baseQuery: "",
      activeTags: [],
    });
  });

  it("lowercases tag tokens", () => {
    expect(parseTagFilter("#Backend")).toEqual({
      baseQuery: "",
      activeTags: ["backend"],
    });
  });

  it("ignores lone hash", () => {
    expect(parseTagFilter("hello #")).toEqual({
      baseQuery: "hello",
      activeTags: [],
    });
  });
});

describe("matchesTags", () => {
  it("matches tags case-insensitively", () => {
    const r = repo({ name: "a", tags: ["Backend"] });
    expect(matchesTags(r, ["backend"])).toBe(true);
  });

  it("requires all active tags (AND)", () => {
    const r = repo({ name: "a", tags: ["backend", "infra"] });
    expect(matchesTags(r, ["backend", "infra"])).toBe(true);
    expect(matchesTags(r, ["backend", "missing"])).toBe(false);
  });

  it("passes when no active tags", () => {
    const r = repo({ name: "a", tags: [] });
    expect(matchesTags(r, [])).toBe(true);
  });
});
