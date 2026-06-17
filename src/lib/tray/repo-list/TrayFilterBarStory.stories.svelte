<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { expect, fn } from "storybook/test";
  import TrayFilterBarStory from "./TrayFilterBarStory.svelte";

  const noopBind = (() => {}) as (el: HTMLInputElement | null) => void;

  const { Story } = defineMeta({
    title: "Composites/FilterBar",
    component: TrayFilterBarStory,
    tags: ["autodocs"],
    args: {
      allTags: ["rust", "frontend"],
      onFilterKeydown: fn(),
      onTagSelect: fn(),
      bindFilterInput: noopBind,
      onRefresh: fn(),
      refreshing: false,
    },
  });
</script>

<Story
  name="ShowsTagAutocompleteOnHash"
  play={async ({ canvas, userEvent }) => {
    const input = canvas.getByPlaceholderText("Filter repos…");
    await userEvent.clear(input);
    await userEvent.type(input, "#rust");
    await expect(canvas.getByRole("listbox")).toBeVisible();
    await expect(canvas.getByRole("option", { name: "#rust" })).toBeVisible();
  }}
/>
