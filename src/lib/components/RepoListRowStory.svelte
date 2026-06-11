<script lang="ts">
  import RepoListRow from "./RepoListRow.svelte";
  import type { RepoDto } from "$lib/types";

  type Props = {
    repo: RepoDto;
    selected?: boolean;
    rowIndex?: number;
    listRowDraggable?: boolean;
    onOpen: () => void;
    onDetail: () => void;
    onRowContextMenu?: (e: MouseEvent) => void;
    onRowDragStart?: (e: DragEvent) => void;
    onRowDragOver?: (e: DragEvent) => void;
    onRowDrop?: (e: DragEvent) => void;
    onRowDragEnd?: (e: DragEvent) => void;
  };

  let props: Props = $props();
</script>

<div
  class="panel-shell w-full max-w-md rounded-xl bg-inverse-surface p-2 text-inverse-on-surface"
>
  <ul role="list" class="w-full">
    <RepoListRow {...props} />
  </ul>
</div>
