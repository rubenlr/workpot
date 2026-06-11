<script lang="ts">
  import DetailPane from "$lib/tray/repo-detail/DetailPane.svelte";
  import type { TrayListView } from "$lib/tray/logic/list/listState";
  import type { SectionedRepos } from "$lib/tray/logic/list/sort";
  import type { RepoDto } from "$lib/types";
  import type { toPinOrderPayload } from "$lib/tray/logic/list/pinOrder";
  import LaunchErrorBanner from "./LaunchErrorBanner.svelte";
  import TrayFilterBar from "$lib/tray/repo-list/TrayFilterBar.svelte";
  import TrayListBody from "$lib/tray/repo-list/TrayListBody.svelte";

  const noopBindFilter = (() => {}) as (el: HTMLInputElement | null) => void;

  let {
    listMaxHeightPx,
    launchError = null,
    onDismissLaunchError,
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
    detailRepo = null,
    focusTagOnDetailOpen = false,
    onTagFocusDone,
    onCloseDetail,
    onDetailMutated,
  }: {
    listMaxHeightPx: number;
    launchError?: string | null;
    onDismissLaunchError?: () => void;
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
    detailRepo?: RepoDto | null;
    focusTagOnDetailOpen?: boolean;
    onTagFocusDone?: () => void;
    onCloseDetail?: () => void;
    onDetailMutated?: () => void;
  } = $props();
</script>

<main
  class="panel-shell flex h-screen flex-col overflow-hidden rounded-xl bg-inverse-surface text-inverse-on-surface shadow-2xl"
  style="max-height: {listMaxHeightPx}px"
>
  {#if launchError && onDismissLaunchError}
    <LaunchErrorBanner message={launchError} onDismiss={onDismissLaunchError} />
  {/if}

  <TrayFilterBar
    {onRefresh}
    {refreshing}
    bind:filterQuery
    {allTags}
    tagAutocompletePrefix={tagAutocompletePrefix ?? ""}
    {onFilterKeydown}
    {onTagSelect}
    bindFilterInput={bindFilterInput ?? noopBindFilter}
  />

  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if detailRepo && onCloseDetail && onDetailMutated}
      <DetailPane
        repo={detailRepo}
        {allTags}
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
        {onOpen}
        {onDetail}
      />
    {/if}
  </div>
</main>
