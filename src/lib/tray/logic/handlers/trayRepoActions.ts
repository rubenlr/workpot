import { resyncDetailRepo } from "$lib/tray/logic/detail/detailRepoSync";
import type { RepoDto } from "$lib/types";
import { resolveContextAction, type ContextCommand } from "./trayContextAction";

export type TrayInvokeFn = (
  cmd: string,
  args?: Record<string, unknown>,
) => Promise<unknown>;

export interface TrayRepoActionsDeps {
  invoke: TrayInvokeFn;
  refresh: () => Promise<void>;
  onError: (e: unknown) => void;
  openDetailWithTagFocus: (repo: RepoDto) => void;
  onConvert?: (repoPath: string) => Promise<void>;
}

async function mutateThenRefresh(
  invokeFn: () => Promise<void>,
  refresh: () => Promise<void>,
  onError: (e: unknown) => void,
): Promise<void> {
  try {
    await invokeFn();
    await refresh();
  } catch (e) {
    onError(e);
  }
}

export async function removeTag(
  repoPath: string,
  tag: string,
  deps: TrayRepoActionsDeps,
): Promise<void> {
  await mutateThenRefresh(
    () => deps.invoke("remove_tag", { repoPath, tag }) as Promise<void>,
    deps.refresh,
    deps.onError,
  );
}

export async function setPinOrder(
  items: { path: string; order: number }[],
  deps: TrayRepoActionsDeps,
): Promise<void> {
  await mutateThenRefresh(
    () => deps.invoke("set_pin_order", { items }) as Promise<void>,
    deps.refresh,
    deps.onError,
  );
}

export async function executeContextCommand(
  cmd: ContextCommand,
  deps: TrayRepoActionsDeps,
): Promise<void> {
  switch (cmd.kind) {
    case "toggle_pin":
      await mutateThenRefresh(
        () =>
          deps.invoke("set_pin", {
            repoPath: cmd.repoPath,
            pinned: cmd.pinned,
          }) as Promise<void>,
        deps.refresh,
        deps.onError,
      );
      break;
    case "remove_tag":
      await removeTag(cmd.repoPath, cmd.tag, deps);
      break;
    case "open_detail_tag_focus":
      deps.openDetailWithTagFocus(cmd.repo);
      break;
    case "convert_repo":
      await deps.onConvert?.(cmd.repoPath);
      break;
    case "noop":
      break;
  }
}

export async function handleRepoContextAction(
  payload: { action: string; repo_path: string },
  repos: RepoDto[],
  deps: TrayRepoActionsDeps,
): Promise<void> {
  const { action, repo_path } = payload;
  const repo = resyncDetailRepo(repos, repo_path);
  await executeContextCommand(
    resolveContextAction(action, repo, repo_path),
    deps,
  );
}
