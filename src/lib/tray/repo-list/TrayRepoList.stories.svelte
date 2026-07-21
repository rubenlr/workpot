<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { expect, fireEvent, waitFor } from "storybook/test";
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
      contextMenuFeedback: null,
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

<Story
  name="ContextMenuDelegated"
  args={{
    sectionedRepos: {
      ...emptySectionedRepos(),
      rest: [
        {
          path: "/tmp/story/workpot",
          name: "workpot",
          alias: null,
          branch: "main",
          ahead: null,
          behind: null,
          is_dirty: false,
          parent_dir: "~/tmp/story",
          last_opened_at: null,
          git_state_error: null,
          pinned: false,
          pin_order: null,
          notes: null,
          tags: ["backend"],
          branches: [],
          is_bare: false,
          convert_to: null,
          convert_block_reason: null,
        },
      ],
    },
    flatIndexByPath: new Map([["/tmp/story/workpot", 0]]),
    contextMenuFeedback:
      "Right-click a row to invoke show_repo_context_menu (mocked in Storybook)",
  }}
  play={async ({ canvas }) => {
    const openControl = canvas.getByRole("button", { name: "Open workpot" });
    await fireEvent.contextMenu(openControl, { clientX: 64, clientY: 24 });
    await waitFor(() => {
      expect(canvas.getByTestId("context-menu-feedback")).toBeVisible();
    });
  }}
/>
