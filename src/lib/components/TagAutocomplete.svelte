<script lang="ts">
  import { filterTagsForAutocomplete } from "../tagAutocomplete";

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
    class="absolute z-10 mt-1 w-48 rounded-lg border border-neutral-200 bg-white py-1 shadow-lg dark:border-neutral-700 dark:bg-neutral-800"
    onkeydown={onKeydown}
  >
    <input
      type="text"
      bind:value={inputValue}
      class="w-full border-0 border-b border-neutral-200 bg-transparent px-3 py-1.5 text-sm outline-none dark:border-neutral-700"
      placeholder="Filter tags…"
    />
    <ul class="max-h-40 overflow-y-auto">
      {#each filtered as tag, i (tag)}
        <li>
          <button
            type="button"
            role="option"
            aria-selected={i === highlightedIndex}
            class="w-full px-3 py-1 text-left text-sm hover:bg-neutral-100 dark:hover:bg-neutral-700 {i ===
            highlightedIndex
              ? 'bg-neutral-100 dark:bg-neutral-700'
              : ''}"
            onclick={() => selectTag(tag)}
          >
            #{tag}
          </button>
        </li>
      {/each}
    </ul>
  </div>
{/if}
