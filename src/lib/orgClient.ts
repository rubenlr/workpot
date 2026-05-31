/** Case-sensitive match against repo.tags (same as DB collation). */
export function tagAlreadyOnRepo(tag: string, repoTags: string[]): boolean {
  const trimmed = tag.trim();
  if (!trimmed) {
    return false;
  }
  return repoTags.includes(trimmed);
}

/** Client-side tag add validation before IPC (DetailPane). */
export function clientTagAddError(raw: string): string | null {
  const tag = raw.trim();
  if (!tag) {
    return null;
  }
  if (tag.startsWith("#")) {
    return "Tag cannot start with #";
  }
  return null;
}

/** True when blur-save should call set_notes. */
export function shouldSaveNotes(
  notesValue: string,
  repoNotes: string | null | undefined,
): boolean {
  const trimmed = notesValue.trim() || null;
  const previous = repoNotes?.trim() || null;
  return trimmed !== previous;
}
