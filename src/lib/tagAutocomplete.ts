/** Prefix + input filter for tag dropdown (D-10). */
export function filterTagsForAutocomplete(
  allTags: string[],
  prefix: string,
  inputValue: string,
): string[] {
  let tags = allTags;
  if (prefix.length > 0) {
    const p = prefix.toLowerCase();
    tags = tags.filter((t) => t.toLowerCase().startsWith(p));
  }
  if (inputValue.length > 0) {
    const q = inputValue.toLowerCase();
    tags = tags.filter((t) => t.toLowerCase().startsWith(q));
  }
  return tags;
}
