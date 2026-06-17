import { describe, expect, it } from "vitest";
import {
  appendTagToFilterQuery,
  matchesTags,
  parseTagFilter,
  replaceTrailingTagAutocomplete,
  trailingTagAutocompletePrefix,
} from "./tagFilter";
import type { RepoDto } from "./types";

type RepoWithTags = RepoDto & { tags?: string[] };

function repo(
  partial: Partial<RepoWithTags> & Pick<RepoDto, "name">,
): RepoWithTags {
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
    git_state_error: null,
    pinned: partial.pinned ?? false,
    pin_order: partial.pin_order ?? null,
    notes: partial.notes ?? null,
    tags: partial.tags ?? [],
    branches: partial.branches ?? [],
    is_bare: partial.is_bare ?? false,
    convert_to: partial.convert_to ?? null,
    convert_block_reason: partial.convert_block_reason ?? null,
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

  it("deduplicates repeated tag tokens", () => {
    expect(parseTagFilter("x #foo #foo")).toEqual({
      baseQuery: "x",
      activeTags: ["foo"],
    });
  });

  it("parses unicode tag tokens", () => {
    expect(parseTagFilter("find #后端")).toEqual({
      baseQuery: "find",
      activeTags: ["后端"],
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

  it("matches emoji tags", () => {
    const emoji = "🏷️";
    const r = repo({ name: "a", tags: [emoji] });
    expect(matchesTags(r, [emoji])).toBe(true);
  });
});

describe("appendTagToFilterQuery", () => {
  it("appends #tag token with spacing", () => {
    expect(appendTagToFilterQuery("alpha", "backend")).toBe("alpha #backend");
  });

  it("does not duplicate an existing token", () => {
    expect(appendTagToFilterQuery("x #backend", "backend")).toBe("x #backend");
  });

  it("does not treat a tag name as duplicate when it is only a substring", () => {
    expect(appendTagToFilterQuery("x #foobar", "foo")).toBe("x #foobar #foo");
  });

  it("does not duplicate when filter tag differs only by case", () => {
    expect(appendTagToFilterQuery("x #Backend", "backend")).toBe("x #Backend");
    expect(appendTagToFilterQuery("x #backend", "Backend")).toBe("x #backend");
  });

  it("does not duplicate an existing unicode tag token", () => {
    const tag = "后端";
    expect(appendTagToFilterQuery(`x #${tag}`, tag)).toBe(`x #${tag}`);
  });

  it("appends a distinct unicode tag", () => {
    expect(appendTagToFilterQuery("find", "后端")).toBe("find #后端");
  });
});

describe("replaceTrailingTagAutocomplete", () => {
  it("replaces trailing #partial with completed tag", () => {
    expect(replaceTrailingTagAutocomplete("find #ba", "backend")).toBe(
      "find #backend ",
    );
  });

  it("replaces trailing partial when the tag contains a hyphen", () => {
    expect(replaceTrailingTagAutocomplete("find #my-", "my-tag")).toBe(
      "find #my-tag ",
    );
  });

  it("replaces lone trailing hash with completed tag", () => {
    expect(replaceTrailingTagAutocomplete("find #", "infra")).toBe(
      "find #infra ",
    );
  });

  it("replaces trailing partial when the tag contains emoji", () => {
    const emoji = "🏷️";
    expect(
      replaceTrailingTagAutocomplete(`find #${emoji.slice(0, 1)}`, emoji),
    ).toBe(`find #${emoji} `);
  });
});

describe("trailingTagAutocompletePrefix", () => {
  it("captures unicode partial after trailing hash", () => {
    expect(trailingTagAutocompletePrefix("find #后")).toBe("后");
  });

  it("returns empty when hash is not trailing", () => {
    expect(trailingTagAutocompletePrefix("find #foo bar")).toBe("");
  });

  it("captures emoji partial after trailing hash", () => {
    const emoji = "🏷️";
    expect(trailingTagAutocompletePrefix(`find #${emoji.slice(0, 1)}`)).toBe(
      emoji.slice(0, 1),
    );
  });
});
