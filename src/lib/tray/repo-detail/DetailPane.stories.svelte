<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import DetailPane from "./DetailPane.svelte";
  import { storyRepo } from "$lib/tray/repo-list/repoStoryFixtures";

  const noop = () => {};

  const { Story } = defineMeta({
    title: "Composites/DetailPane",
    component: DetailPane,
    tags: ["autodocs"],
    parameters: { layout: "fullscreen" },
    args: {
      allTags: ["rust", "frontend", "backend"],
      onClose: noop,
      onMutated: noop,
    },
  });
</script>

<Story
  name="FullPane"
  args={{
    repo: storyRepo({
      alias: "workpot",
      pinned: true,
      tags: ["rust"],
      notes: "Tray HUD reskin",
      branch: "main",
      ahead: 0,
      behind: 0,
    }),
  }}
/>
