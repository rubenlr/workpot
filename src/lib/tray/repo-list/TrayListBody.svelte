<script lang="ts">
  import type { TrayListView } from "$lib/tray/logic/list/listState";
  import {
    TRAY_EMPTY_LIST_MESSAGE,
    TRAY_NO_MATCH_MESSAGE,
  } from "$lib/tray/logic/handlers/constants";
  import type { SectionedRepos } from "$lib/tray/logic/list/sort";
  import type {
    ActiveConvert,
    ActiveSync,
    RepoDto,
    SyncDirection,
  } from "$lib/types";
  import TrayListPlaceholder from "./TrayListPlaceholder.svelte";
  import TrayRepoList from "./TrayRepoList.svelte";
  import type { toPinOrderPayload } from "$lib/tray/logic/list/pinOrder";

  let {
    listView,
    emptyListMessage = TRAY_EMPTY_LIST_MESSAGE,
    noMatchMessage = TRAY_NO_MATCH_MESSAGE,
    sectionedRepos,
    flatIndexByPath,
    selectedIndex = $bindable(0),
    onPinReorder,
    onSelectRow,
    onOpen,
    onDetail,
    activeSync = null,
    onSync,
    activeConvert = null,
    onConvert,
  }: {
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
    activeSync?: ActiveSync | null;
    onSync?: (
      repoPath: string,
      branch: string,
      direction: SyncDirection,
    ) => void;
    activeConvert?: ActiveConvert | null;
    onConvert?: (repoPath: string) => void;
  } = $props();
</script>

{#if listView.kind === "empty-list"}
  <TrayListPlaceholder message={emptyListMessage} />
{:else if listView.kind === "no-match"}
  <TrayListPlaceholder message={noMatchMessage} />
{:else if listView.kind === "list"}
  <TrayRepoList
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
