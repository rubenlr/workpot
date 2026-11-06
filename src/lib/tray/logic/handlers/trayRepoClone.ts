import type { RepoCloneEvent, RepoDto } from "$lib/types";
import { trayTrace } from "./trayTrace";
import type { TrayInvokeFn } from "./trayRepoConvert";

export interface TrayRepoCloneDeps {
  invoke: TrayInvokeFn;
  refresh: () => Promise<void>;
  onError: (e: unknown) => void;
  addCloning: (repoPath: string) => void;
  removeCloning: (repoPath: string) => void;
  /** Called after a clone-and-open finished and the IDE launched. */
  onOpened?: () => void;
}

const PRE_START_CLONE_ERRORS = ["already being cloned"] as const;

function isPreStartCloneError(message: string): boolean {
  return PRE_START_CLONE_ERRORS.some((fragment) => message.includes(fragment));
}

/** Re-show spinners after a webview reload while clones are still running. */
export async function restoreRepoCloneStatus(
  invoke: TrayInvokeFn,
  addCloning: (repoPath: string) => void,
): Promise<void> {
  const active = (await invoke("get_repo_clone_status")) as string[] | null;
  for (const path of active ?? []) {
    addCloning(path);
  }
}

/**
 * Clone a not-cloned repo. `openAfter` clones and then opens it in the IDE. Allowed under
 * every clone policy.
 */
export async function cloneRepo(
  repo: Pick<RepoDto, "path">,
  openAfter: boolean,
  deps: Pick<
    TrayRepoCloneDeps,
    "invoke" | "onError" | "addCloning" | "removeCloning"
  >,
): Promise<void> {
  trayTrace("invoke clone_repo", { repoPath: repo.path, openAfter });
  deps.addCloning(repo.path);
  try {
    await deps.invoke("clone_repo", { repoPath: repo.path, openAfter });
    trayTrace("clone_repo ok", { repoPath: repo.path });
  } catch (e) {
    trayTrace("clone_repo failed", e);
    // A rejected invoke means this call never started a clone, so no events will follow.
    // "already being cloned" keeps the spinner: another clone of this repo is running.
    if (!isPreStartCloneError(String(e))) {
      deps.removeCloning(repo.path);
    }
    deps.onError(e);
  }
}

export function onRepoCloneStarted(
  payload: RepoCloneEvent,
  addCloning: (repoPath: string) => void,
): void {
  addCloning(payload.repo_path);
}

export async function onRepoCloneComplete(
  payload: RepoCloneEvent,
  deps: Pick<
    TrayRepoCloneDeps,
    "removeCloning" | "refresh" | "onError" | "onOpened"
  >,
): Promise<void> {
  deps.removeCloning(payload.repo_path);
  await deps.refresh();
  if (payload.error) {
    // Cloned fine, but the IDE launch failed.
    deps.onError(payload.error);
    return;
  }
  if (payload.opened) {
    deps.onOpened?.();
  }
}

export function onRepoCloneFailed(
  payload: RepoCloneEvent,
  deps: Pick<TrayRepoCloneDeps, "removeCloning" | "onError">,
): void {
  deps.removeCloning(payload.repo_path);
  deps.onError(payload.error ?? "clone failed");
}
