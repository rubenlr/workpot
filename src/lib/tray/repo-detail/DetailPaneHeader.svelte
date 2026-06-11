<script lang="ts">
  import MaterialIcon from "$lib/tray/commons/MaterialIcon.svelte";
  import type { RepoDto } from "$lib/types";

  let {
    repo,
    onClose,
    onPinToggle,
  }: {
    repo: RepoDto;
    onClose: () => void;
    onPinToggle: () => void;
  } = $props();

  const title = $derived(repo.alias ?? repo.name);
</script>

<header
  class="sticky top-0 z-10 flex items-center gap-2 border-b border-card-border bg-inverse-surface/95 px-3 py-2 backdrop-blur-sm"
>
  <button
    type="button"
    class="flex shrink-0 cursor-pointer items-center justify-center rounded-md border-0 bg-transparent p-1 text-inverse-on-surface-variant shadow-none outline-none hover:bg-white/10 focus-visible:ring-1 focus-visible:ring-primary"
    aria-label="Close detail pane"
    onclick={onClose}
  >
    <MaterialIcon name="arrow_back" size={22} />
  </button>
  <h2
    class="min-w-0 flex-1 truncate text-base font-semibold text-inverse-on-surface"
  >
    {title}
  </h2>
  <button
    type="button"
    class="flex shrink-0 cursor-pointer items-center justify-center rounded-md border-0 bg-transparent p-1 shadow-none outline-none hover:bg-white/10 focus-visible:ring-1 focus-visible:ring-primary {repo.pinned
      ? 'text-primary'
      : 'text-inverse-on-surface-variant'}"
    aria-label={repo.pinned ? "Unpin" : "Pin"}
    aria-pressed={repo.pinned}
    onclick={onPinToggle}
  >
    <MaterialIcon name="keep" size={22} filled={repo.pinned} />
  </button>
</header>
