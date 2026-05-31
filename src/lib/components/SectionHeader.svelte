<script lang="ts">
  let { label }: { label: string } = $props();
</script>

<div
  class="select-none px-3 py-1 text-xs font-semibold uppercase tracking-wide text-neutral-400 dark:text-neutral-500"
>
  {label}
</div>
