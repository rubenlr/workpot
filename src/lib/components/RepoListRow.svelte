<script lang="ts">
  import MaterialIcon from "./MaterialIcon.svelte";
  import SyncBadge from "./SyncBadge.svelte";
  import { dirtyDotClass } from "$lib/repoRow";
  import type { RepoDto } from "$lib/types";

  let {
    repo,
    selected = false,
    rowIndex,
    listRowDraggable = false,
    onOpen,
    onDetail,
    onRowContextMenu,
    onRowDragStart,
    onRowDragOver,
    onRowDrop,
    onRowDragEnd,
  }: {
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
  } = $props();

  const rowLabel = $derived(repo.alias ?? repo.name);

  function activateRow(metaKey: boolean) {
    if (metaKey) {
      onDetail();
    } else {
      onOpen();
    }
  }
</script>

<li
  role="listitem"
  aria-current={selected ? "true" : undefined}
  data-row-index={rowIndex}
  draggable={listRowDraggable ? "true" : undefined}
  oncontextmenu={onRowContextMenu}
  ondragstart={onRowDragStart}
  ondragover={onRowDragOver}
  ondrop={onRowDrop}
  ondragend={onRowDragEnd}
  class="group relative w-full overflow-hidden rounded-lg text-left transition-transform {selected
    ? 'scale-[1.01] bg-primary text-primary-foreground shadow-[var(--shadow-row-selected)]'
    : 'text-inverse-on-surface hover:bg-white/5'}"
>
  <div class="flex w-full items-stretch">
    <button
      type="button"
      class="flex min-w-0 flex-1 cursor-pointer items-center gap-2 border-0 bg-transparent px-3 py-2.5 text-left text-inherit shadow-none outline-none focus-visible:ring-1 focus-visible:ring-primary"
      aria-label="Open {rowLabel}"
      onclick={(e) => activateRow(e.metaKey)}
    >
      <span
        class="h-2 w-2 shrink-0 rounded-full {dirtyDotClass(repo)}"
        aria-hidden="true"
      ></span>
      <span class="min-w-0 flex-1">
        <span class="block truncate text-sm font-medium leading-tight"
          >{rowLabel}</span
        >
        {#if repo.branch}
          <span
            class="block truncate text-xs leading-tight {selected
              ? 'text-primary-foreground/80'
              : 'text-inverse-on-surface-variant'}"
          >
            {repo.branch}
          </span>
        {/if}
      </span>
      <SyncBadge ahead={repo.ahead} behind={repo.behind} />
    </button>
    <button
      type="button"
      class="flex shrink-0 cursor-pointer items-center justify-center border-0 bg-transparent px-2 shadow-none outline-none focus-visible:ring-1 focus-visible:ring-primary {selected
        ? 'text-primary-foreground/90'
        : 'text-inverse-on-surface-variant opacity-60 group-hover:opacity-100'}"
      aria-label="Open detail for {rowLabel}"
      onclick={(e) => {
        e.stopPropagation();
        onDetail();
      }}
    >
      <MaterialIcon name="chevron_right" size={20} />
    </button>
  </div>
</li>
