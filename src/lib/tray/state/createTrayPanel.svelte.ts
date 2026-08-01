import { invoke } from "@tauri-apps/api/core";
import { toPinOrderPayload } from "$lib/tray/logic/list/pinOrder";
import { createTrayConfig } from "./trayConfig.svelte";
import { createTrayDetail } from "./trayDetail.svelte";
import {
  onGitRefreshComplete,
  onGitRefreshFailed,
  onPanelOpened,
} from "$lib/tray/logic/handlers/trayGitRefreshHandlers";
import {
  onSyncComplete,
  onSyncFailed,
} from "$lib/tray/logic/handlers/traySyncHandlers";
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
  restoreRepoSyncStatus,
  syncRepoBranch,
  type TrayRepoSyncDeps,
} from "$lib/tray/logic/handlers/trayRepoSync";
import {
  convertRepo,
  onRepoConvertComplete,
  onRepoConvertFailed,
  onRepoConvertStarted,
  restoreRepoConvertStatus,
  type TrayRepoConvertDeps,
} from "$lib/tray/logic/handlers/trayRepoConvert";
import type { ActiveConvert, ActiveSync, SyncDirection } from "$lib/types";

const MIN_SYNC_MS = 1000;
const SYNC_SUCCESS_FLASH_MS = 400;

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

  let unsubscribeEvents: (() => void) | null = null;
  let activeSync = $state<ActiveSync | null>(null);
  let activeConvert = $state<ActiveConvert | null>(null);
  let syncing = $state(false);
  let syncSuccess = $state(false);
  let syncingStartedAt = $state<number | null>(null);
  let branchRevision = $state(0);

  function resetPanelToInitialState() {
    detail.closeDetail();
    list.filterQuery = "";
    list.selectedIndex = 0;
  }

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

  const convertDeps: TrayRepoConvertDeps = {
    invoke,
    refresh: () => data.refresh(),
    onError: (e) => data.setListError(String(e)),
    setActiveConvert: (convert) => {
      activeConvert = convert;
    },
  };

  async function handleConvert(repoPath: string): Promise<void> {
    const repo = data.repos.find((r) => r.path === repoPath);
    if (!repo?.convert_to) {
      return;
    }
    await convertRepo(repoPath, repo.convert_to, convertDeps);
  }

  const actionDeps: TrayRepoActionsDeps = {
    invoke,
    refresh: () => data.refresh(),
    onError: (e) => data.setListError(String(e)),
    openDetailWithTagFocus: (repo) => detail.openDetailWithTagFocus(repo),
    onConvert: handleConvert,
  };

  const catalogSyncDeps = {
    setSelectedIndex: (index: number) => {
      list.selectedIndex = index;
    },
    refresh: (clearError: boolean) => data.refresh(clearError),
    resyncDetail: () => detail.resync(data.repos),
    setError: (message: string | null) => data.setListError(message),
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
    bumpBranchRevision: () => {
      branchRevision += 1;
    },
  };

  async function finishSync(success: boolean): Promise<void> {
    const started = syncingStartedAt ?? Date.now();
    const remaining = MIN_SYNC_MS - (Date.now() - started);
    if (remaining > 0) {
      await new Promise((resolve) => setTimeout(resolve, remaining));
    }
    syncing = false;
    syncingStartedAt = null;
    if (success) {
      syncSuccess = true;
      await new Promise((resolve) =>
        setTimeout(resolve, SYNC_SUCCESS_FLASH_MS),
      );
      syncSuccess = false;
    }
  }

  async function startSync(): Promise<void> {
    trayTrace("invoke refresh_sync");
    try {
      await invoke("refresh_sync");
      trayTrace("refresh_sync ok");
    } catch (e) {
      trayTrace("refresh_sync failed", e);
      data.setListError(String(e));
      await finishSync(false);
    }
  }

  function beginSync(): void {
    syncing = true;
    syncingStartedAt = Date.now();
    trayTrace("refresh_sync requested");
    void startSync();
  }

  const keyboard = createTrayPanelKeyboard({
    list,
    detail,
    launch,
    startSync: beginSync,
  });

  async function mount(): Promise<void> {
    trayTrace("mount start");
    unsubscribeEvents = await subscribeTrayPanelEvents({
      onPanelOpened: () => onPanelOpened(gitRefreshDeps),
      onPanelClosed: () => resetPanelToInitialState(),
      onGitRefreshStarted: () => {},
      onGitRefreshComplete: (summary) => {
        onGitRefreshComplete(summary, gitRefreshDeps);
      },
      onGitRefreshFailed: (message) => {
        onGitRefreshFailed(message, gitRefreshDeps);
      },
      onSyncStarted: () => {
        trayTrace("sync-started");
        if (!syncing) {
          syncing = true;
          syncingStartedAt = Date.now();
        }
      },
      onSyncComplete: (summary) => {
        void (async () => {
          await finishSync(true);
          onSyncComplete(summary, catalogSyncDeps);
        })();
      },
      onSyncFailed: (message) => {
        void (async () => {
          await finishSync(false);
          onSyncFailed(message, catalogSyncDeps);
        })();
      },
      onRepoSyncStarted: (payload) =>
        onRepoSyncStarted(payload, syncDeps.setActiveSync),
      onRepoSyncComplete: (payload) => {
        void onRepoSyncComplete(payload, syncDeps);
      },
      onRepoSyncFailed: (payload) => onRepoSyncFailed(payload, syncDeps),
      onRepoConvertStarted: (payload) =>
        onRepoConvertStarted(payload, convertDeps.setActiveConvert),
      onRepoConvertComplete: (payload) => {
        void onRepoConvertComplete(payload, convertDeps);
      },
      onRepoConvertFailed: (payload) =>
        onRepoConvertFailed(payload, convertDeps),
      onRepoContextAction: (payload) => {
        void handleRepoContextAction(payload, data.repos, actionDeps);
      },
    });

    await Promise.all([
      data.loadRepos(),
      data.loadAllTags(),
      config.loadConfig(),
      restoreRepoSyncStatus(invoke, syncDeps.setActiveSync),
      restoreRepoConvertStatus(invoke, convertDeps.setActiveConvert),
    ]);
    trayTrace("mount ready", { repos: data.repos.length });
    keyboard.focusFilter();
  }

  function destroy() {
    clearGitRefreshWatchdog();
    unsubscribeEvents?.();
    unsubscribeEvents = null;
  }

  const panel = {
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
    get listError() {
      return data.error;
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
    get activeConvert() {
      return activeConvert;
    },
    get syncing() {
      return syncing;
    },
    get syncSuccess() {
      return syncSuccess;
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
    dismissListError: data.dismissListError,
    bindFilterInput: keyboard.bindFilterInput,
    refreshReposAndDetail: () => data.refresh(),
    startSync: beginSync,
    mount,
    destroy,
  };

  return panel;
}

export type TrayPanel = ReturnType<typeof createTrayPanel>;
