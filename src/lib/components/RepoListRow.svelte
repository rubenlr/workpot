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

  function activateRow(metaKey: boolean) {
    if (metaKey) {
      onDetail();
    } else {
      onOpen();
    }
  }
</script>

<div
  role="option"
  aria-selected={selected}
  tabindex={selected ? 0 : -1}
  data-row-index={rowIndex}
  draggable={listRowDraggable ? "true" : undefined}
  oncontextmenu={onRowContextMenu}
  ondragstart={onRowDragStart}
  ondragover={onRowDragOver}
  ondrop={onRowDrop}
  ondragend={onRowDragEnd}
  class="relative w-full rounded-md text-left {selected
    ? 'bg-blue-600 text-white dark:bg-blue-500'
    : 'hover:bg-black/5 dark:hover:bg-white/10'}"
>
  <button
    type="button"
    class="w-full cursor-pointer rounded-md border-0 bg-transparent px-2 py-1.5 pr-8 text-left text-inherit shadow-none outline-none focus-visible:ring-1 focus-visible:ring-blue-400 dark:focus-visible:ring-blue-300"
    aria-label="Open {repo.alias ?? repo.name}"
    onclick={(e) => activateRow(e.metaKey)}
  >
    <div class="flex items-center gap-2">
      <span
        class="h-2 w-2 shrink-0 rounded-full {dirtyDotClass(repo)}"
        aria-hidden="true"
      ></span>
      <span class="truncate font-medium">{repo.alias ?? repo.name}</span>
      {#if repo.branch}
        <span
          class="truncate text-xs {selected
            ? 'text-blue-100'
            : 'text-neutral-500'}"
        >
          {repo.branch}
        </span>
      {/if}
    </div>
    {#if repo.parent_dir}
      <div
        class="mt-0.5 truncate pl-4 text-xs {selected
          ? 'text-blue-100/90'
          : 'text-neutral-500'}"
      >
        {repo.parent_dir}
      </div>
    {/if}
  </button>
  <button
    type="button"
    class="absolute right-2 top-1.5 shrink-0 rounded px-1 text-xs {selected
      ? 'text-blue-100 hover:text-white'
      : 'text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200'}"
    aria-label="Open detail"
    onclick={(e) => {
      e.stopPropagation();
      onDetail();
    }}
  >
    ⓘ
  </button>
  {#if repo.tags.length > 0}
    <div class="mt-1 flex flex-wrap gap-1 px-2 pb-1.5 pl-6">
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
