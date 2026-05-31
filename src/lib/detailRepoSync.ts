import type { RepoDto } from "./types";

/** Keep detail pane in sync after `repos` reload (CR-02). */
export function resyncDetailRepo(
  repos: RepoDto[],
  currentPath: string | null | undefined,
): RepoDto | null {
  if (!currentPath) {
    return null;
  }
  return repos.find((r) => r.path === currentPath) ?? null;
}

/** Resync only when the detail pane is still open after reload (WR-01). */
export function resyncDetailIfOpen(
  repos: RepoDto[],
  detailRepo: RepoDto | null,
): RepoDto | null {
  if (detailRepo === null) {
    return null;
  }
  return resyncDetailRepo(repos, detailRepo.path);
}
