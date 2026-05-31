<script lang="ts">
  import { tagChipTitle } from "../tagChip";

  let {
    tag,
    onRemove,
    onFilter,
  }: {
    tag: string;
    onRemove?: () => void;
    onFilter?: () => void;
  } = $props();

  const title = $derived(tagChipTitle(!!onRemove, !!onFilter));

  function onLabelClick(e: MouseEvent) {
    if (e.metaKey) {
      if (onRemove) {
        onRemove();
      }
      return;
    }
    if (onFilter) {
      onFilter();
    }
  }

  function onRemoveClick(e: MouseEvent) {
    e.stopPropagation();
    e.preventDefault();
    onRemove?.();
  }
</script>

<span
  class="inline-flex max-w-full items-center gap-0.5 rounded-full bg-blue-100 py-0.5 pl-2 pr-0.5 text-xs font-medium text-blue-800 dark:bg-blue-900 dark:text-blue-200"
  {title}
>
  <button
    type="button"
    class="min-w-0 truncate cursor-default"
    onclick={onLabelClick}
  >
    #{tag}
  </button>
  {#if onRemove}
    <button
      type="button"
      class="shrink-0 rounded-full px-1 leading-none text-blue-700 hover:bg-blue-200/80 dark:text-blue-200 dark:hover:bg-blue-800"
      aria-label="Remove tag {tag}"
      onclick={onRemoveClick}
    >
      ×
    </button>
  {/if}
</span>
