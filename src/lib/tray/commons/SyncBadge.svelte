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

  const pushChipClass =
    "inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface transition-colors hover:bg-primary/20 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary cursor-pointer disabled:cursor-not-allowed disabled:opacity-70";

  const pullChipClass =
    "inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface-variant transition-colors hover:bg-white/20 focus-visible:outline-none focus-visible:ring-1 focus-visible:ring-primary cursor-pointer disabled:cursor-not-allowed disabled:opacity-70";

  const pushSpanClass =
    "inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface opacity-70 cursor-default";

  const pullSpanClass =
    "inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface-variant opacity-70 cursor-default";

  const pushSyncingSpanClass =
    "inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface opacity-70 cursor-default animate-pulse";

  const pullSyncingSpanClass =
    "inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface-variant opacity-70 cursor-default animate-pulse";

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
    if (!pushInteractive || chipDisabled) {
      return;
    }
    onPush?.();
  }

  function handlePullClick(e: MouseEvent) {
    e.stopPropagation();
    if (!pullInteractive || chipDisabled) {
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
      {#if pushInteractive && !pushSyncing}
        <button
          type="button"
          class={pushChipClass}
          aria-label={pushLabel(ahead!)}
          disabled={chipDisabled}
          onclick={handlePushClick}
        >
          <MaterialIcon name="north" size={12} />
          {ahead}
        </button>
      {:else if showAhead}
        <span class={pushSyncing ? pushSyncingSpanClass : pushSpanClass}>
          {#if pushSyncing}
            <MaterialIcon name="sync" size={12} class="animate-spin" />
          {:else}
            <MaterialIcon name="north" size={12} />
          {/if}
          {ahead}
        </span>
      {/if}
    {/if}
    {#if showBehind}
      {#if pullInteractive && !pullSyncing}
        <button
          type="button"
          class={pullChipClass}
          aria-label={pullLabel(behind!)}
          disabled={chipDisabled}
          onclick={handlePullClick}
        >
          <MaterialIcon name="south" size={12} />
          {behind}
        </button>
      {:else if showBehind}
        <span class={pullSyncing ? pullSyncingSpanClass : pullSpanClass}>
          {#if pullSyncing}
            <MaterialIcon name="sync" size={12} class="animate-spin" />
          {:else}
            <MaterialIcon name="south" size={12} />
          {/if}
          {behind}
        </span>
      {/if}
    {/if}
  </div>
{/if}
