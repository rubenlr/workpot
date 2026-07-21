<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { expect, waitFor } from "storybook/test";
  import DetailPane from "./DetailPane.svelte";
  import { storyRepo } from "$lib/tray/repo-list/repoStoryFixtures";

  const noop = () => {};

  const fullPaneArgs = {
    repo: storyRepo({
      alias: "workpot",
      pinned: true,
      tags: ["rust"],
      notes: "Tray HUD reskin",
      branch: "main",
      ahead: 0,
      behind: 0,
    }),
  };

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

<Story name="FullPane" args={fullPaneArgs} />

<Story
  name="ShowAllExpanded"
  args={fullPaneArgs}
  play={async ({ canvas, userEvent }) => {
    await waitFor(() => {
      expect(canvas.getByRole("button", { name: "Show all" })).toBeVisible();
    });
    await userEvent.click(canvas.getByRole("button", { name: "Show all" }));
    await expect(
      canvas.getByRole("button", { name: "Show less" }),
    ).toBeVisible();
    await expect(
      canvas.getByRole("button", { name: "Activate branch feat/ui" }),
    ).toBeVisible();
  }}
/>
