<script lang="ts">
  import TagAutocomplete from "./TagAutocomplete.svelte";

  let {
    allTags,
    visible,
    prefix = "",
    onSelect,
  }: {
    allTags: string[];
    visible: boolean;
    prefix?: string;
    onSelect: (tag: string) => void;
  } = $props();
</script>

<div class="relative w-64 bg-inverse-surface p-4 text-inverse-on-surface">
  <TagAutocomplete {allTags} {visible} {prefix} {onSelect} />
</div>
