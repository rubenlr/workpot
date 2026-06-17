<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { expect, fn } from "storybook/test";
  import TrayPanelChrome from "./TrayPanelChrome.svelte";
  import {
    emptySectionedRepos,
    storyFlatIndexByPath,
    storyListViews,
    storySectionedRepos,
    storyTrayRepos,
  } from "$lib/tray/storybook/trayPanelStoryFixtures";

  const noop = () => {};
  const noopAsync = async () => {};
  const noopBindFilter = (() => {}) as (el: HTMLInputElement | null) => void;

  const sectioned = storySectionedRepos();
  const flatIndex = storyFlatIndexByPath(sectioned);

  const { Story } = defineMeta({
    title: "Composites/Panel",
    component: TrayPanelChrome,
    tags: ["autodocs"],
    parameters: { layout: "fullscreen" },
    args: {
      listMaxHeightPx: 480,
      filterQuery: "",
      allTags: ["rust", "frontend"],
      tagAutocompletePrefix: null,
      onFilterKeydown: noop,
      onTagSelect: noop,
      bindFilterInput: noopBindFilter,
      onRefresh: noop,
      refreshing: false,
      listView: storyListViews.list,
      sectionedRepos: sectioned,
      flatIndexByPath: flatIndex,
      selectedIndex: 1,
      onPinReorder: noopAsync,
      onSelectRow: noop,
      onOpen: noop,
      onDetail: noop,
    },
  });
</script>

<Story name="PopulatedList" />

<Story name="DarkMode" parameters={{ backgrounds: { default: "dark" } }} />

<Story name="LightMode" parameters={{ backgrounds: { default: "light" } }} />

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
    listError: "SQLite database is locked",
    listView: storyListViews.error,
    sectionedRepos: emptySectionedRepos(),
    flatIndexByPath: new Map(),
    selectedIndex: 0,
    allTags: [],
  }}
/>

<Story
  name="ListErrorWithCachedRepos"
  args={{
    listError: "git push failed: rejected",
    listView: storyListViews.list,
    sectionedRepos: sectioned,
    flatIndexByPath: flatIndex,
    selectedIndex: 0,
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

<Story
  name="DismissListError"
  args={{
    listError: "git push failed: rejected",
    listView: storyListViews.list,
    sectionedRepos: sectioned,
    flatIndexByPath: flatIndex,
    selectedIndex: 0,
    onDismissListError: fn(),
  }}
  play={async ({ canvas, userEvent, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "DISMISS" }));
    await expect(args.onDismissListError).toHaveBeenCalledOnce();
  }}
/>
