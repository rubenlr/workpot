<script lang="ts">
  import TagChip from "./TagChip.svelte";
  import { dirtyDotClass } from "$lib/repoRow";
  import type { RepoDto } from "$lib/types";

  let {
    repo,
    selected = false,
    onOpen,
    onDetail,
    onTagRemove,
    onTagFilter,
  }: {
    repo: RepoDto;
    selected?: boolean;
    onOpen: () => void;
    onDetail: () => void;
    onTagRemove?: (tag: string) => void | Promise<void>;
    onTagFilter?: (tag: string) => void;
  } = $props();
</script>

<div
  role="option"
  aria-selected={selected}
  class="w-full cursor-pointer rounded-md px-2 py-1.5 text-left {selected
    ? 'bg-blue-600 text-white dark:bg-blue-500'
    : 'hover:bg-black/5 dark:hover:bg-white/10'}"
  onclick={(e) => {
    if (e.metaKey) {
      onDetail();
    } else {
      onOpen();
    }
  }}
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
    <button
      type="button"
      class="ml-auto shrink-0 rounded px-1 text-xs text-neutral-400 hover:text-neutral-700 dark:hover:text-neutral-200"
      aria-label="Open detail"
      onclick={(e) => {
        e.stopPropagation();
        onDetail();
      }}
    >
      ⓘ
    </button>
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
  {#if repo.tags.length > 0}
    <div class="mt-1 flex flex-wrap gap-1 pl-4">
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
