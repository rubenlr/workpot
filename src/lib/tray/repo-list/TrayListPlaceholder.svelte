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
  class="px-3 py-2.5 text-center text-sm {tone === 'error'
    ? 'text-error'
    : 'text-inverse-on-surface-variant'}"
>
  {message}
</p>
