import type { RepoDto } from "$lib/types";

export type ContextAction = "pin" | "remove_tag" | "add_tag";

export type ContextCommand =
  | { kind: "toggle_pin"; repoPath: string; pinned: boolean }
  | { kind: "remove_tag"; repoPath: string; tag: string }
  | { kind: "open_detail_tag_focus"; repo: RepoDto }
  | { kind: "noop" };

export function resolveContextAction(
  action: string,
  repo: RepoDto | null,
  repoPath: string,
): ContextCommand {
  if (action === "pin") {
    if (repo) {
      return { kind: "toggle_pin", repoPath, pinned: !repo.pinned };
    }
    return { kind: "noop" };
  }
  if (action === "remove_tag") {
    if (!repo) {
      return { kind: "noop" };
    }
    if (repo.tags.length === 1) {
      return { kind: "remove_tag", repoPath, tag: repo.tags[0] };
    }
    return { kind: "open_detail_tag_focus", repo };
  }
  if (action === "add_tag" && repo) {
    return { kind: "open_detail_tag_focus", repo };
  }
  return { kind: "noop" };
}
