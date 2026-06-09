import { invoke } from "@tauri-apps/api/core";
import type { RepoDto } from "$lib/types";
import { trayTrace } from "./trayTrace";

export interface TrayRepoDataOptions {
  onAfterRefresh?: (repos: RepoDto[]) => void;
}

export function createTrayRepoData(options: TrayRepoDataOptions = {}) {
  let repos = $state<RepoDto[]>([]);
  let allTags = $state<string[]>([]);
  let error = $state<string | null>(null);

  function setListError(message: string | null) {
    error = message;
  }

  async function loadRepos(clearError = true): Promise<void> {
    trayTrace("invoke list_repos");
    try {
      repos = await invoke<RepoDto[]>("list_repos");
      trayTrace("list_repos ok", { count: repos.length });
      if (clearError) {
        error = null;
      }
    } catch (e) {
      trayTrace("list_repos failed", e);
      setListError(String(e));
    }
  }

  async function loadAllTags(): Promise<void> {
    try {
      allTags = await invoke<string[]>("list_all_tags");
    } catch (e) {
      console.warn("list_all_tags failed", e);
      allTags = [];
    }
  }

  async function refresh(clearError = true): Promise<void> {
    await Promise.all([loadRepos(clearError), loadAllTags()]);
    options.onAfterRefresh?.(repos);
  }

  async function startBackgroundRefresh(): Promise<void> {
    trayTrace("invoke refresh_all_git_state");
    try {
      await invoke("refresh_all_git_state");
    } catch (e) {
      trayTrace("refresh_all_git_state failed", e);
      setListError(String(e));
    }
  }

  return {
    get repos() {
      return repos;
    },
    get allTags() {
      return allTags;
    },
    get error() {
      return error;
    },
    loadRepos,
    loadAllTags,
    refresh,
    startBackgroundRefresh,
    setListError,
  };
}

export type TrayRepoData = ReturnType<typeof createTrayRepoData>;
