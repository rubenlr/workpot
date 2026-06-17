<script lang="ts">
  import MaterialIcon from "$lib/tray/commons/MaterialIcon.svelte";
  import SyncBadge from "$lib/tray/commons/SyncBadge.svelte";
  import { branchListItemLabel, isCheckoutable } from "$lib/branchStatus";
  import type {
    ActiveSync,
    BranchListItemDto,
    SyncDirection,
  } from "$lib/types";

  let {
    branch,
    repoPath,
    activeSync = null,
    onSync,
    onActivate,
    onSyncHoverChange,
  }: {
    branch: BranchListItemDto;
    repoPath: string;
    activeSync?: ActiveSync | null;
    onSync?: (
      repoPath: string,
      branch: string,
      direction: SyncDirection,
    ) => void;
    onActivate?: (branch: BranchListItemDto) => void;
    onSyncHoverChange?: (hovered: boolean) => void;
  } = $props();

  const isRemoteOnly = $derived(branch.tracking === "remote_only");
  const isLocalOnly = $derived(branch.tracking === "local_only");
  const isClickable = $derived(
    branch.checked_out || isCheckoutable(branch.checked_out),
  );

  const syncingDirection = $derived(
    activeSync &&
      activeSync.repoPath === repoPath &&
      activeSync.branch === branch.name
      ? activeSync.direction
      : null,
  );

  const syncDisabled = $derived(activeSync != null);

  const nameButtonClass = $derived(
    [
      "flex min-w-0 flex-1 items-center gap-2 rounded-lg border-0 bg-transparent px-0 py-0 text-left shadow-none outline-none focus-visible:ring-1 focus-visible:ring-primary",
      branch.checked_out
        ? "font-medium text-inverse-on-surface"
        : "text-inverse-on-surface-variant",
      isClickable ? "cursor-pointer" : "cursor-default",
    ].join(" "),
  );
</script>

<div
  class="flex items-center gap-2 rounded-lg px-3 py-2 {branch.checked_out
    ? 'bg-primary/15 ring-1 ring-primary/30'
    : 'bg-card-surface hover:bg-hover-overlay'}"
  title={branchListItemLabel(branch)}
>
  {#if branch.checked_out}
    <MaterialIcon name="check" size={18} class="shrink-0 text-primary-accent" />
  {:else}
    <span class="w-[18px] shrink-0" aria-hidden="true"></span>
  {/if}
  <button
    type="button"
    class={nameButtonClass}
    aria-label="Activate branch {branch.name}"
    disabled={!isClickable}
    onclick={() => onActivate?.(branch)}
  >
    <span class="min-w-0 flex-1 truncate text-sm">
      {branch.name}
    </span>
  </button>
  {#if isRemoteOnly}
    <span
      class="shrink-0 rounded-full bg-hover-overlay px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-inverse-on-surface-variant"
    >
      remote
    </span>
  {:else if isLocalOnly}
    <span
      class="shrink-0 rounded-full bg-hover-overlay px-2 py-0.5 text-[10px] font-medium uppercase tracking-wide text-inverse-on-surface-variant"
    >
      local
    </span>
  {/if}
  <div class="flex shrink-0 items-center self-center">
    <SyncBadge
      ahead={branch.ahead}
      behind={branch.behind}
      branch={branch.name}
      {syncingDirection}
      disabled={syncDisabled}
      onHoverChange={onSyncHoverChange}
      onPush={onSync ? () => onSync(repoPath, branch.name, "push") : undefined}
      onPull={onSync ? () => onSync(repoPath, branch.name, "pull") : undefined}
    />
  </div>
</div>
