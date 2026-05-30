import { filterAndSortRepos } from "./trayList";
import type { RepoDto } from "./types";

/** Restore list selection after Cmd+Enter background open (D-36). */
export function selectionIndexAfterBackgroundOpen(
  repos: RepoDto[],
  query: string,
  openedPath: string,
): number {
  const idx = filterAndSortRepos(repos, query).findIndex(
    (r) => r.path === openedPath,
  );
  return idx >= 0 ? idx : 0;
}
