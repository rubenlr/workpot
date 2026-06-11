<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import DetailPaneHeader from "./DetailPaneHeader.svelte";
  import { storyRepo } from "$lib/tray/repo-list/repoStoryFixtures";

  const noop = () => {};

  const { Story } = defineMeta({
    title: "Tray/DetailPaneHeader",
    component: DetailPaneHeader,
    args: {
      onClose: noop,
      onPinToggle: noop,
    },
  });
</script>

<Story
  name="AliasSet"
  args={{ repo: storyRepo({ alias: "my-project", pinned: false }) }}
/>
<Story
  name="Pinned"
  args={{ repo: storyRepo({ alias: null, pinned: true }) }}
/>
<Story
  name="DarkSurface"
  args={{ repo: storyRepo({ alias: "alpha", pinned: false }) }}
/>
