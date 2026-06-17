<script lang="ts">
  import TrayFilterBar from "./TrayFilterBar.svelte";
  import { trailingTagAutocompletePrefix } from "$lib/tagFilter";

  let {
    allTags,
    onFilterKeydown,
    onTagSelect,
    bindFilterInput,
    onRefresh,
    refreshing = false,
    refreshSuccess = false,
  }: {
    allTags: string[];
    onFilterKeydown: (e: KeyboardEvent) => void;
    onTagSelect: (tag: string) => void;
    bindFilterInput: (el: HTMLInputElement | null) => void;
    onRefresh?: () => void;
    refreshing?: boolean;
    refreshSuccess?: boolean;
  } = $props();

  let filterQuery = $state("");
  const tagAutocompletePrefix = $derived(
    trailingTagAutocompletePrefix(filterQuery) ?? "",
  );
</script>

<TrayFilterBar
  bind:filterQuery
  {allTags}
  {tagAutocompletePrefix}
  {onFilterKeydown}
  {onTagSelect}
  {bindFilterInput}
  {onRefresh}
  {refreshing}
  {refreshSuccess}
/>
