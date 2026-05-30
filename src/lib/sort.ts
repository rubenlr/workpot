import type { RepoDto } from "./types";

function dirtyTier(repo: RepoDto): number {
  if (repo.is_dirty === true) {
    return 0;
  }
  if (repo.is_dirty === false) {
    return 1;
  }
  return 2;
}

/** Dirty first, then last_opened_at desc (null last), then name. */
export function traySort(a: RepoDto, b: RepoDto): number {
  const tier = dirtyTier(a) - dirtyTier(b);
  if (tier !== 0) {
    return tier;
  }

  const aTs = a.last_opened_at;
  const bTs = b.last_opened_at;
  if (aTs != null && bTs != null && aTs !== bTs) {
    return bTs - aTs;
  }
  if (aTs != null && bTs == null) {
    return -1;
  }
  if (aTs == null && bTs != null) {
    return 1;
  }

  return a.name.localeCompare(b.name);
}
