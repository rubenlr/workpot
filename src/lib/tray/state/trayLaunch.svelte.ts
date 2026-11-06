import { invoke } from "@tauri-apps/api/core";
import { getCurrentWindow } from "@tauri-apps/api/window";
import type { SectionConfig } from "$lib/tray/logic/list/sort";
import { isCloned } from "$lib/types";
import type { RepoDto } from "$lib/types";
import { trayTrace } from "$lib/tray/logic/handlers/trayTrace";
import { selectionIndexAfterBackgroundOpen } from "$lib/tray/logic/list/openSelection";

interface TrayLaunchDeps {
  getSelectedRepo: () => RepoDto | undefined;
  getFilterQuery: () => string;
  getSectionCfg: () => SectionConfig;
  getRepos: () => RepoDto[];
  refresh: (clearError: boolean) => Promise<void>;
  setSelectedIndex: (index: number) => void;
  /** Clone a not-cloned repo, then open it (clone-on-open policies). */
  cloneAndOpen?: (repo: RepoDto) => Promise<void>;
}

export function createTrayLaunch(deps: TrayLaunchDeps) {
  let launchError = $state<string | null>(null);

  function dismissLaunchError() {
    launchError = null;
  }

  async function openSelected(background: boolean): Promise<void> {
    const repo = deps.getSelectedRepo();
    if (!repo) {
      return;
    }
    launchError = null;
    if (!isCloned(repo)) {
      if (repo.clone_policy === "manual" || !deps.cloneAndOpen) {
        launchError = `${repo.name} is not cloned yet. Use the Clone button.`;
        return;
      }
      await deps.cloneAndOpen(repo);
      return;
    }
    try {
      await invoke("open_in_cursor", { path: repo.path, background });
      if (background) {
        const openedPath = repo.path;
        const query = deps.getFilterQuery();
        await deps.refresh(false);
        deps.setSelectedIndex(
          selectionIndexAfterBackgroundOpen(
            deps.getRepos(),
            query,
            openedPath,
            deps.getSectionCfg(),
          ),
        );
      } else {
        await getCurrentWindow().hide();
      }
    } catch (e) {
      launchError = String(e);
    }
  }

  /** Hide the panel after an open; a failed hide is surfaced like a failed launch. */
  async function hidePanelReportingErrors(): Promise<void> {
    try {
      await getCurrentWindow().hide();
    } catch (e) {
      trayTrace("hide panel failed", e);
      launchError = String(e);
    }
  }

  return {
    get launchError() {
      return launchError;
    },
    openSelected,
    dismissLaunchError,
    hidePanel: () => getCurrentWindow().hide(),
    hidePanelReportingErrors,
  };
}

export type TrayLaunch = ReturnType<typeof createTrayLaunch>;
