import { describe, expect, it } from "vitest";
import { clampSelectionIndex, moveSelectionIndex } from "./selection";

describe("clampSelectionIndex", () => {
  it("returns 0 for empty list", () => {
    expect(clampSelectionIndex(3, 0)).toBe(0);
  });

  it("clamps high index", () => {
    expect(clampSelectionIndex(5, 3)).toBe(2);
  });

  it("clamps negative index", () => {
    expect(clampSelectionIndex(-1, 3)).toBe(0);
  });
});

describe("moveSelectionIndex", () => {
  it("returns 0 for empty list", () => {
    expect(moveSelectionIndex(0, 1, 0)).toBe(0);
  });

  it("wraps forward from last row", () => {
    expect(moveSelectionIndex(2, 1, 3)).toBe(0);
  });

  it("wraps backward from first row", () => {
    expect(moveSelectionIndex(0, -1, 3)).toBe(2);
  });

  it("moves down without wrap in middle", () => {
    expect(moveSelectionIndex(1, 1, 3)).toBe(2);
  });
});
