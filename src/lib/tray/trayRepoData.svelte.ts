import { invoke } from "@tauri-apps/api/core";
import type { RepoDto } from "$lib/types";

export interface TrayRepoDataOptions {
  onAfterRefresh?: (repos: RepoDto[]) => void;
}

export function createTrayRepoData(options: TrayRepoDataOptions = {}) {
  let repos = $state<RepoDto[]>([]);
  let allTags = $state<string[]>([]);
  let error = $state<string | null>(null);
  let refreshing = $state(false);

  function setError(e: unknown) {
    error = String(e);
  }

  function setListError(message: string | null) {
    error = message;
  }

  async function loadRepos(clearError = true): Promise<void> {
    try {
      repos = await invoke<RepoDto[]>("list_repos");
      if (clearError) {
        error = null;
      }
    } catch (e) {
      setError(e);
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
    await loadRepos(clearError);
    await loadAllTags();
    options.onAfterRefresh?.(repos);
  }

  async function startBackgroundRefresh(): Promise<void> {
    refreshing = true;
    try {
      await invoke("refresh_all_git_state");
    } catch (e) {
      refreshing = false;
      setError(e);
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
    get refreshing() {
      return refreshing;
    },
    set refreshing(value: boolean) {
      refreshing = value;
    },
    loadRepos,
    loadAllTags,
    refresh,
    startBackgroundRefresh,
    setError,
    setListError,
  };
}

export type TrayRepoData = ReturnType<typeof createTrayRepoData>;
