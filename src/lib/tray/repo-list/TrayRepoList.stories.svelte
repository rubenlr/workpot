<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import TrayRepoListStory from "./TrayRepoListStory.svelte";
  import {
    emptySectionedRepos,
    storyFlatIndexByPath,
    storySectionedRepos,
  } from "$lib/tray/storybook/trayPanelStoryFixtures";
  import { STORY_REPO_PATH_PREFIX } from "$lib/tray/repo-list/repoStoryFixtures";

  const noop = () => {};
  const noopAsync = async () => {};

  const sectioned = storySectionedRepos();
  const flatIndex = storyFlatIndexByPath(sectioned);
  const alphaPath = `${STORY_REPO_PATH_PREFIX}/alpha`;

  const { Story } = defineMeta({
    title: "Composites/TrayRepoList",
    component: TrayRepoListStory,
    tags: ["autodocs"],
    args: {
      sectionedRepos: sectioned,
      flatIndexByPath: flatIndex,
      selectedIndex: 0,
      onPinReorder: noopAsync,
      onSelectRow: noop,
      onOpen: noop,
      onDetail: noop,
      onSync: noop,
    },
  });
</script>

<Story
  name="EmptyShell"
  args={{
    sectionedRepos: emptySectionedRepos(),
    flatIndexByPath: new Map(),
  }}
/>

<Story name="MultiSection" />

<Story name="SelectedRow" args={{ selectedIndex: 1 }} />

<Story
  name="ActiveSync"
  args={{
    activeSync: {
      repoPath: alphaPath,
      branch: "feat/ui",
      direction: "push",
    },
  }}
/>
