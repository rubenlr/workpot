<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { expect, fn } from "storybook/test";
  import RemoteDetailPane from "./RemoteDetailPane.svelte";
  import { storyRemoteRepo } from "$lib/tray/repo-list/repoStoryFixtures";

  const { Story } = defineMeta({
    title: "Composites/RemoteDetailPane",
    component: RemoteDetailPane,
    tags: ["autodocs"],
    parameters: { layout: "fullscreen" },
    args: {
      repo: storyRemoteRepo(),
      onClone: fn(),
      onClose: fn(),
    },
  });
</script>

<Story
  name="Manual"
  play={async ({ canvas, userEvent, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "Clone" }));
    await expect(args.onClone).toHaveBeenCalledOnce();
  }}
/>

<Story
  name="CloneOnOpen"
  args={{ repo: storyRemoteRepo({ clone_policy: "on_open" }) }}
/>

<Story
  name="AutoClone"
  args={{ repo: storyRemoteRepo({ clone_policy: "auto" }) }}
/>

<Story name="Cloning" args={{ cloning: true }} />
