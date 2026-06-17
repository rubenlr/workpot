<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import TrayFilterBar from "./TrayFilterBar.svelte";

  const noop = () => {};
  const noopBind = (() => {}) as (el: HTMLInputElement | null) => void;

  const { Story } = defineMeta({
    title: "Composites/FilterBar",
    component: TrayFilterBar,
    tags: ["autodocs"],
    args: {
      filterQuery: "",
      allTags: ["rust", "frontend"],
      tagAutocompletePrefix: "",
      onFilterKeydown: noop,
      onTagSelect: noop,
      bindFilterInput: noopBind,
      onRefresh: noop,
      refreshing: false,
    },
  });
</script>

<Story name="Default" />

<Story
  name="TagAutocompleteVisible"
  args={{ filterQuery: "#rust", tagAutocompletePrefix: "rust" }}
/>

<Story name="Refreshing" args={{ refreshing: true }} />
