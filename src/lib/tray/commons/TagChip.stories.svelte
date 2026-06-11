<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import TagChip from "./TagChip.svelte";

  const noop = () => {};

  const { Story } = defineMeta({
    title: "Components/TagChip",
    component: TagChip,
  });
</script>

<Story name="List" args={{ tag: "rust", variant: "list" }} />
<Story
  name="DetailWithRemove"
  args={{ tag: "frontend", variant: "detail", onRemove: noop }}
/>
