import { invoke } from "@tauri-apps/api/core";
import { toPinOrderPayload } from "$lib/tray/logic/list/pinOrder";
import { createTrayConfig } from "./trayConfig.svelte";
import { createTrayDetail } from "./trayDetail.svelte";
import {
  onGitRefreshComplete,
  onGitRefreshFailed,
  onPanelOpened,
} from "$lib/tray/logic/handlers/trayGitRefreshHandlers";
import { createTrayLaunch } from "./trayLaunch.svelte";
import { createTrayListSelection } from "./trayListSelection.svelte";
import { createTrayPanelKeyboard } from "./trayPanelKeyboard.svelte";
import { clearGitRefreshWatchdog } from "$lib/tray/logic/handlers/gitRefreshWatchdog";
import { subscribeTrayPanelEvents } from "$lib/tray/logic/handlers/trayPanelEvents";
import { trayTrace } from "$lib/tray/logic/handlers/trayTrace";
import { createTrayRepoData } from "./trayRepoData.svelte";
import {
  handleRepoContextAction,
  removeTag,
  setPinOrder,
  type TrayRepoActionsDeps,
} from "$lib/tray/logic/handlers/trayRepoActions";
import {
  onRepoSyncComplete,
  onRepoSyncFailed,
  onRepoSyncStarted,
  syncRepoBranch,
  type TrayRepoSyncDeps,
} from "$lib/tray/logic/handlers/trayRepoSync";
import type { ActiveSync, SyncDirection } from "$lib/types";

export function createTrayPanel() {
  const config = createTrayConfig();
  const detail = createTrayDetail();
  const data = createTrayRepoData({
    onAfterRefresh: (repos) => detail.resync(repos),
  });
  const list = createTrayListSelection({
    getRepos: () => data.repos,
    getSectionCfg: () => config.sectionCfg,
    getError: () => data.error,
  });
  const launch = createTrayLaunch({
    getSelectedRepo: () => list.getSelectedRepo(),
    getFilterQuery: () => list.filterQuery,
    getSectionCfg: () => config.sectionCfg,
    getRepos: () => data.repos,
    refresh: (clearError) => data.refresh(clearError),
    setSelectedIndex: (index) => {
      list.selectedIndex = index;
    },
  });
  const keyboard = createTrayPanelKeyboard({ list, detail, launch, data });

  let unsubscribeEvents: (() => void) | null = null;
  let activeSync = $state<ActiveSync | null>(null);
  let refreshing = $state(false);
  let branchRevision = $state(0);

  const actionDeps: TrayRepoActionsDeps = {
    invoke,
    refresh: () => data.refresh(),
    onError: (e) => data.setListError(String(e)),
    openDetailWithTagFocus: (repo) => detail.openDetailWithTagFocus(repo),
  };

  const syncDeps: TrayRepoSyncDeps = {
    invoke,
    refresh: () => data.refresh(),
    onError: (e) => data.setListError(String(e)),
    setActiveSync: (sync) => {
      activeSync = sync;
    },
    bumpBranchRevision: () => {
      branchRevision += 1;
    },
  };

  async function removeTagFromRepo(repoPath: string, tag: string) {
    await removeTag(repoPath, tag, actionDeps);
  }

  async function handlePinReorder(items: ReturnType<typeof toPinOrderPayload>) {
    await setPinOrder(items, actionDeps);
  }

  async function handleSync(
    repoPath: string,
    branch: string,
    direction: SyncDirection,
  ) {
    await syncRepoBranch(repoPath, branch, direction, syncDeps);
  }

  const gitRefreshDeps = {
    setSelectedIndex: (index: number) => {
      list.selectedIndex = index;
    },
    refresh: (clearError: boolean) => data.refresh(clearError),
    setError: (message: string | null) => data.setListError(message),
    focusFilter: () => keyboard.focusFilter(),
  };

  async function mount(): Promise<void> {
    trayTrace("mount start");
    unsubscribeEvents = await subscribeTrayPanelEvents({
      onPanelOpened: () => onPanelOpened(gitRefreshDeps),
      onGitRefreshStarted: () => {
        refreshing = true;
      },
      onGitRefreshComplete: (summary) => {
        refreshing = false;
        onGitRefreshComplete(summary, gitRefreshDeps);
      },
      onGitRefreshFailed: (message) => {
        refreshing = false;
        onGitRefreshFailed(message, gitRefreshDeps);
      },
      onRepoSyncStarted: (payload) =>
        onRepoSyncStarted(payload, syncDeps.setActiveSync),
      onRepoSyncComplete: (payload) => {
        void onRepoSyncComplete(payload, syncDeps);
      },
      onRepoSyncFailed: (payload) => onRepoSyncFailed(payload, syncDeps),
      onRepoContextAction: (payload) => {
        void handleRepoContextAction(payload, data.repos, actionDeps);
      },
    });

    await Promise.all([
      data.loadRepos(),
      data.loadAllTags(),
      config.loadConfig(),
    ]);
    trayTrace("mount ready", { repos: data.repos.length });
    keyboard.focusFilter();
  }

  function destroy() {
    clearGitRefreshWatchdog();
    unsubscribeEvents?.();
    unsubscribeEvents = null;
  }

  return {
    get filterQuery() {
      return list.filterQuery;
    },
    set filterQuery(value: string) {
      list.filterQuery = value;
    },
    get selectedIndex() {
      return list.selectedIndex;
    },
    set selectedIndex(value: number) {
      list.selectedIndex = value;
    },
    get detailRepo() {
      return detail.detailRepo;
    },
    get listView() {
      return list.listView;
    },
    get sectionedRepos() {
      return list.sectionedRepos;
    },
    get flatIndexByPath() {
      return list.flatIndexByPath;
    },
    get allTags() {
      return data.allTags;
    },
    get launchError() {
      return launch.launchError;
    },
    get listMaxHeightPx() {
      return config.listMaxHeightPx;
    },
    get tagAutocompletePrefix() {
      return list.tagAutocompletePrefix;
    },
    get focusTagOnDetailOpen() {
      return detail.focusTagOnDetailOpen;
    },
    get activeSync() {
      return activeSync;
    },
    get refreshing() {
      return refreshing;
    },
    get branchRevision() {
      return branchRevision;
    },
    clearTagFocusRequest: detail.clearTagFocusRequest,
    openDetail: detail.openDetail,
    moveSelection: list.moveSelection,
    openSelected: launch.openSelected,
    hidePanel: launch.hidePanel,
    closeDetail: detail.closeDetail,
    openDetailWithTagFocus: detail.openDetailWithTagFocus,
    appendTagFilter: list.appendTagFilter,
    onTagAutocompleteSelect: list.onTagAutocompleteSelect,
    removeTagFromRepo,
    handlePinReorder,
    handleSync,
    onFilterKeydown: keyboard.onFilterKeydown,
    onPanelKeydown: keyboard.onPanelKeydown,
    dismissLaunchError: launch.dismissLaunchError,
    bindFilterInput: keyboard.bindFilterInput,
    refreshReposAndDetail: () => data.refresh(),
    startBackgroundRefresh: () => data.startBackgroundRefresh(),
    mount,
    destroy,
  };
}

export type TrayPanel = ReturnType<typeof createTrayPanel>;
