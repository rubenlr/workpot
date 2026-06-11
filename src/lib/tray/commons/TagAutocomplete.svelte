<script lang="ts">
  import { filterTagsForAutocomplete } from "$lib/tagAutocomplete";

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

  let inputValue = $state("");
  let highlightedIndex = $state(-1);

  let filtered = $derived(
    filterTagsForAutocomplete(allTags, prefix, inputValue),
  );

  $effect(() => {
    inputValue;
    prefix;
    highlightedIndex = -1;
  });

  function selectTag(tag: string) {
    onSelect(tag);
    inputValue = "";
    highlightedIndex = -1;
  }

  function onKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      e.preventDefault();
      if (filtered.length === 0) {
        return;
      }
      highlightedIndex = Math.min(
        highlightedIndex < 0 ? 0 : highlightedIndex + 1,
        filtered.length - 1,
      );
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      if (filtered.length === 0) {
        return;
      }
      highlightedIndex = Math.max(
        highlightedIndex < 0 ? 0 : highlightedIndex - 1,
        0,
      );
    } else if (e.key === "Enter") {
      e.preventDefault();
      if (
        highlightedIndex >= 0 &&
        highlightedIndex < filtered.length &&
        filtered[highlightedIndex]
      ) {
        selectTag(filtered[highlightedIndex]);
      } else if (inputValue.trim().length > 0) {
        selectTag(inputValue.trim());
      }
    }
  }
</script>

{#if visible}
  <div
    role="listbox"
    tabindex="-1"
    class="absolute z-10 mt-1 w-48 rounded-lg border border-card-border bg-inverse-surface py-1 shadow-lg"
    onkeydown={onKeydown}
  >
    <input
      type="text"
      bind:value={inputValue}
      class="w-full border-0 border-b border-card-border bg-transparent px-3 py-1.5 text-sm text-inverse-on-surface outline-none placeholder:text-inverse-on-surface-variant"
      placeholder="Filter tags…"
    />
    <ul class="max-h-40 overflow-y-auto">
      {#each filtered as tag, i (tag)}
        <li>
          <button
            type="button"
            role="option"
            aria-selected={i === highlightedIndex}
            class="w-full px-3 py-1 text-left text-sm text-inverse-on-surface {i ===
            highlightedIndex
              ? 'bg-white/10'
              : ''}"
            onmouseenter={() => {
              highlightedIndex = i;
            }}
            onclick={() => selectTag(tag)}
          >
            #{tag}
          </button>
        </li>
      {/each}
    </ul>
  </div>
{/if}
