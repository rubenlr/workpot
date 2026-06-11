import type { RepoDto } from "$lib/types";

export type Section = "pinned" | "dirty" | "recent" | "rest";

export interface SectionedRepos {
  pinned: RepoDto[];
  dirty: RepoDto[];
  recent: RepoDto[];
  rest: RepoDto[];
}

export interface SectionConfig {
  maxRecentDays: number;
  minRecentCount: number;
}

function dirtyTier(repo: RepoDto): number {
  if (repo.is_dirty === true) {
    return 0;
  }
  if (repo.is_dirty === false) {
    return 1;
  }
  return 2;
}

function byLastOpenedDesc(a: RepoDto, b: RepoDto): number {
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

/** Dirty first, then last_opened_at desc (null last), then name. */
export function traySort(a: RepoDto, b: RepoDto): number {
  const tier = dirtyTier(a) - dirtyTier(b);
  if (tier !== 0) {
    return tier;
  }
  return byLastOpenedDesc(a, b);
}

export function sectionSort(
  repos: RepoDto[],
  config: SectionConfig,
  nowSeconds: number,
): SectionedRepos {
  const pinned = repos
    .filter((r) => r.pinned)
    .sort((a, b) => (a.pin_order ?? 999) - (b.pin_order ?? 999));

  const nonPinned = repos.filter((r) => !r.pinned);
  const dirty = nonPinned
    .filter((r) => r.is_dirty === true)
    .sort(byLastOpenedDesc);

  const nonDirty = nonPinned.filter((r) => r.is_dirty !== true);
  const windowSecs = config.maxRecentDays * 86400;

  const recentByTime = nonDirty
    .filter(
      (r) =>
        r.last_opened_at != null && nowSeconds - r.last_opened_at < windowSecs,
    )
    .sort(byLastOpenedDesc);

  const recent = [...recentByTime];
  if (recent.length < config.minRecentCount) {
    const inRecent = new Set(recent);
    const candidates = nonDirty
      .filter((r) => !inRecent.has(r) && r.last_opened_at != null)
      .sort(byLastOpenedDesc);
    for (const r of candidates) {
      if (recent.length >= config.minRecentCount) {
        break;
      }
      recent.push(r);
    }
  }

  const recentSet = new Set(recent);
  const rest = nonDirty
    .filter((r) => !recentSet.has(r))
    .sort((a, b) => a.name.localeCompare(b.name));

  return { pinned, dirty, recent, rest };
}
