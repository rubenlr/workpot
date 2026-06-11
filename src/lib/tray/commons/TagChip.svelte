<script lang="ts">
  import { tagChipTitle } from "$lib/tagChip";

  let {
    tag,
    variant = "list",
    onRemove,
    onFilter,
  }: {
    tag: string;
    variant?: "list" | "detail";
    onRemove?: () => void;
    onFilter?: () => void;
  } = $props();

  const title = $derived(tagChipTitle(!!onRemove, !!onFilter));

  const chipClass = $derived(
    variant === "detail"
      ? "bg-tag-blue-bg/10 border border-tag-blue-text/30 text-tag-blue-text"
      : "bg-blue-100 text-blue-800 dark:bg-blue-900 dark:text-blue-200",
  );

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
  class="inline-flex max-w-full items-center gap-0.5 rounded-full py-0.5 pl-2 pr-0.5 text-xs font-medium {chipClass}"
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
      class="shrink-0 rounded-full px-1 leading-none opacity-70 hover:opacity-100"
      aria-label="Remove tag {tag}"
      onclick={onRemoveClick}
    >
      ×
    </button>
  {/if}
</span>
