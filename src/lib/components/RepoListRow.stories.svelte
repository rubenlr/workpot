<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import RepoListRowStory from "./RepoListRowStory.svelte";
  import { storyRepo } from "./repoStoryFixtures";

  const noop = () => {};

  const { Story } = defineMeta({
    title: "Tray/RepoListRow",
    component: RepoListRowStory,
    args: {
      onOpen: noop,
      onDetail: noop,
    },
  });
</script>

<Story
  name="Default"
  args={{
    repo: storyRepo({
      alias: null,
      branch: "main",
      is_dirty: null,
      pinned: false,
    }),
    selected: false,
  }}
/>

<Story
  name="Selected"
  args={{
    repo: storyRepo({
      alias: null,
      branch: "main",
      is_dirty: null,
      pinned: false,
    }),
    selected: true,
  }}
/>

<Story
  name="Dirty"
  args={{
    repo: storyRepo({
      alias: null,
      branch: "main",
      is_dirty: true,
      pinned: false,
    }),
    selected: false,
  }}
/>

<Story
  name="Pinned"
  args={{
    repo: storyRepo({
      alias: null,
      branch: "main",
      is_dirty: false,
      pinned: true,
    }),
    selected: false,
  }}
/>

<Story
  name="WithAlias"
  args={{
    repo: storyRepo({
      alias: "my-project",
      name: "workpot-folder",
      branch: "feat/alias",
      is_dirty: false,
      pinned: false,
    }),
    selected: false,
  }}
/>

<Story
  name="BareRepo"
  args={{
    repo: storyRepo({
      alias: null,
      branch: null,
      is_dirty: null,
      pinned: false,
    }),
    selected: false,
  }}
/>
