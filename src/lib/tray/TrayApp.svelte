<script lang="ts">
  import { onMount } from "svelte";
  import { createTrayPanel } from "$lib/tray/state/createTrayPanel.svelte";
  import TrayPanelChrome from "./chrome/TrayPanelChrome.svelte";

  const panel = createTrayPanel();

  onMount(() => {
    void panel.mount();
    return () => panel.destroy();
  });
</script>

<svelte:window onkeydown={panel.onPanelKeydown} />

<TrayPanelChrome
  listMaxHeightPx={panel.listMaxHeightPx}
  launchError={panel.launchError}
  onDismissLaunchError={panel.dismissLaunchError}
  bind:filterQuery={panel.filterQuery}
  allTags={panel.allTags}
  tagAutocompletePrefix={panel.tagAutocompletePrefix}
  onFilterKeydown={panel.onFilterKeydown}
  onTagSelect={panel.onTagAutocompleteSelect}
  bindFilterInput={panel.bindFilterInput}
  listView={panel.listView}
  sectionedRepos={panel.sectionedRepos}
  flatIndexByPath={panel.flatIndexByPath}
  bind:selectedIndex={panel.selectedIndex}
  onPinReorder={panel.handlePinReorder}
  onSelectRow={(idx) => {
    panel.selectedIndex = idx;
  }}
  onOpen={() => void panel.openSelected(false)}
  onDetail={(repo, idx) => {
    panel.selectedIndex = idx;
    panel.openDetail(repo);
  }}
  onRefresh={() => void panel.startBackgroundRefresh()}
  detailRepo={panel.detailRepo}
  focusTagOnDetailOpen={panel.focusTagOnDetailOpen}
  onTagFocusDone={panel.clearTagFocusRequest}
  onCloseDetail={panel.closeDetail}
  onDetailMutated={() => void panel.refreshReposAndDetail()}
/>
