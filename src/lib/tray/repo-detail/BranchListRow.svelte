<script lang="ts">
  import MaterialIcon from "$lib/tray/commons/MaterialIcon.svelte";
  import SyncBadge from "$lib/tray/commons/SyncBadge.svelte";
  import { branchPresenceLabel } from "$lib/branchStatus";
  import type { BranchListItemDto } from "$lib/types";

  let { branch }: { branch: BranchListItemDto } = $props();

  const isCheckout = $derived(branch.presence === "checkout");
  const isRemoteOnly = $derived(branch.presence === "remote_only");
</script>

<div
  class="flex items-center gap-2 rounded-lg px-3 py-2 {isCheckout
    ? 'bg-primary/15 ring-1 ring-primary/30'
    : 'bg-card-surface'}"
  title={branchPresenceLabel(branch.presence)}
>
  {#if isCheckout}
    <MaterialIcon name="check" size={18} class="text-primary" />
  {:else}
    <span class="w-[18px] shrink-0" aria-hidden="true"></span>
  {/if}
  <span
    class="min-w-0 flex-1 truncate text-sm {isCheckout
      ? 'font-medium text-inverse-on-surface'
      : 'text-inverse-on-surface-variant'}"
  >
    {branch.name}
  </span>
  {#if isRemoteOnly}
    <span
      class="shrink-0 rounded-full bg-white/10 px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-inverse-on-surface-variant"
    >
      remote
    </span>
  {/if}
  <SyncBadge ahead={branch.ahead} behind={branch.behind} />
</div>
