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

  function onclick(e: MouseEvent) {
    if (e.metaKey) {
      if (onRemove) {
        onRemove();
      }
    } else if (onFilter) {
      onFilter();
    }
  }
</script>

<button
  type="button"
  {title}
  {onclick}
  class="inline-flex cursor-default items-center rounded-full bg-blue-100 px-2 py-0.5 text-xs font-medium text-blue-800 dark:bg-blue-900 dark:text-blue-200"
>
  #{tag}
</button>
