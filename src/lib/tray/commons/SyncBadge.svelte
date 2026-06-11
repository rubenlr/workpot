<script lang="ts">
  import MaterialIcon from "./MaterialIcon.svelte";
  import type { SyncDirection } from "$lib/types";

  let {
    ahead = null,
    behind = null,
    branch = null,
    syncingDirection = null,
    disabled = false,
    onPush,
    onPull,
    onHoverChange,
  }: {
    ahead?: number | null;
    behind?: number | null;
    branch?: string | null;
    syncingDirection?: SyncDirection | null;
    disabled?: boolean;
    onPush?: () => void;
    onPull?: () => void;
    onHoverChange?: (hovered: boolean) => void;
  } = $props();

  const showAhead = $derived(ahead != null && ahead > 0);
  const showBehind = $derived(behind != null && behind > 0);
  const interactive = $derived(
    branch != null && (onPush != null || onPull != null),
  );
  const pushInteractive = $derived(interactive && onPush != null);
  const pullInteractive = $derived(interactive && onPull != null);
  const pushSyncing = $derived(syncingDirection === "push");
  const pullSyncing = $derived(syncingDirection === "pull");
  const chipDisabled = $derived(disabled || pushSyncing || pullSyncing);

  function pushLabel(count: number): string {
    const branchLabel = branch ? ` on ${branch}` : "";
    return `Push ${count} commit${count === 1 ? "" : "s"}${branchLabel}`;
  }

  function pullLabel(count: number): string {
    const branchLabel = branch ? ` on ${branch}` : "";
    return `Pull ${count} commit${count === 1 ? "" : "s"}${branchLabel}`;
  }

  function handlePushClick(e: MouseEvent) {
    e.stopPropagation();
    if (!pushInteractive || chipDisabled || pushSyncing) {
      return;
    }
    onPush?.();
  }

  function handlePullClick(e: MouseEvent) {
    e.stopPropagation();
    if (!pullInteractive || chipDisabled || pullSyncing) {
      return;
    }
    onPull?.();
  }

  function handleMouseEnter() {
    onHoverChange?.(true);
  }

  function handleMouseLeave() {
    onHoverChange?.(false);
  }
</script>

{#if showAhead || showBehind}
  <div
    class="flex shrink-0 items-center gap-1"
    aria-hidden={interactive ? undefined : "true"}
    onmouseenter={onHoverChange ? handleMouseEnter : undefined}
    onmouseleave={onHoverChange ? handleMouseLeave : undefined}
  >
    {#if showAhead}
      {#if pushInteractive}
        <button
          type="button"
          class="inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface transition-colors hover:bg-primary/20 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary disabled:cursor-not-allowed disabled:opacity-70 {pushSyncing
            ? 'animate-pulse'
            : 'cursor-pointer'}"
          aria-label={pushLabel(ahead!)}
          disabled={chipDisabled && !pushSyncing}
          onclick={handlePushClick}
        >
          {#if pushSyncing}
            <MaterialIcon name="sync" size={12} class="animate-spin" />
          {:else}
            <MaterialIcon name="north" size={12} />
          {/if}
          {ahead}
        </button>
      {:else}
        <span
          class="inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface"
        >
          <MaterialIcon name="north" size={12} />
          {ahead}
        </span>
      {/if}
    {/if}
    {#if showBehind}
      {#if pullInteractive}
        <button
          type="button"
          class="inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface-variant transition-colors hover:bg-white/20 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary disabled:cursor-not-allowed disabled:opacity-70 {pullSyncing
            ? 'animate-pulse'
            : 'cursor-pointer'}"
          aria-label={pullLabel(behind!)}
          disabled={chipDisabled && !pullSyncing}
          onclick={handlePullClick}
        >
          {#if pullSyncing}
            <MaterialIcon name="sync" size={12} class="animate-spin" />
          {:else}
            <MaterialIcon name="south" size={12} />
          {/if}
          {behind}
        </button>
      {:else}
        <span
          class="inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface-variant"
        >
          <MaterialIcon name="south" size={12} />
          {behind}
        </span>
      {/if}
    {/if}
  </div>
{/if}
