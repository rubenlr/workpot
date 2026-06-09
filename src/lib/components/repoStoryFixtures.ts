import type { RepoDto } from "../types";

/** Storybook-only path prefix — not a real publicly writable directory. */
export const STORY_REPO_PATH_PREFIX = "/Users/storybook/Developer";

export const storyRepoBase: RepoDto = {
  path: `${STORY_REPO_PATH_PREFIX}/workpot-demo`,
  name: "workpot",
  alias: null,
  branch: "main",
  is_dirty: null,
  parent_dir: "~/projects",
  last_opened_at: null,
  git_state_error: null,
  pinned: false,
  pin_order: null,
  notes: null,
  tags: [],
  branches: ["main", "develop"],
};

export function storyRepo(overrides: Partial<RepoDto>): RepoDto {
  return { ...storyRepoBase, ...overrides };
}
