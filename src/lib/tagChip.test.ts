import { describe, expect, it } from "vitest";
import { tagChipTitle } from "./tagChip";

describe("tagChipTitle", () => {
  it("describes filter + remove when both handlers exist", () => {
    expect(tagChipTitle(true, true)).toBe(
      "Click to filter · Cmd+Click to remove",
    );
  });

  it("describes remove-only in detail pane", () => {
    expect(tagChipTitle(true, false)).toBe("Cmd+Click to remove");
  });

  it("describes filter-only on repo row", () => {
    expect(tagChipTitle(false, true)).toBe("Click to filter");
  });

  it("returns undefined when no handlers", () => {
    expect(tagChipTitle(false, false)).toBeUndefined();
  });
});
