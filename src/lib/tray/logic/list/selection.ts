/** Clamp list selection index to [0, length - 1]; empty list → 0. */
export function clampSelectionIndex(
  selectedIndex: number,
  length: number,
): number {
  if (length === 0) {
    return 0;
  }
  if (selectedIndex < 0) {
    return 0;
  }
  if (selectedIndex >= length) {
    return length - 1;
  }
  return selectedIndex;
}

/** Move selection with wrap-around (ArrowDown/Up, Tab). */
export function moveSelectionIndex(
  selectedIndex: number,
  delta: number,
  length: number,
): number {
  if (length === 0) {
    return 0;
  }
  const clamped = clampSelectionIndex(selectedIndex, length);
  return (clamped + delta + length) % length;
}
