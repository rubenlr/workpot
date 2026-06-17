<script lang="ts">
  import {
    branchBadgeAriaLabel,
    branchBadgeTitle,
    branchTrackingIcon,
    formatBranchAheadBehind,
  } from "$lib/branchStatus";
  import type { BranchListItemDto } from "$lib/types";

  let { branch }: { branch: BranchListItemDto } = $props();

  const syncSuffix = $derived(
    formatBranchAheadBehind(branch.ahead, branch.behind),
  );
</script>

<span
  class="inline-flex max-w-full items-center gap-0.5 rounded-full px-2 py-0.5 text-xs font-medium
    {branch.checked_out
    ? 'bg-tag-blue-bg/15 text-tag-blue-text'
    : 'bg-card-surface text-inverse-on-surface-variant'}"
  title={branchBadgeTitle(branch)}
  aria-label={branchBadgeAriaLabel(branch)}
>
  <span class="shrink-0 leading-none" aria-hidden="true"
    >{branchTrackingIcon(branch.tracking)}</span
  >
  <span class="min-w-0 truncate">{branch.name}</span>
  {#if syncSuffix}
    <span class="shrink-0 tabular-nums leading-none" aria-hidden="true"
      >{syncSuffix}</span
    >
  {/if}
</span>
