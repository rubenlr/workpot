import type { RepoDto } from "./types";

export type TagFilterParse = {
  baseQuery: string;
  activeTags: string[];
};

type RepoWithTags = RepoDto & { tags?: string[] };

/** Trailing `#partial` in the filter bar (D-10); aligned with parseTagFilter tokens. */
export const TRAILING_TAG_PARTIAL_RE = /#([^\s#]*)$/;

function repoTags(repo: RepoDto): string[] {
  return (repo as RepoWithTags).tags ?? [];
}

export function parseTagFilter(query: string): TagFilterParse {
  const trimmed = query.trim();
  if (!trimmed) {
    return { baseQuery: "", activeTags: [] };
  }

  const tokens = trimmed.split(/\s+/);
  const activeTags = [
    ...new Set(
      tokens
        .filter((t) => t.startsWith("#") && t.length > 1)
        .map((t) => t.slice(1).toLowerCase()),
    ),
  ];
  const baseQuery = tokens
    .filter((t) => !t.startsWith("#"))
    .join(" ")
    .trim();

  return { baseQuery, activeTags };
}

/** AND semantics: every active tag must be present on the repo (case-insensitive). */
export function matchesTags(repo: RepoDto, activeTags: string[]): boolean {
  if (activeTags.length === 0) {
    return true;
  }
  const normalized = new Set(repoTags(repo).map((t) => t.toLowerCase()));
  return activeTags.every((at) => normalized.has(at));
}

/** Idempotent append of `#tag` to the tray filter (T-05-06-05). */
export function appendTagToFilterQuery(
  filterQuery: string,
  tag: string,
): string {
  const normalized = tag.toLowerCase();
  if (parseTagFilter(filterQuery).activeTags.includes(normalized)) {
    return filterQuery;
  }
  return (filterQuery.trimEnd() + " #" + tag).trimStart();
}

/** Replace trailing `#partial` with a completed tag token (D-10 autocomplete). */
export function trailingTagAutocompletePrefix(filterQuery: string): string {
  const m = TRAILING_TAG_PARTIAL_RE.exec(filterQuery);
  return m ? m[1] : "";
}

export function replaceTrailingTagAutocomplete(
  filterQuery: string,
  tag: string,
): string {
  return filterQuery.replace(TRAILING_TAG_PARTIAL_RE, "") + "#" + tag + " ";
}
