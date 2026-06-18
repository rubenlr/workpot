<script lang="ts">
  import MaterialIcon from "$lib/tray/commons/MaterialIcon.svelte";
  import SyncBadge from "$lib/tray/commons/SyncBadge.svelte";
  import { dirtyDotClass } from "$lib/tray/logic/list/repoRow";
  import type {
    ActiveConvert,
    ActiveSync,
    RepoDto,
    SyncDirection,
  } from "$lib/types";

  let {
    repo,
    selected = false,
    hovered = false,
    rowIndex,
    listRowDraggable = false,
    activeSync = null,
    activeConvert = null,
    onOpen,
    onDetail,
    onSync,
    onRowContextMenu,
    onRowDragStart,
    onRowDragOver,
    onRowDrop,
    onRowDragEnd,
    onRowMouseEnter,
    onRowMouseLeave,
  }: {
    repo: RepoDto;
    selected?: boolean;
    hovered?: boolean;
    rowIndex?: number;
    listRowDraggable?: boolean;
    activeSync?: ActiveSync | null;
    activeConvert?: ActiveConvert | null;
    onOpen: () => void;
    onDetail: () => void;
    onSync?: (
      repoPath: string,
      branch: string,
      direction: SyncDirection,
    ) => void;
    onRowContextMenu?: (e: MouseEvent) => void;
    onRowDragStart?: (e: DragEvent) => void;
    onRowDragOver?: (e: DragEvent) => void;
    onRowDrop?: (e: DragEvent) => void;
    onRowDragEnd?: (e: DragEvent) => void;
    onRowMouseEnter?: () => void;
    onRowMouseLeave?: () => void;
  } = $props();

  const rowLabel = $derived(repo.alias ?? repo.name);

  const syncingDirection = $derived(
    activeSync &&
      activeSync.repoPath === repo.path &&
      activeSync.branch === repo.branch
      ? activeSync.direction
      : null,
  );

  const syncDisabled = $derived(activeSync != null || activeConvert != null);

  const converting = $derived(activeConvert?.repoPath === repo.path);

  const rowSurfaceClass = $derived(
    selected
      ? "bg-primary text-primary-foreground"
      : hovered
        ? "bg-hover-overlay text-inverse-on-surface"
        : "text-inverse-on-surface",
  );

  const openButtonClass =
    "flex min-w-0 flex-1 cursor-pointer items-center gap-2 border-0 bg-transparent px-3 py-2.5 text-left text-inherit shadow-none outline-none focus-visible:ring-1 focus-visible:ring-primary";

  const chevronClass = $derived(
    [
      "detail-btn flex shrink-0 cursor-pointer items-center justify-center self-center rounded-lg border-0 bg-transparent px-2 py-2 text-inherit shadow-none outline-none focus-visible:ring-1 focus-visible:ring-primary",
      selected ? "" : "opacity-80",
    ].join(" "),
  );

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
  onmouseenter={onRowMouseEnter}
  onmouseleave={onRowMouseLeave}
  class="relative w-full overflow-hidden rounded-lg text-left transition-transform {selected
    ? 'scale-[1.01] shadow-[var(--shadow-row-selected)]'
    : ''}"
>
  <div class="flex w-full items-center rounded-lg {rowSurfaceClass}">
    <button
      type="button"
      class={openButtonClass}
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
              ? 'text-primary-foreground'
              : 'text-inverse-on-surface-variant'}"
          >
            {repo.branch}
          </span>
        {/if}
      </span>
    </button>
    <div class="flex shrink-0 items-center self-center pr-1">
      <SyncBadge
        ahead={repo.ahead}
        behind={repo.behind}
        branch={repo.branch}
        tone={selected ? "on-primary" : "default"}
        {syncingDirection}
        disabled={syncDisabled}
        onPush={repo.branch && onSync
          ? () => onSync(repo.path, repo.branch!, "push")
          : undefined}
        onPull={repo.branch && onSync
          ? () => onSync(repo.path, repo.branch!, "pull")
          : undefined}
      />
      {#if repo.convert_to && converting}
        <span
          class="flex shrink-0 items-center self-center p-1.5 opacity-80"
          aria-label="Converting to {repo.convert_to}"
        >
          <MaterialIcon name="sync" size={14} class="animate-spin" />
        </span>
      {/if}
    </div>
    <div
      role="separator"
      aria-orientation="vertical"
      class="h-6 w-px shrink-0 self-center bg-current opacity-20"
    ></div>
    <button
      type="button"
      class={chevronClass}
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

<style>
  /*
   * When the detail chevron button is hovered, suppress the row-wide hover
   * overlay so only the chevron itself is highlighted — not the whole row.
   * This visually communicates that the two zones trigger different actions.
   */
  div:has(.detail-btn:hover) {
    background-color: transparent !important;
  }

  .detail-btn:hover {
    background-color: color-mix(in sRGB, currentColor 12%, transparent);
  }
</style>
