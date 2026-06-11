import type { BranchListItemDto } from "$lib/types";

/** Storybook stub — no Tauri runtime. */
export async function invoke(cmd: string): Promise<unknown> {
  if (cmd === "list_branches") {
    const branches: BranchListItemDto[] = [
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
    return branches;
  }
  return undefined;
}

export async function listen(): Promise<() => void> {
  return () => {};
}
