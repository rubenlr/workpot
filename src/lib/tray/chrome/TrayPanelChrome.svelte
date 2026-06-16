<script lang="ts">
  import DetailPane from "$lib/tray/repo-detail/DetailPane.svelte";
  import type { TrayListView } from "$lib/tray/logic/list/listState";
  import type { SectionedRepos } from "$lib/tray/logic/list/sort";
  import type {
    ActiveConvert,
    ActiveSync,
    RepoDto,
    SyncDirection,
  } from "$lib/types";
  import type { toPinOrderPayload } from "$lib/tray/logic/list/pinOrder";
  import { observePanelHeight } from "$lib/tray/logic/layout/observePanelHeight";
  import TrayErrorBanner from "./TrayErrorBanner.svelte";
  import TrayFilterBar from "$lib/tray/repo-list/TrayFilterBar.svelte";
  import TrayListBody from "$lib/tray/repo-list/TrayListBody.svelte";

  const noopBindFilter = (() => {}) as (el: HTMLInputElement | null) => void;

  let {
    listMaxHeightPx,
    onPanelHeightChange,
    launchError = null,
    onDismissLaunchError,
    listError = null,
    onDismissListError,
    filterQuery = $bindable(""),
    allTags,
    tagAutocompletePrefix,
    onFilterKeydown,
    onTagSelect,
    bindFilterInput,
    listView,
    emptyListMessage,
    noMatchMessage,
    sectionedRepos,
    flatIndexByPath,
    selectedIndex = $bindable(0),
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
    activeConvert = null,
    onConvert,
    branchRevision = 0,
  }: {
    listMaxHeightPx: number;
    onPanelHeightChange?: (heightPx: number) => void;
    launchError?: string | null;
    onDismissLaunchError?: () => void;
    listError?: string | null;
    onDismissListError?: () => void;
    filterQuery?: string;
    allTags: string[];
    tagAutocompletePrefix?: string | null;
    onFilterKeydown: (event: KeyboardEvent) => void;
    onTagSelect: (tag: string) => void;
    bindFilterInput?: (el: HTMLInputElement | null) => void;
    listView: TrayListView;
    emptyListMessage?: string;
    noMatchMessage?: string;
    sectionedRepos: SectionedRepos;
    flatIndexByPath: Map<string, number>;
    selectedIndex?: number;
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
    activeConvert?: ActiveConvert | null;
    onConvert?: (repoPath: string) => void;
    branchRevision?: number;
  } = $props();
</script>

<main
  class="panel-shell flex h-auto flex-col overflow-hidden rounded-xl bg-inverse-surface text-inverse-on-surface shadow-2xl"
  style="max-height: {listMaxHeightPx}px"
  use:observePanelHeight={onPanelHeightChange}
>
  {#if launchError && onDismissLaunchError}
    <TrayErrorBanner message={launchError} onDismiss={onDismissLaunchError} />
  {/if}

  {#if listError && onDismissListError}
    <TrayErrorBanner message={listError} onDismiss={onDismissListError} />
  {:else if listError}
    <TrayErrorBanner message={listError} />
  {/if}

  {#if !detailRepo}
    <TrayFilterBar
      {onRefresh}
      {refreshing}
      {refreshSuccess}
      bind:filterQuery
      {allTags}
      tagAutocompletePrefix={tagAutocompletePrefix ?? ""}
      {onFilterKeydown}
      {onTagSelect}
      bindFilterInput={bindFilterInput ?? noopBindFilter}
    />
  {/if}

  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if detailRepo && onCloseDetail && onDetailMutated}
      <DetailPane
        repo={detailRepo}
        {allTags}
        {activeSync}
        {onSync}
        {branchRevision}
        requestTagFocus={focusTagOnDetailOpen}
        {onTagFocusDone}
        onClose={onCloseDetail}
        onMutated={onDetailMutated}
      />
    {:else}
      <TrayListBody
        {listView}
        {emptyListMessage}
        {noMatchMessage}
        {sectionedRepos}
        {flatIndexByPath}
        bind:selectedIndex
        {onPinReorder}
        {onSelectRow}
        {activeSync}
        {onSync}
        {activeConvert}
        {onConvert}
        {onOpen}
        {onDetail}
      />
    {/if}
  </div>
</main>
