<script lang="ts">
  import TrayListBody from "./TrayListBody.svelte";
  import type { TrayListView } from "$lib/tray/logic/list/listState";
  import type { SectionedRepos } from "$lib/tray/logic/list/sort";
  import type { RepoDto } from "$lib/types";
  import type { toPinOrderPayload } from "$lib/tray/logic/list/pinOrder";

  let {
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
  } = $props();
</script>

<div
  class="w-full max-w-md rounded-xl bg-inverse-surface text-inverse-on-surface"
>
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
</div>
