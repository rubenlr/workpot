<script lang="ts">
  import type { Snippet } from "svelte";

  let { children }: { children: Snippet } = $props();
</script>

<h3
  class="pb-1 text-xs font-semibold uppercase tracking-widest text-inverse-on-surface-variant"
>
  # {@render children()}
</h3>
