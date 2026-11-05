<script lang="ts">
  import MaterialIcon from "$lib/tray/commons/MaterialIcon.svelte";
  import type { ClonePolicy, RepoDto } from "$lib/types";

  let {
    repo,
    cloning = false,
    onClone,
    onClose,
  }: {
    repo: RepoDto;
    cloning?: boolean;
    onClone?: (repo: RepoDto) => void;
    onClose: () => void;
  } = $props();

  const title = $derived(repo.alias ?? repo.name);

  const POLICY_HINTS: Record<ClonePolicy, string> = {
    manual: "Not cloned. Clone it here, or with `workpot clone`.",
    on_open: "Opening this repository clones it first.",
    auto: "Cloned automatically on the next sync.",
  };

  const policyHint = $derived(
    POLICY_HINTS[repo.clone_policy ?? "manual"] ?? POLICY_HINTS.manual,
  );
</script>

<div
  class="flex h-full flex-col overflow-y-auto bg-inverse-surface text-inverse-on-surface"
  data-testid="remote-detail-pane"
>
  <div
    class="sticky top-0 z-10 flex items-center gap-2 border-b border-card-border bg-inverse-surface/95 px-3 py-2 backdrop-blur-sm"
  >
    <button
      type="button"
      class="flex shrink-0 cursor-pointer items-center justify-center rounded-md border-0 bg-transparent p-1 text-inverse-on-surface-variant shadow-none outline-none hover:bg-hover-overlay focus-visible:ring-1 focus-visible:ring-primary"
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
  </div>

  <div class="flex flex-col gap-3 p-3">
    <div
      class="flex flex-col gap-1 rounded-lg bg-card-surface p-2.5 text-xs text-inverse-on-surface-variant"
    >
      {#if repo.remote_full_path}
        <span class="font-mono">{repo.remote_full_path}</span>
      {/if}
      {#if repo.remote_url}
        <span class="break-all font-mono opacity-80">{repo.remote_url}</span>
      {/if}
      <span class="break-all font-mono opacity-80"
        >Clones to {repo.parent_dir
          ? `${repo.parent_dir}/${repo.name}`
          : repo.path}</span
      >
    </div>

    <p class="text-xs text-inverse-on-surface-variant">{policyHint}</p>

    <button
      type="button"
      class="inline-flex cursor-pointer items-center justify-center gap-1.5 rounded-lg border-0 bg-primary px-3 py-2 text-sm font-medium text-primary-foreground shadow-none outline-none hover:opacity-90 focus-visible:ring-1 focus-visible:ring-primary disabled:cursor-wait disabled:opacity-70"
      disabled={cloning || !onClone}
      onclick={() => onClone?.(repo)}
    >
      <MaterialIcon
        name={cloning ? "sync" : "download"}
        size={16}
        class={cloning ? "animate-spin" : ""}
      />
      {cloning ? "Cloning…" : "Clone"}
    </button>
  </div>
</div>
