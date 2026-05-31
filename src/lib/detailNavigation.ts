/** Keys that still reach list navigation while the detail pane is open (WR-05). */
export function shouldSuppressTrayListKeyWhenDetailOpen(
  key: string,
  metaKey: boolean,
): boolean {
  if (metaKey && (key === "r" || key === "R")) {
    return false;
  }
  if (key === "ArrowLeft" || key === "Escape" || key === "ArrowRight") {
    return false;
  }
  return true;
}
