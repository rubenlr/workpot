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
    onTagRemove?: (tag: string) => void | Promise<void>;
    onTagFilter?: (tag: string) => void;
    onRowContextMenu?: (e: MouseEvent) => void;
    onRowDragStart?: (e: DragEvent) => void;
    onRowDragOver?: (e: DragEvent) => void;
    onRowDrop?: (e: DragEvent) => void;
    onRowDragEnd?: (e: DragEvent) => void;
  };

  let props: Props = $props();
</script>

<div
  class="panel-shell w-full max-w-md rounded-xl text-neutral-900 dark:text-neutral-100"
>
  <ul role="list" class="w-full">
    <RepoListRow {...props} />
  </ul>
</div>
