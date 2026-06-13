<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { expect, fn } from "storybook/test";
  import TagAutocompleteStory from "./TagAutocompleteStory.svelte";

  const noop = () => {};

  const { Story } = defineMeta({
    title: "Components/TagAutocomplete",
    component: TagAutocompleteStory,
    tags: ["autodocs"],
    parameters: {
      docs: {
        description: {
          component:
            "Keyboard navigation (ArrowUp/ArrowDown/Enter) drives highlight state; use the OpenFullList story and interact in the Canvas.",
        },
      },
    },
    args: {
      allTags: ["rust", "frontend", "backend", "infra"],
      visible: true,
      prefix: "",
      onSelect: noop,
    },
  });
</script>

<Story name="Hidden" args={{ visible: false }} />
<Story name="OpenFullList" />
<Story name="PrefixFiltered" args={{ prefix: "fr" }} />
<Story name="EmptyResults" args={{ prefix: "zzzz" }} />

<Story
  name="SelectsTagWithKeyboard"
  args={{
    allTags: ["rust", "frontend", "backend", "infra"],
    visible: true,
    prefix: "",
    onSelect: fn(),
  }}
  play={async ({ canvas, userEvent, args }) => {
    const combobox = canvas.getByRole("combobox");
    await userEvent.click(combobox);
    await userEvent.keyboard("{ArrowDown}");
    await userEvent.keyboard("{Enter}");
    await expect(args.onSelect).toHaveBeenCalledWith("rust");
  }}
/>
