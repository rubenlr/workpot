import { fuzzyMatch } from "./fuzzy";
import { traySort } from "./sort";
import type { RepoDto } from "./types";

/** Client-side tray list: fuzzy filter then dirty-first sort (SRCH-01..03, D-22). */
export function filterAndSortRepos(repos: RepoDto[], query: string): RepoDto[] {
  return repos.filter((r) => fuzzyMatch(query, r)).sort(traySort);
}
