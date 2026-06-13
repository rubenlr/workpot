<script lang="ts">
  import TrayRepoList from "./TrayRepoList.svelte";
  import type { SectionedRepos } from "$lib/tray/logic/list/sort";
  import type { ActiveSync, RepoDto, SyncDirection } from "$lib/types";
  import type { toPinOrderPayload } from "$lib/tray/logic/list/pinOrder";

  let {
    sectionedRepos,
    flatIndexByPath,
    selectedIndex = $bindable(0),
    onPinReorder,
    onSelectRow,
    onOpen,
    onDetail,
    activeSync = null,
    onSync,
  }: {
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
  } = $props();
</script>

<div
  class="w-full max-w-md rounded-xl bg-inverse-surface text-inverse-on-surface"
>
  <TrayRepoList
    {sectionedRepos}
    {flatIndexByPath}
    bind:selectedIndex
    {onPinReorder}
    {onSelectRow}
    {onOpen}
    {onDetail}
    {activeSync}
    {onSync}
  />
</div>
