import type { RepoDto } from "../types";

export const storyRepoBase: RepoDto = {
  path: "/tmp/workpot-demo",
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
