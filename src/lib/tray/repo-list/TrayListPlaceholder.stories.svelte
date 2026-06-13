<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import TrayListPlaceholder from "./TrayListPlaceholder.svelte";
  import { TRAY_EMPTY_LIST_MESSAGE } from "$lib/tray/logic/handlers/constants";

  const { Story } = defineMeta({
    title: "Components/TrayListPlaceholder",
    component: TrayListPlaceholder,
    tags: ["autodocs"],
    args: {
      message: TRAY_EMPTY_LIST_MESSAGE,
    },
  });
</script>

<Story name="Muted" />
<Story
  name="ErrorTone"
  args={{ message: "SQLite database is locked", tone: "error" }}
/>
