<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import RepoListRowStory from "./RepoListRowStory.svelte";
  import {
    storyRepo,
    storyRepoWithSync,
  } from "$lib/tray/repo-list/repoStoryFixtures";

  const noop = () => {};

  const { Story } = defineMeta({
    title: "Components/RepoListRow",
    component: RepoListRowStory,
    tags: ["autodocs"],
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
      is_dirty: false,
      pinned: false,
    }),
    selected: true,
  }}
/>

<Story
  name="SelectedDirty"
  args={{
    repo: storyRepo({
      alias: null,
      branch: "feat/ui",
      is_dirty: true,
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
  name="AheadBehind"
  args={{
    repo: storyRepoWithSync({
      alias: null,
      branch: "feat/sync",
      is_dirty: false,
      pinned: false,
    }),
    selected: false,
  }}
/>

<Story
  name="GitError"
  args={{
    repo: storyRepo({
      alias: null,
      branch: null,
      is_dirty: null,
      git_state_error: "bare repo",
      pinned: false,
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
