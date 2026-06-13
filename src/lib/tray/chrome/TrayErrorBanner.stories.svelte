<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
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
