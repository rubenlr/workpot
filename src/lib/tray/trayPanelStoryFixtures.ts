import { storyRepo } from "$lib/components/repoStoryFixtures";
import { sectionSort } from "$lib/sort";
import type { SectionedRepos } from "$lib/sort";
import type { RepoDto, TrayConfigDto } from "$lib/types";
import { DEFAULT_SECTION_CFG } from "$lib/openSelection";
import type { TrayListView } from "$lib/listState";

export function storyTrayRepos(): RepoDto[] {
  return [
    storyRepo({
      path: "/tmp/workpot",
      name: "workpot",
      branch: "main",
      is_dirty: false,
      pinned: true,
      pin_order: 0,
      tags: ["rust"],
      last_opened_at: Math.floor(Date.now() / 1000) - 3600,
    }),
    storyRepo({
      path: "/tmp/alpha",
      name: "alpha",
      branch: "feat/ui",
      is_dirty: true,
      pinned: false,
      tags: ["frontend"],
      last_opened_at: Math.floor(Date.now() / 1000) - 7200,
    }),
    storyRepo({
      path: "/tmp/beta",
      name: "beta",
      branch: "develop",
      is_dirty: false,
      pinned: false,
      last_opened_at: Math.floor(Date.now() / 1000) - 86400 * 3,
    }),
    storyRepo({
      path: "/tmp/gamma",
      name: "gamma",
      branch: null,
      is_dirty: null,
      pinned: false,
      last_opened_at: null,
    }),
    storyRepo({
      path: "/tmp/delta",
      name: "delta",
      branch: "release",
      is_dirty: false,
      pinned: false,
      last_opened_at: Math.floor(Date.now() / 1000) - 86400 * 10,
    }),
  ];
}

export function storySectionedRepos(repos = storyTrayRepos()): SectionedRepos {
  return sectionSort(repos, DEFAULT_SECTION_CFG, Math.floor(Date.now() / 1000));
}

export function emptySectionedRepos(): SectionedRepos {
  return { pinned: [], dirty: [], recent: [], rest: [] };
}

export function storyFlatIndexByPath(
  sectioned: SectionedRepos = storySectionedRepos(),
): Map<string, number> {
  const flat = [
    ...sectioned.pinned,
    ...sectioned.dirty,
    ...sectioned.recent,
    ...sectioned.rest,
  ];
  return new Map(flat.map((r, i) => [r.path, i] as const));
}

export function storyTrayConfig(): TrayConfigDto {
  return {
    max_visible_rows: 15,
    max_recent_days: 14,
    min_recent_count: 3,
    max_pinned: 5,
    stale_dirty_days: 7,
  };
}

export const storyListViews = {
  emptyList: { kind: "empty-list" } satisfies TrayListView,
  noMatch: { kind: "no-match" } satisfies TrayListView,
  list: { kind: "list" } satisfies TrayListView,
  error: {
    kind: "error",
    message: "SQLite database is locked",
  } satisfies TrayListView,
};
