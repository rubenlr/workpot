export function tagChipTitle(
  hasOnRemove: boolean,
  hasOnFilter: boolean,
): string | undefined {
  if (hasOnRemove && hasOnFilter) {
    return "Click to filter · Cmd+Click to remove";
  }
  if (hasOnRemove) {
    return "Cmd+Click to remove";
  }
  if (hasOnFilter) {
    return "Click to filter";
  }
  return undefined;
}
