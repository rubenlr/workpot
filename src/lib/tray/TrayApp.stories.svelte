<script module lang="ts">
  import { defineMeta } from "@storybook/addon-svelte-csf";
  import TrayApp from "./TrayApp.svelte";

  const { Story } = defineMeta({
    title: "Pages/TrayApp",
    component: TrayApp,
    tags: ["autodocs"],
    parameters: {
      layout: "fullscreen",
      docs: {
        description: {
          component:
            "Full tray shell wired through createTrayPanel with Storybook Tauri mocks.",
        },
      },
    },
  });
</script>

<Story name="Default" />
