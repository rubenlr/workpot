<script lang="ts">
  import TrayPanelChrome from "./TrayPanelChrome.svelte";
  import { trayListView } from "$lib/tray/logic/list/listState";
  import { DEFAULT_SECTION_CFG } from "$lib/tray/logic/list/openSelection";
  import {
    filterAndSectionRepos,
    flatSectioned,
  } from "$lib/tray/logic/list/trayList";
  import { storyTrayRepos } from "$lib/tray/storybook/trayPanelStoryFixtures";
  import { trailingTagAutocompletePrefix } from "$lib/tagFilter";
  import type { ActiveSync, RepoDto, SyncDirection } from "$lib/types";
  import type { toPinOrderPayload } from "$lib/tray/logic/list/pinOrder";

  const noopBindFilter = (() => {}) as (el: HTMLInputElement | null) => void;

  let {
    repos = storyTrayRepos(),
    listMaxHeightPx,
    onPanelHeightChange,
    launchError = null,
    onDismissLaunchError,
    listError = null,
    onDismissListError,
    allTags,
    onFilterKeydown,
    onTagSelect,
    bindFilterInput = noopBindFilter,
    onPinReorder,
    onSelectRow,
    onOpen,
    onDetail,
    onRefresh,
    refreshing = false,
    refreshSuccess = false,
    detailRepo = null,
    focusTagOnDetailOpen = false,
    onTagFocusDone,
    onCloseDetail,
    onDetailMutated,
    activeSync = null,
    onSync,
    branchRevision = 0,
    emptyListMessage,
    noMatchMessage,
  }: {
    repos?: RepoDto[];
    listMaxHeightPx: number;
    onPanelHeightChange?: (heightPx: number) => void;
    launchError?: string | null;
    onDismissLaunchError?: () => void;
    listError?: string | null;
    onDismissListError?: () => void;
    allTags: string[];
    onFilterKeydown: (event: KeyboardEvent) => void;
    onTagSelect: (tag: string) => void;
    bindFilterInput?: (el: HTMLInputElement | null) => void;
    onPinReorder: (
      items: ReturnType<typeof toPinOrderPayload>,
    ) => void | Promise<void>;
    onSelectRow: (index: number) => void;
    onOpen: (index: number) => void;
    onDetail: (repo: RepoDto, index: number) => void;
    onRefresh?: () => void;
    refreshing?: boolean;
    refreshSuccess?: boolean;
    detailRepo?: RepoDto | null;
    focusTagOnDetailOpen?: boolean;
    onTagFocusDone?: () => void;
    onCloseDetail?: () => void;
    onDetailMutated?: () => void;
    activeSync?: ActiveSync | null;
    onSync?: (
      repoPath: string,
      branch: string,
      direction: SyncDirection,
    ) => void;
    branchRevision?: number;
    emptyListMessage?: string;
    noMatchMessage?: string;
  } = $props();

  let filterQuery = $state("");
  let selectedIndex = $state(0);

  const sectionedRepos = $derived(
    filterAndSectionRepos(repos, filterQuery, DEFAULT_SECTION_CFG),
  );
  const flatVisible = $derived(flatSectioned(sectionedRepos));
  const flatIndexByPath = $derived(
    new Map(flatVisible.map((r, i) => [r.path, i] as const)),
  );
  const tagAutocompletePrefix = $derived(
    trailingTagAutocompletePrefix(filterQuery),
  );
  const listView = $derived(
    trayListView(listError, repos.length, filterQuery, flatVisible.length),
  );
</script>

<TrayPanelChrome
  {listMaxHeightPx}
  {onPanelHeightChange}
  {launchError}
  {onDismissLaunchError}
  {listError}
  {onDismissListError}
  bind:filterQuery
  {allTags}
  {tagAutocompletePrefix}
  {onFilterKeydown}
  {onTagSelect}
  {bindFilterInput}
  {listView}
  {emptyListMessage}
  {noMatchMessage}
  {sectionedRepos}
  {flatIndexByPath}
  bind:selectedIndex
  {onPinReorder}
  {onSelectRow}
  {onOpen}
  {onDetail}
  {onRefresh}
  {refreshing}
  {refreshSuccess}
  {detailRepo}
  {focusTagOnDetailOpen}
  {onTagFocusDone}
  {onCloseDetail}
  {onDetailMutated}
  {activeSync}
  {onSync}
  {branchRevision}
/>
