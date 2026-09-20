import { describe, expect, it } from "vitest";
import {
  MATERIAL_ICON_FALLBACKS,
  materialIconFallback,
} from "./materialIconFallbacks";

describe("materialIconFallback", () => {
  it("maps known Material icon names to UTF-8 glyphs", () => {
    expect(materialIconFallback("chevron_right")).toBe("›");
    expect(materialIconFallback("south")).toBe("↓");
    expect(materialIconFallback("north")).toBe("↑");
    expect(materialIconFallback("sync")).toBe("↻");
    expect(materialIconFallback("check")).toBe("✓");
    expect(materialIconFallback("search")).toBe("⌕");
    expect(materialIconFallback("folder")).toBe("📁");
    expect(materialIconFallback("arrow_back")).toBe("←");
  });

  it("varies pin/star by filled", () => {
    expect(materialIconFallback("keep", false)).toBe("📍");
    expect(materialIconFallback("keep", true)).toBe("📌");
    expect(materialIconFallback("star", false)).toBe("☆");
    expect(materialIconFallback("star", true)).toBe("★");
  });

  it("uses a bullet for unknown names", () => {
    expect(materialIconFallback("not_a_real_icon")).toBe("•");
  });

  it("covers every fallback table key", () => {
    for (const name of Object.keys(MATERIAL_ICON_FALLBACKS)) {
      expect(materialIconFallback(name).length).toBeGreaterThan(0);
    }
  });
});
