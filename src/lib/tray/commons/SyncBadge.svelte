<script lang="ts">
  import MaterialIcon from "./MaterialIcon.svelte";

  let {
    ahead = null,
    behind = null,
  }: {
    ahead?: number | null;
    behind?: number | null;
  } = $props();

  const showAhead = $derived(ahead != null && ahead > 0);
  const showBehind = $derived(behind != null && behind > 0);
</script>

{#if showAhead || showBehind}
  <div class="flex shrink-0 items-center gap-1" aria-hidden="true">
    {#if showAhead}
      <span
        class="inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface"
      >
        <MaterialIcon name="north" size={12} />
        {ahead}
      </span>
    {/if}
    {#if showBehind}
      <span
        class="inline-flex items-center gap-0.5 rounded-full bg-white/10 px-1.5 py-0.5 text-[10px] font-medium tabular-nums text-inverse-on-surface-variant"
      >
        <MaterialIcon name="south" size={12} />
        {behind}
      </span>
    {/if}
  </div>
{/if}
