import type { ActiveConvert, RepoConvertEvent } from "$lib/types";
import { trayTrace } from "./trayTrace";

export type TrayInvokeFn = (
  cmd: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

export interface TrayRepoConvertDeps {
  invoke: TrayInvokeFn;
  refresh: () => Promise<void>;
  onError: (e: unknown) => void;
  setActiveConvert: (convert: ActiveConvert | null) => void;
}

const PRE_START_CONVERT_ERRORS = [
  "a repo convert is already in progress",
  "invalid convert target",
] as const;

function isPreStartConvertError(message: string): boolean {
  return PRE_START_CONVERT_ERRORS.some((fragment) =>
    message.includes(fragment),
  );
}

export async function restoreRepoConvertStatus(
  invoke: TrayInvokeFn,
  setActiveConvert: (convert: ActiveConvert | null) => void,
): Promise<void> {
  const status = (await invoke(
    "get_repo_convert_status",
  )) as RepoConvertEvent | null;
  if (!status?.repo_path) {
    return;
  }
  setActiveConvert({ repoPath: status.repo_path });
}

export async function convertRepo(
  repoPath: string,
  target: "bare" | "local",
  deps: TrayRepoConvertDeps,
): Promise<void> {
  trayTrace("invoke convert_repo", { repoPath, target });
  try {
    await deps.invoke("convert_repo", { repoPath, target });
    trayTrace("convert_repo ok", { repoPath, target });
  } catch (e) {
    trayTrace("convert_repo failed", e);
    const message = String(e);
    if (isPreStartConvertError(message)) {
      deps.onError(e);
    }
  }
}

export function onRepoConvertStarted(
  payload: RepoConvertEvent,
  setActiveConvert: (convert: ActiveConvert | null) => void,
): void {
  setActiveConvert({ repoPath: payload.repo_path });
}

export async function onRepoConvertComplete(
  _payload: RepoConvertEvent,
  deps: Pick<TrayRepoConvertDeps, "setActiveConvert" | "refresh">,
): Promise<void> {
  deps.setActiveConvert(null);
  await deps.refresh();
}

export function onRepoConvertFailed(
  payload: RepoConvertEvent,
  deps: Pick<TrayRepoConvertDeps, "setActiveConvert" | "onError">,
): void {
  deps.setActiveConvert(null);
  deps.onError(payload.error ?? "repo convert failed");
}
