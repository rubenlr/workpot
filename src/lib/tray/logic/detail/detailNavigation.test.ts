import { describe, expect, it } from "vitest";
import { shouldSuppressTrayListKeyWhenDetailOpen } from "./detailNavigation";

describe("shouldSuppressTrayListKeyWhenDetailOpen", () => {
  it("allows close and detail-switch keys", () => {
    expect(shouldSuppressTrayListKeyWhenDetailOpen("ArrowLeft", false)).toBe(
      false,
    );
    expect(shouldSuppressTrayListKeyWhenDetailOpen("Escape", false)).toBe(
      false,
    );
    expect(shouldSuppressTrayListKeyWhenDetailOpen("ArrowRight", false)).toBe(
      false,
    );
  });

  it("allows Cmd+R refresh", () => {
    expect(shouldSuppressTrayListKeyWhenDetailOpen("r", true)).toBe(false);
    expect(shouldSuppressTrayListKeyWhenDetailOpen("R", true)).toBe(false);
  });

  it("suppresses list navigation keys", () => {
    expect(shouldSuppressTrayListKeyWhenDetailOpen("ArrowDown", false)).toBe(
      true,
    );
    expect(shouldSuppressTrayListKeyWhenDetailOpen("ArrowUp", false)).toBe(
      true,
    );
    expect(shouldSuppressTrayListKeyWhenDetailOpen("Enter", false)).toBe(true);
    expect(shouldSuppressTrayListKeyWhenDetailOpen("Tab", false)).toBe(true);
  });
});
