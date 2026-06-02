<script lang="ts">
  import type { TrayListView } from "$lib/listState";
  import {
    TRAY_EMPTY_LIST_MESSAGE,
    TRAY_LIST_ERROR_FALLBACK,
    TRAY_NO_MATCH_MESSAGE,
  } from "./constants";
  import type { SectionedRepos } from "$lib/sort";
  import type { RepoDto } from "$lib/types";
  import TrayListPlaceholder from "./TrayListPlaceholder.svelte";
  import TrayRepoList from "./TrayRepoList.svelte";
  import type { toPinOrderPayload } from "$lib/pinOrder";

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
    onTagRemove,
    onTagFilter,
  }: {
    listView: TrayListView;
    emptyListMessage?: string;
    noMatchMessage?: string;
    sectionedRepos: SectionedRepos;
    flatIndexByPath: Map<string, number>;
    selectedIndex?: number;
    onPinReorder: (items: ReturnType<typeof toPinOrderPayload>) => void | Promise<void>;
    onSelectRow: (index: number) => void;
    onOpen: (index: number) => void;
    onDetail: (repo: RepoDto, index: number) => void;
    onTagRemove: (repoPath: string, tag: string) => void | Promise<void>;
    onTagFilter: (tag: string) => void;
  } = $props();
</script>

{#if listView.kind === "error"}
  <TrayListPlaceholder
    message={listView.message || TRAY_LIST_ERROR_FALLBACK}
    tone="error"
  />
{:else if listView.kind === "empty-list"}
  <TrayListPlaceholder message={emptyListMessage} />
{:else if listView.kind === "no-match"}
  <TrayListPlaceholder message={noMatchMessage} />
{:else}
  <TrayRepoList
    {sectionedRepos}
    {flatIndexByPath}
    bind:selectedIndex
    {onPinReorder}
    {onSelectRow}
    {onOpen}
    {onDetail}
    {onTagRemove}
    {onTagFilter}
  />
{/if}
