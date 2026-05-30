import { describe, expect, it } from "vitest";
import { shouldNavigateListOnFilterArrow } from "./filterNavigation";

describe("shouldNavigateListOnFilterArrow", () => {
  it("always navigates when filter is empty", () => {
    expect(
      shouldNavigateListOnFilterArrow("ArrowDown", "", 0, 0, 0),
    ).toBe(true);
    expect(
      shouldNavigateListOnFilterArrow("ArrowUp", "", 3, 3, 3),
    ).toBe(true);
  });

  it("navigates ArrowDown only at end of input", () => {
    expect(
      shouldNavigateListOnFilterArrow("ArrowDown", "wp", 2, 2, 2),
    ).toBe(true);
    expect(
      shouldNavigateListOnFilterArrow("ArrowDown", "wp", 0, 0, 2),
    ).toBe(false);
    expect(
      shouldNavigateListOnFilterArrow("ArrowDown", "wp", 1, 2, 2),
    ).toBe(false);
  });

  it("navigates ArrowUp only at start of input", () => {
    expect(
      shouldNavigateListOnFilterArrow("ArrowUp", "wp", 0, 0, 2),
    ).toBe(true);
    expect(
      shouldNavigateListOnFilterArrow("ArrowUp", "wp", 2, 2, 2),
    ).toBe(false);
  });
});
