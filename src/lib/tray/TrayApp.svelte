<script lang="ts">
  import { onMount } from "svelte";
  import DetailPane from "$lib/components/DetailPane.svelte";
  import { createTrayPanel } from "./createTrayPanel.svelte";
  import LaunchErrorBanner from "./LaunchErrorBanner.svelte";
  import TrayFilterBar from "./TrayFilterBar.svelte";
  import TrayRepoList from "./TrayRepoList.svelte";

  const panel = createTrayPanel();

  onMount(() => {
    panel.mount();
    return () => panel.destroy();
  });
</script>

<svelte:window onkeydown={panel.onPanelKeydown} />

<main
  class="panel-shell flex h-screen flex-col overflow-hidden rounded-xl text-neutral-900 shadow-2xl dark:text-neutral-100"
  style="max-height: {panel.listMaxHeightPx}px"
>
  {#if panel.launchError}
    <LaunchErrorBanner
      message={panel.launchError}
      onDismiss={panel.dismissLaunchError}
    />
  {/if}

  <TrayFilterBar
    bind:filterQuery={panel.filterQuery}
    allTags={panel.allTags}
    refreshing={panel.refreshing}
    tagAutocompletePrefix={panel.tagAutocompletePrefix}
    onFilterKeydown={panel.onFilterKeydown}
    onTagSelect={panel.onTagAutocompleteSelect}
    bindFilterInput={panel.bindFilterInput}
  />

  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if panel.detailRepo}
      <DetailPane
        repo={panel.detailRepo}
        allTags={panel.allTags}
        requestTagFocus={panel.focusTagOnDetailOpen}
        onTagFocusDone={panel.clearTagFocusRequest}
        onClose={panel.closeDetail}
        onMutated={() => void panel.refreshReposAndDetail()}
      />
    {:else if panel.listView.kind === "error"}
      <p class="text-sm text-red-600 dark:text-red-400">{panel.listView.message}</p>
    {:else if panel.listView.kind === "empty-index"}
      <p class="text-sm text-neutral-500">No repos indexed yet.</p>
    {:else if panel.listView.kind === "no-match"}
      <p class="text-sm text-neutral-500">No repos match</p>
    {:else}
      <TrayRepoList
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
        onTagRemove={panel.removeTagFromRepo}
        onTagFilter={panel.appendTagFilter}
      />
    {/if}
  </div>
</main>
