import { isCloned } from "$lib/types";
import type { RepoDto } from "$lib/types";

export type Section = "pinned" | "dirty" | "recent" | "rest" | "remote";

export interface SectionedRepos {
  pinned: RepoDto[];
  dirty: RepoDto[];
  recent: RepoDto[];
  rest: RepoDto[];
  /** Indexed remote repos that are not cloned yet. */
  remote: RepoDto[];
}

export interface SectionConfig {
  maxRecentDays: number;
  minRecentCount: number;
}

/** `pin_order` fallback that sorts an unordered pinned repo after every ordered one. */
const UNORDERED_PIN_ORDER = 999;
const SECS_PER_DAY = 86400;

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

/** Recent section: opened within the window, padded up to `minRecentCount` with the next most recent. */
function pickRecent(
  nonDirty: RepoDto[],
  config: SectionConfig,
  nowSeconds: number,
): RepoDto[] {
  const windowSecs = config.maxRecentDays * SECS_PER_DAY;
  const recent = nonDirty
    .filter(
      (r) =>
        r.last_opened_at != null && nowSeconds - r.last_opened_at < windowSecs,
    )
    .sort(byLastOpenedDesc);
  if (recent.length >= config.minRecentCount) {
    return recent;
  }
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
  return recent;
}

export function sectionSort(
  repos: RepoDto[],
  config: SectionConfig,
  nowSeconds: number,
): SectionedRepos {
  const remote = repos
    .filter((r) => !isCloned(r))
    .sort((a, b) =>
      (a.remote_full_path ?? a.name).localeCompare(
        b.remote_full_path ?? b.name,
      ),
    );
  const local = repos.filter(isCloned);
  const pinned = local
    .filter((r) => r.pinned)
    .sort(
      (a, b) =>
        (a.pin_order ?? UNORDERED_PIN_ORDER) -
        (b.pin_order ?? UNORDERED_PIN_ORDER),
    );

  const nonPinned = local.filter((r) => !r.pinned);
  const dirty = nonPinned
    .filter((r) => r.is_dirty === true)
    .sort(byLastOpenedDesc);

  const nonDirty = nonPinned.filter((r) => r.is_dirty !== true);
  const recent = pickRecent(nonDirty, config, nowSeconds);

  const recentSet = new Set(recent);
  const rest = nonDirty
    .filter((r) => !recentSet.has(r))
    .sort((a, b) => a.name.localeCompare(b.name));

  return { pinned, dirty, recent, rest, remote };
}
