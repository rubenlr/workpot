/** Whether ArrowDown/Up in the filter field should move list selection (UAT #4). */
export function shouldNavigateListOnFilterArrow(
  key: "ArrowDown" | "ArrowUp",
  filterQuery: string,
  selectionStart: number,
  selectionEnd: number,
  valueLength: number,
): boolean {
  if (filterQuery.length === 0) {
    return true;
  }
  if (key === "ArrowDown") {
    return selectionStart === valueLength && selectionEnd === valueLength;
  }
  return selectionStart === 0 && selectionEnd === 0;
}
