import type { ActiveSync, RepoSyncEvent, SyncDirection } from "$lib/types";

export type TrayInvokeFn = (
  cmd: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

export interface TrayRepoSyncDeps {
  invoke: TrayInvokeFn;
  refresh: () => Promise<void>;
  onError: (e: unknown) => void;
  setActiveSync: (sync: ActiveSync | null) => void;
  bumpBranchRevision?: () => void;
}

export async function syncRepoBranch(
  repoPath: string,
  branch: string,
  direction: SyncDirection,
  deps: TrayRepoSyncDeps,
): Promise<void> {
  try {
    await deps.invoke("sync_repo_branch", { repoPath, branch, direction });
  } catch (e) {
    deps.onError(e);
  }
}

export function onRepoSyncStarted(
  payload: RepoSyncEvent,
  setActiveSync: (sync: ActiveSync | null) => void,
): void {
  setActiveSync({
    repoPath: payload.repo_path,
    branch: payload.branch,
    direction: payload.direction,
  });
}

export async function onRepoSyncComplete(
  payload: RepoSyncEvent,
  deps: Pick<
    TrayRepoSyncDeps,
    "setActiveSync" | "refresh" | "bumpBranchRevision"
  >,
): Promise<void> {
  deps.setActiveSync(null);
  await deps.refresh();
  if (payload.repo_path) {
    deps.bumpBranchRevision?.();
  }
}

export function onRepoSyncFailed(
  payload: RepoSyncEvent,
  deps: Pick<TrayRepoSyncDeps, "setActiveSync" | "onError">,
): void {
  deps.setActiveSync(null);
  deps.onError(payload.error ?? "repo sync failed");
}
