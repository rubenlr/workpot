import { fuzzyMatch } from "./fuzzy";
import { parseTagFilter, matchesTags } from "./tagFilter";
import { sectionSort, traySort } from "./sort";
import type { SectionConfig, SectionedRepos } from "./sort";

export function flatSectioned(sectioned: SectionedRepos): RepoDto[] {
  return [
    ...sectioned.pinned,
    ...sectioned.dirty,
    ...sectioned.recent,
    ...sectioned.rest,
  ];
}
import type { RepoDto } from "./types";

/** Client-side tray list: fuzzy filter then dirty-first sort (SRCH-01..03, D-22). */
export function filterAndSortRepos(repos: RepoDto[], query: string): RepoDto[] {
  return repos.filter((r) => fuzzyMatch(query, r)).sort(traySort);
}

/** Fuzzy + tag AND filter, then four-tier section grouping. */
export function filterAndSectionRepos(
  repos: RepoDto[],
  query: string,
  config: SectionConfig,
  nowSeconds?: number,
): SectionedRepos {
  const { baseQuery, activeTags } = parseTagFilter(query);
  const filtered = repos.filter(
    (r) => fuzzyMatch(baseQuery, r) && matchesTags(r, activeTags),
  );
  const now = nowSeconds ?? Math.floor(Date.now() / 1000);
  return sectionSort(filtered, config, now);
}
