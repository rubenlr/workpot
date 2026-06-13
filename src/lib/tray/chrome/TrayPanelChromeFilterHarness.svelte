<script lang="ts">
  import TrayPanelChromeStory from "./TrayPanelChromeStory.svelte";

  const noop = () => {};
  const noopAsync = async () => {};
</script>

<TrayPanelChromeStory
  listMaxHeightPx={480}
  allTags={["rust", "frontend"]}
  onFilterKeydown={noop}
  onTagSelect={noop}
  onRefresh={noop}
  onPinReorder={noopAsync}
  onSelectRow={noop}
  onOpen={noop}
  onDetail={noop}
/>
