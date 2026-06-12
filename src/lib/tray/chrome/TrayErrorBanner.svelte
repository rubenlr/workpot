<script lang="ts">
  let {
    message,
    onDismiss,
  }: {
    message: string;
    onDismiss?: () => void;
  } = $props();
</script>

<div
  class="flex items-start gap-2 bg-error px-3 py-2 text-sm text-error-foreground"
  role="alert"
>
  <span class="min-w-0 flex-1 break-words">{message}</span>
  {#if onDismiss}
    <button
      type="button"
      class="shrink-0 cursor-pointer rounded border-0 bg-transparent px-1.5 py-0.5 text-xs font-semibold uppercase tracking-wide text-error-foreground/90 shadow-none outline-none hover:bg-white/10"
      onclick={onDismiss}
    >
      DISMISS
    </button>
  {/if}
</div>
