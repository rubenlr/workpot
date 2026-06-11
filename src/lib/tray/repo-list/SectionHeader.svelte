<script lang="ts">
  let { label }: { label: string } = $props();
</script>

<div
  class="select-none bg-black/20 px-3 py-1.5 text-[10px] font-semibold uppercase tracking-widest text-inverse-on-surface-variant"
>
  {label}
</div>
