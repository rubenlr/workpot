<script lang="ts">
  import TagChip from "./TagChip.svelte";
  import { dirtyDotClass } from "$lib/repoRow";
  import type { RepoDto } from "$lib/types";

  let {
    repo,
    selected = false,
    rowIndex,
    listRowDraggable = false,
    onOpen,
    onDetail,
    onTagRemove,
    onTagFilter,
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
    onTagRemove?: (tag: string) => void | Promise<void>;
    onTagFilter?: (tag: string) => void;
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
  class="group relative w-full overflow-hidden rounded-md text-left {selected
    ? 'bg-blue-600 text-white dark:bg-blue-500'
    : 'text-neutral-900 hover:bg-black/5 dark:text-neutral-100 dark:hover:bg-white/10'}"
>
  <div class="flex w-full items-stretch">
    <div class="min-w-0 flex flex-1 flex-col">
      <button
        type="button"
        class="w-full cursor-pointer rounded-l-md border-0 bg-transparent px-2 py-1.5 text-left text-inherit shadow-none outline-none focus-visible:ring-1 focus-visible:ring-blue-400 dark:focus-visible:ring-blue-300"
        aria-label="Open {rowLabel}"
        onclick={(e) => activateRow(e.metaKey)}
      >
        <div class="flex items-center gap-2">
          <span
            class="h-2 w-2 shrink-0 rounded-full {dirtyDotClass(repo)}"
            aria-hidden="true"
          ></span>
          <span class="truncate font-medium">{rowLabel}</span>
          {#if repo.branch}
            <span
              class="truncate text-xs {selected
                ? 'text-blue-100'
                : 'text-neutral-500 dark:text-neutral-400'}"
            >
              {repo.branch}
            </span>
          {/if}
        </div>
        {#if repo.parent_dir}
          <div
            class="mt-0.5 truncate pl-4 text-xs {selected
              ? 'text-blue-100/90'
              : 'text-neutral-500 dark:text-neutral-400'}"
          >
            {repo.parent_dir}
          </div>
        {/if}
      </button>
      {#if repo.tags.length > 0}
        <div class="flex flex-wrap gap-1 px-2 pb-1.5 pl-6">
          {#each repo.tags as tag (tag)}
            <TagChip
              {tag}
              onRemove={onTagRemove ? () => void onTagRemove(tag) : undefined}
              onFilter={onTagFilter ? () => onTagFilter(tag) : undefined}
            />
          {/each}
        </div>
      {/if}
    </div>
    <button
      type="button"
      class="flex w-6 shrink-0 cursor-pointer items-center justify-center self-stretch border-0 px-0 text-xs leading-none shadow-none outline-none focus-visible:ring-1 focus-visible:ring-blue-400 dark:focus-visible:ring-blue-300 {selected
        ? 'bg-blue-700 text-blue-50 hover:bg-blue-800 hover:text-white dark:bg-blue-600 dark:hover:bg-blue-700'
        : 'border-l border-neutral-200/80 bg-neutral-100 text-neutral-500 hover:bg-neutral-200 hover:text-neutral-800 group-hover:bg-neutral-100 group-hover:text-neutral-700 dark:border-neutral-700 dark:bg-neutral-800/50 dark:text-neutral-400 dark:hover:bg-neutral-700 dark:hover:text-neutral-200 dark:group-hover:bg-neutral-800/70'}"
      aria-label="Open detail for {rowLabel}"
      onclick={(e) => {
        e.stopPropagation();
        onDetail();
      }}
    >
      ▶
    </button>
  </div>
</li>
