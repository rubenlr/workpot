import type { BranchListItemDto } from "$lib/types";
import {
  storyTrayConfig,
  storyTrayRepos,
} from "$lib/tray/storybook/trayPanelStoryFixtures";

function storyAllTags(): string[] {
  const tags = new Set<string>();
  for (const repo of storyTrayRepos()) {
    for (const tag of repo.tags) {
      tags.add(tag);
    }
  }
  return [...tags].sort((a, b) => a.localeCompare(b));
}

const storyBranches: BranchListItemDto[] = [
  {
    name: "main",
    presence: "checkout",
    ahead: 0,
    behind: 0,
  },
  {
    name: "develop",
    presence: "local_remote",
    ahead: 2,
    behind: 0,
  },
  {
    name: "wip",
    presence: "local_only",
    ahead: null,
    behind: null,
  },
  {
    name: "origin-only",
    presence: "remote_only",
    ahead: null,
    behind: null,
  },
];

/** Storybook stub — no Tauri runtime. */
export async function invoke(cmd: string, _args?: unknown): Promise<unknown> {
  switch (cmd) {
    case "list_repos":
      return storyTrayRepos();
    case "list_all_tags":
      return storyAllTags();
    case "get_tray_config":
      return storyTrayConfig();
    case "list_branches":
      return storyBranches;
    case "refresh_index":
    case "refresh_all_git_state":
    case "open_in_cursor":
    case "show_repo_context_menu":
    case "set_pin":
    case "set_pin_order":
    case "remove_tag":
    case "add_tag":
    case "set_notes":
    case "checkout_repo_branch":
    case "get_repo_sync_status":
    case "sync_repo_branch":
      return undefined;
    default:
      return undefined;
  }
}
