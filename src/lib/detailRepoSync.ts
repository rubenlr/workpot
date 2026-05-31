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
