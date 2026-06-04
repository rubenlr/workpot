<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import TrayPanelChrome from "./TrayPanelChrome.svelte";
  import {
    emptySectionedRepos,
    storyFlatIndexByPath,
    storyListViews,
    storySectionedRepos,
    storyTrayRepos,
  } from "./trayPanelStoryFixtures";

  const noop = () => {};
  const noopAsync = async () => {};
  const noopBindFilter = (() => {}) as (el: HTMLInputElement | null) => void;

  const sectioned = storySectionedRepos();
  const flatIndex = storyFlatIndexByPath(sectioned);

  const { Story } = defineMeta({
    title: "Tray/Panel",
    component: TrayPanelChrome,
    args: {
      listMaxHeightPx: 480,
      filterQuery: "",
      allTags: ["rust", "frontend"],
      tagAutocompletePrefix: null,
      onFilterKeydown: noop,
      onTagSelect: noop,
      bindFilterInput: noopBindFilter,
      listView: storyListViews.list,
      sectionedRepos: sectioned,
      flatIndexByPath: flatIndex,
      selectedIndex: 1,
      onPinReorder: noopAsync,
      onSelectRow: noop,
      onOpen: noop,
      onDetail: noop,
      onTagRemove: noopAsync,
      onTagFilter: noop,
    },
  });
</script>

<Story name="PopulatedList" />

<Story
  name="EmptyList"
  args={{
    listView: storyListViews.emptyList,
    sectionedRepos: emptySectionedRepos(),
    flatIndexByPath: new Map(),
    selectedIndex: 0,
    allTags: [],
  }}
/>

<Story
  name="NoMatch"
  args={{
    filterQuery: "zzzz",
    listView: storyListViews.noMatch,
    sectionedRepos: storySectionedRepos(storyTrayRepos()),
    flatIndexByPath: new Map(),
    selectedIndex: 0,
  }}
/>

<Story
  name="ListError"
  args={{
    listView: storyListViews.error,
    sectionedRepos: emptySectionedRepos(),
    flatIndexByPath: new Map(),
    selectedIndex: 0,
    allTags: [],
  }}
/>

<Story
  name="WithLaunchError"
  args={{
    launchError: "Could not launch Cursor for /tmp/workpot",
    onDismissLaunchError: noop,
  }}
/>

<Story
  name="DetailOpen"
  args={{
    detailRepo: storyTrayRepos()[0],
    focusTagOnDetailOpen: false,
    onCloseDetail: noop,
    onDetailMutated: noop,
    onTagFocusDone: noop,
  }}
/>
