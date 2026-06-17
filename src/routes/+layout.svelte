<script lang="ts">
  import { onMount } from "svelte";
  import "../app.css";
  import { initSystemThemeSync } from "$lib/tray/logic/theme/syncSystemTheme";

  let { children } = $props();

  onMount(() => {
    let unlisten: (() => void) | undefined;
    void initSystemThemeSync().then((dispose) => {
      unlisten = dispose;
    });
    return () => unlisten?.();
  });
</script>

{@render children()}
