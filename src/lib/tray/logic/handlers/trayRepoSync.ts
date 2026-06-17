import type { ActiveSync, RepoSyncEvent, SyncDirection } from "$lib/types";
import { trayTrace } from "./trayTrace";

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

const PRE_START_SYNC_ERRORS = [
  "a repo sync is already in progress",
  "branch must not be empty",
  "invalid sync direction",
] as const;

function isPreStartSyncError(message: string): boolean {
  return PRE_START_SYNC_ERRORS.some((fragment) => message.includes(fragment));
}

export async function restoreRepoSyncStatus(
  invoke: TrayInvokeFn,
  setActiveSync: (sync: ActiveSync | null) => void,
): Promise<void> {
  const status = (await invoke("get_repo_sync_status")) as RepoSyncEvent | null;
  if (!status?.repo_path || !status.branch) {
    return;
  }
  setActiveSync({
    repoPath: status.repo_path,
    branch: status.branch,
    direction: status.direction,
  });
}

export async function syncRepoBranch(
  repoPath: string,
  branch: string,
  direction: SyncDirection,
  deps: TrayRepoSyncDeps,
): Promise<void> {
  trayTrace("invoke sync_repo_branch", { repoPath, branch, direction });
  try {
    await deps.invoke("sync_repo_branch", { repoPath, branch, direction });
    trayTrace("sync_repo_branch ok", { repoPath, branch, direction });
  } catch (e) {
    trayTrace("sync_repo_branch failed", e);
    const message = String(e);
    if (isPreStartSyncError(message)) {
      deps.onError(e);
    }
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
