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
  <p class="mb-2 text-xs text-inverse-on-surface-variant">
    Repo context menu is handled by TrayRepoList document delegation in the tray
    app.
  </p>
  <ul role="list" class="w-full">
    <RepoListRow {...props} />
  </ul>
</div>
