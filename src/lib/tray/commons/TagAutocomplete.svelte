<script lang="ts">
  import { filterTagsForAutocomplete } from "$lib/tagAutocomplete";

  const listboxId = "tag-autocomplete-listbox";

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

  function onOptionKeydown(e: KeyboardEvent, tag: string) {
    if (e.key === "Enter" || e.key === " ") {
      e.preventDefault();
      selectTag(tag);
    }
  }
</script>

{#if visible && filtered.length > 0}
  <div
    class="absolute z-10 mt-1 w-48 rounded-lg border border-card-border bg-inverse-surface py-1 shadow-lg"
  >
    <input
      type="text"
      role="combobox"
      aria-expanded="true"
      aria-controls={listboxId}
      aria-autocomplete="list"
      aria-label="Filter tags"
      bind:value={inputValue}
      onkeydown={onKeydown}
      class="w-full border-0 border-b border-card-border bg-transparent px-3 py-1.5 text-sm text-inverse-on-surface outline-none placeholder:text-inverse-on-surface-variant"
      placeholder="Filter tags…"
    />
    <ul
      id={listboxId}
      role="listbox"
      aria-label="Tag suggestions"
      class="max-h-40 overflow-y-auto"
    >
      {#each filtered as tag, i (tag)}
        <li
          role="option"
          aria-selected={i === highlightedIndex}
          tabindex="-1"
          class="cursor-pointer px-3 py-1 text-left text-sm text-inverse-on-surface {i ===
          highlightedIndex
            ? 'bg-hover-overlay'
            : ''}"
          onmouseenter={() => {
            highlightedIndex = i;
          }}
          onclick={() => selectTag(tag)}
          onkeydown={(e) => onOptionKeydown(e, tag)}
        >
          #{tag}
        </li>
      {/each}
    </ul>
  </div>
{/if}
