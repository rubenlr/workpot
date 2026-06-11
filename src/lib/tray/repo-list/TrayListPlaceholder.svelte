<script lang="ts">
  let {
    message,
    tone = "muted",
  }: {
    message: string;
    tone?: "muted" | "error";
  } = $props();
</script>

<p
  class="px-3 py-2 text-sm {tone === 'error'
    ? 'text-red-600 dark:text-red-400'
    : 'text-neutral-500 dark:text-neutral-400'}"
>
  {message}
</p>
