import type { BranchListItemDto, RepoDto } from "$lib/types";

/** Storybook-only path prefix — not a real publicly writable directory. */
export const STORY_REPO_PATH_PREFIX = "/Users/storybook/Developer";

export const storyRepoBase: RepoDto = {
  path: `${STORY_REPO_PATH_PREFIX}/workpot-demo`,
  name: "workpot",
  alias: null,
  branch: "master",
  ahead: null,
  behind: null,
  is_dirty: null,
  parent_dir: "~/projects",
  last_opened_at: null,
  git_state_error: null,
  pinned: false,
  pin_order: null,
  notes: null,
  tags: [],
  branches: ["master", "wip", "feat/ui"],
  is_bare: false,
  convert_to: null,
  convert_block_reason: null,
};

export function storyRepo(overrides: Partial<RepoDto>): RepoDto {
  return { ...storyRepoBase, ...overrides };
}

export function storyRepoWithSync(overrides: Partial<RepoDto> = {}): RepoDto {
  return storyRepo({ ahead: 2, behind: 1, ...overrides });
}

export function storyBranches(): BranchListItemDto[] {
  return [
    {
      name: "master",
      checked_out: true,
      tracking: "local_remote",
      ahead: 5,
      behind: 3,
      hidden: false,
    },
    {
      name: "wip",
      checked_out: true,
      tracking: "local_only",
      ahead: null,
      behind: null,
      hidden: false,
    },
    {
      name: "origin/feature/IP-5481-add-logs-and-observability",
      checked_out: false,
      tracking: "remote_only",
      ahead: null,
      behind: null,
      hidden: false,
    },
    {
      name: "feat/ui",
      checked_out: false,
      tracking: "local_remote",
      ahead: 2,
      behind: 1,
      hidden: false,
    },
  ];
}

export function storyCheckoutLocalOnlyBranch(): BranchListItemDto {
  return {
    name: "wip",
    checked_out: true,
    tracking: "local_only",
    ahead: null,
    behind: null,
    hidden: false,
  };
}
