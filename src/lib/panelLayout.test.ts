import { describe, expect, it } from "vitest";
import {
  FILTER_BAR_HEIGHT_PX,
  ROW_HEIGHT_PX,
  trayListMaxHeightPx,
} from "./panelLayout";

describe("trayListMaxHeightPx", () => {
  it("uses default 15 rows plus filter bar", () => {
    expect(trayListMaxHeightPx(15)).toBe(15 * ROW_HEIGHT_PX + FILTER_BAR_HEIGHT_PX);
  });

  it("scales with configured max_visible_rows", () => {
    expect(trayListMaxHeightPx(1)).toBe(ROW_HEIGHT_PX + FILTER_BAR_HEIGHT_PX);
    expect(trayListMaxHeightPx(100)).toBe(
      100 * ROW_HEIGHT_PX + FILTER_BAR_HEIGHT_PX,
    );
  });
});
