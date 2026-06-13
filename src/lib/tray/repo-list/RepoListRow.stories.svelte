<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { expect, fireEvent, fn } from "storybook/test";
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

<Story
  name="OpensOnClick"
  args={{
    repo: storyRepo({
      name: "testrepo",
      alias: null,
      branch: "main",
      is_dirty: false,
      pinned: false,
    }),
    selected: false,
    onOpen: fn(),
    onDetail: fn(),
  }}
  play={async ({ canvas, userEvent, args }) => {
    await userEvent.click(
      canvas.getByRole("button", { name: "Open testrepo" }),
    );
    await expect(args.onOpen).toHaveBeenCalledOnce();
    await expect(args.onDetail).not.toHaveBeenCalled();
  }}
/>

<Story
  name="OpensDetailOnMetaClick"
  args={{
    repo: storyRepo({
      name: "testrepo",
      alias: null,
      branch: "main",
      is_dirty: false,
      pinned: false,
    }),
    selected: false,
    onOpen: fn(),
    onDetail: fn(),
  }}
  play={async ({ canvas, args }) => {
    const button = canvas.getByRole("button", { name: "Open testrepo" });
    await fireEvent.click(button, { metaKey: true });
    await expect(args.onDetail).toHaveBeenCalledOnce();
    await expect(args.onOpen).not.toHaveBeenCalled();
  }}
/>
