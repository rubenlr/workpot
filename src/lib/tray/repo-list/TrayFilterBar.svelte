<script lang="ts">
  import MaterialIcon from "$lib/tray/commons/MaterialIcon.svelte";
  import TagAutocomplete from "$lib/tray/commons/TagAutocomplete.svelte";

  let {
    filterQuery = $bindable(""),
    allTags,
    tagAutocompletePrefix,
    onFilterKeydown,
    onTagSelect,
    bindFilterInput,
    onRefresh,
    refreshing = false,
  }: {
    filterQuery?: string;
    allTags: string[];
    tagAutocompletePrefix: string;
    onFilterKeydown: (e: KeyboardEvent) => void;
    onTagSelect: (tag: string) => void;
    bindFilterInput: (el: HTMLInputElement | null) => void;
    onRefresh?: () => void;
    refreshing?: boolean;
  } = $props();

  let filterInput = $state<HTMLInputElement | null>(null);

  $effect(() => {
    bindFilterInput(filterInput);
  });
</script>

<div class="border-b border-white/10 bg-inverse-surface px-3 py-2">
  <div class="relative flex items-center gap-2">
    <div class="relative min-w-0 flex-1">
      <MaterialIcon
        name="search"
        size={18}
        class="pointer-events-none absolute left-2.5 top-1/2 -translate-y-1/2 text-inverse-on-surface-variant"
      />
      <input
        id="repo-filter"
        bind:this={filterInput}
        type="search"
        placeholder="Filter repos…"
        maxlength="256"
        class="w-full rounded-lg border border-white/10 bg-black/20 py-2 pl-9 pr-3 text-sm text-inverse-on-surface outline-none placeholder:text-inverse-on-surface-variant focus:border-primary/50 focus:ring-1 focus:ring-primary"
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
    {#if onRefresh}
      <button
        type="button"
        class="flex shrink-0 cursor-pointer items-center justify-center rounded-lg border border-white/10 bg-black/20 p-2 text-inverse-on-surface-variant outline-none hover:bg-white/10 hover:text-inverse-on-surface focus-visible:ring-1 focus-visible:ring-primary disabled:opacity-50"
        aria-label="Refresh index"
        disabled={refreshing}
        onclick={onRefresh}
      >
        <MaterialIcon
          name="sync"
          size={20}
          class={refreshing ? "animate-spin" : ""}
        />
      </button>
    {/if}
  </div>
</div>
