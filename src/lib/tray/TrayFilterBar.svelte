<script lang="ts">
  import TagAutocomplete from "$lib/components/TagAutocomplete.svelte";

  let {
    filterQuery = $bindable(""),
    allTags,
    tagAutocompletePrefix,
    onFilterKeydown,
    onTagSelect,
    bindFilterInput,
  }: {
    filterQuery?: string;
    allTags: string[];
    tagAutocompletePrefix: string;
    onFilterKeydown: (e: KeyboardEvent) => void;
    onTagSelect: (tag: string) => void;
    bindFilterInput: (el: HTMLInputElement | null) => void;
  } = $props();

  let filterInput = $state<HTMLInputElement | null>(null);

  $effect(() => {
    bindFilterInput(filterInput);
  });
</script>

<div
  class="border-b border-neutral-500/20 px-3 py-2 dark:border-neutral-400/15"
>
  <div class="relative flex items-center gap-2">
    <input
      id="repo-filter"
      bind:this={filterInput}
      type="search"
      placeholder="Filter repos…"
      maxlength="256"
      class="min-w-0 flex-1 rounded-md border border-neutral-500/25 bg-white/40 px-2 py-1.5 text-sm outline-none ring-blue-500 backdrop-blur-sm focus:ring-2 dark:border-neutral-400/20 dark:bg-black/25"
      bind:value={filterQuery}
      onkeydown={onFilterKeydown}
    />
    <TagAutocomplete
      {allTags}
      visible={filterQuery.includes("#")}
      prefix={tagAutocompletePrefix}
      onSelect={onTagSelect}
    />
  </div>
</div>
