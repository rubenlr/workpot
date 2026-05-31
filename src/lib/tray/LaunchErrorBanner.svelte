<script lang="ts">
  let { message, onDismiss }: { message: string; onDismiss: () => void } = $props();
</script>

<div
  class="flex items-start gap-2 border-b border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800 dark:border-red-900 dark:bg-red-950/80 dark:text-red-200"
  role="alert"
>
  <span class="min-w-0 flex-1 break-words">{message}</span>
  <button
    type="button"
    class="shrink-0 rounded px-1.5 py-0.5 text-xs font-medium hover:bg-red-100 dark:hover:bg-red-900"
    onclick={onDismiss}
  >
    Dismiss
  </button>
</div>
