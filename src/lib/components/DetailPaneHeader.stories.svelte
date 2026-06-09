<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import DetailPane from "./DetailPane.svelte";
  import { storyRepo } from "./repoStoryFixtures";

  const noop = () => {};

  const { Story } = defineMeta({
    title: "Tray/DetailPaneHeader",
    component: DetailPane,
    args: {
      onClose: noop,
      onMutated: noop,
    },
  });
</script>

<Story
  name="AliasSet"
  args={{
    repo: storyRepo({ alias: "my-project", pinned: false }),
  }}
/>

<Story
  name="AliasNull"
  args={{
    repo: storyRepo({ alias: null, name: "workpot-folder", pinned: false }),
  }}
/>

<Story
  name="Pinned"
  args={{
    repo: storyRepo({ alias: null, pinned: true }),
  }}
/>

<Story
  name="Unpinned"
  args={{
    repo: storyRepo({ alias: null, pinned: false }),
  }}
/>
