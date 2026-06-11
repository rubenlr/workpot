/** Row height in px — keep in sync with tray list row Tailwind padding. */
export const ROW_HEIGHT_PX = 44;

/** Filter bar chrome height in px. */
export const FILTER_BAR_HEIGHT_PX = 52;

/** Max panel height from `max_visible_rows` config (D-12). */
export function trayListMaxHeightPx(maxVisibleRows: number): number {
  return maxVisibleRows * ROW_HEIGHT_PX + FILTER_BAR_HEIGHT_PX;
}
