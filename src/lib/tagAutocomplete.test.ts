import { describe, expect, it } from "vitest";
import { filterTagsForAutocomplete } from "./tagAutocomplete";

describe("filterTagsForAutocomplete", () => {
  const all = ["backend", "frontend", "infra", "rust"];

  it("returns all tags when prefix and input are empty", () => {
    expect(filterTagsForAutocomplete(all, "", "")).toEqual(all);
  });

  it("filters by external prefix from filter bar (D-10)", () => {
    expect(filterTagsForAutocomplete(all, "back", "")).toEqual(["backend"]);
  });

  it("ANDs prefix with dropdown input filter", () => {
    expect(filterTagsForAutocomplete(all, "f", "fr")).toEqual(["frontend"]);
  });

  it("is case-insensitive", () => {
    expect(filterTagsForAutocomplete(all, "RUST", "")).toEqual(["rust"]);
  });

  it("returns empty when no tag matches", () => {
    expect(filterTagsForAutocomplete(all, "zzz", "")).toEqual([]);
  });

  it("filters by unicode prefix from filter bar", () => {
    const tags = ["后端", "frontend"];
    expect(filterTagsForAutocomplete(tags, "后", "")).toEqual(["后端"]);
  });
});
