<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import { expect, fn } from "storybook/test";
  import TrayErrorBanner from "./TrayErrorBanner.svelte";

  const noop = () => {};

  const { Story } = defineMeta({
    title: "Components/TrayErrorBanner",
    component: TrayErrorBanner,
    tags: ["autodocs"],
    args: {
      message: "Could not launch Cursor for /tmp/workpot",
    },
  });
</script>

<Story name="AlertOnly" />
<Story name="Dismissible" args={{ onDismiss: noop }} />

<Story
  name="DismissesOnClick"
  args={{
    message: "Could not launch Cursor for /tmp/workpot",
    onDismiss: fn(),
  }}
  play={async ({ canvas, userEvent, args }) => {
    await userEvent.click(canvas.getByRole("button", { name: "DISMISS" }));
    await expect(args.onDismiss).toHaveBeenCalledOnce();
  }}
/>
