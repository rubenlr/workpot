<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import TrayListBodyStory from "./TrayListBodyStory.svelte";
  import {
    storyFlatIndexByPath,
    storyListViews,
    storySectionedRepos,
    storyTrayRepos,
    emptySectionedRepos,
  } from "$lib/tray/storybook/trayPanelStoryFixtures";

  const noop = () => {};
  const noopAsync = async () => {};

  const sectioned = storySectionedRepos();
  const flatIndex = storyFlatIndexByPath(sectioned);

  const { Story } = defineMeta({
    title: "Composites/TrayListBody",
    component: TrayListBodyStory,
    tags: ["autodocs"],
    args: {
      listView: storyListViews.list,
      sectionedRepos: sectioned,
      flatIndexByPath: flatIndex,
      selectedIndex: 0,
      onPinReorder: noopAsync,
      onSelectRow: noop,
      onOpen: noop,
      onDetail: noop,
    },
  });
</script>

<Story
  name="EmptyList"
  args={{
    listView: storyListViews.emptyList,
    sectionedRepos: emptySectionedRepos(),
    flatIndexByPath: new Map(),
  }}
/>

<Story
  name="NoMatch"
  args={{
    listView: storyListViews.noMatch,
    sectionedRepos: storySectionedRepos(storyTrayRepos()),
    flatIndexByPath: new Map(),
  }}
/>

<Story name="List" />

<Story
  name="CustomMessages"
  args={{
    listView: storyListViews.emptyList,
    emptyListMessage: "No repositories found in watch roots.",
    noMatchMessage: "Nothing matched your filter.",
    sectionedRepos: emptySectionedRepos(),
    flatIndexByPath: new Map(),
  }}
/>

<Story
  name="ErrorKind"
  parameters={{
    docs: {
      description: {
        story:
          "TrayListBody does not render error UI — list errors are shown in TrayPanelChrome via TrayErrorBanner. An error listView renders an empty body.",
      },
    },
  }}
  args={{
    listView: storyListViews.error,
    sectionedRepos: emptySectionedRepos(),
    flatIndexByPath: new Map(),
  }}
/>
