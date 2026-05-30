<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { fuzzyMatch } from "$lib/fuzzy";
  import { traySort } from "$lib/sort";
  import type { RepoDto, TrayConfigDto } from "$lib/types";

  const ROW_HEIGHT_PX = 44;
  const FILTER_BAR_HEIGHT_PX = 52;

  let repos = $state<RepoDto[]>([]);
  let error = $state<string | null>(null);
  let filterQuery = $state("");
  let selectedIndex = $state(0);
  let maxVisibleRows = $state(15);
  let filterInput = $state<HTMLInputElement | null>(null);

  let listMaxHeightPx = $derived(
    maxVisibleRows * ROW_HEIGHT_PX + FILTER_BAR_HEIGHT_PX,
  );

  let displayRepos = $derived(
    repos.filter((r) => fuzzyMatch(filterQuery, r)).sort(traySort),
  );

  $effect(() => {
    filterQuery;
    displayRepos.length;
    selectedIndex = 0;
  });

  $effect(() => {
    const idx = selectedIndex;
    queueMicrotask(() => {
      document
        .querySelector(`[data-row-index="${idx}"]`)
        ?.scrollIntoView({ block: "nearest" });
    });
  });

  function dirtyDotClass(repo: RepoDto): string {
    if (repo.git_state_error) {
      return "bg-neutral-400";
    }
    if (repo.is_dirty === true) {
      return "bg-amber-500";
    }
    if (repo.is_dirty === false) {
      return "bg-emerald-500";
    }
    return "bg-neutral-400";
  }

  function focusFilter() {
    filterInput?.focus();
  }

  function clampSelection() {
    if (displayRepos.length === 0) {
      selectedIndex = 0;
      return;
    }
    if (selectedIndex >= displayRepos.length) {
      selectedIndex = displayRepos.length - 1;
    }
    if (selectedIndex < 0) {
      selectedIndex = 0;
    }
  }

  function moveSelection(delta: number) {
    if (displayRepos.length === 0) {
      return;
    }
    clampSelection();
    selectedIndex = (selectedIndex + delta + displayRepos.length) % displayRepos.length;
  }

  async function hidePanel() {
    await getCurrentWindow().hide();
  }

  /** Placeholder for plan 04-04 Cursor launch. */
  async function openSelected() {
    const repo = displayRepos[selectedIndex];
    if (!repo) {
      return;
    }
    console.debug("openSelected (stub)", repo.path);
  }

  function onFilterKeydown(e: KeyboardEvent) {
    if (e.key === "ArrowDown") {
      const input = e.currentTarget as HTMLInputElement;
      const atEnd =
        input.selectionStart === input.value.length &&
        input.selectionEnd === input.value.length;
      if (atEnd || filterQuery.length === 0) {
        e.preventDefault();
        moveSelection(1);
      }
    } else if (e.key === "ArrowUp" && filterQuery.length === 0) {
      e.preventDefault();
      moveSelection(-1);
    } else if (e.key === "Escape") {
      e.preventDefault();
      void hidePanel();
    } else if (e.key === "Enter") {
      e.preventDefault();
      void openSelected();
    }
  }

  function onPanelKeydown(e: KeyboardEvent) {
    if (
      e.target instanceof HTMLInputElement &&
      e.target.id === "repo-filter"
    ) {
      return;
    }
    if (e.key === "ArrowDown") {
      e.preventDefault();
      moveSelection(1);
    } else if (e.key === "ArrowUp") {
      e.preventDefault();
      moveSelection(-1);
    } else if (e.key === "Tab" && !e.shiftKey) {
      e.preventDefault();
      moveSelection(1);
    } else if (e.key === "Escape") {
      e.preventDefault();
      void hidePanel();
    } else if (e.key === "Enter") {
      e.preventDefault();
      void openSelected();
    }
  }

  async function loadRepos() {
    try {
      repos = await invoke<RepoDto[]>("list_repos");
      error = null;
    } catch (e) {
      error = String(e);
    }
  }

  onMount(() => {
    void loadRepos();

    invoke<TrayConfigDto>("get_tray_config")
      .then((cfg) => {
        maxVisibleRows = cfg.max_visible_rows;
      })
      .catch(() => {
        maxVisibleRows = 15;
      });

    const unlisten = listen("panel-opened", () => {
      void loadRepos();
      focusFilter();
    });

    focusFilter();

    return () => {
      void unlisten.then((fn) => fn());
    };
  });
</script>

<svelte:window onkeydown={onPanelKeydown} />

<main
  class="panel-shell flex h-screen flex-col overflow-hidden rounded-xl text-neutral-900 shadow-2xl dark:text-neutral-100"
  style="max-height: {listMaxHeightPx}px"
>
  <div
    class="border-b border-neutral-200/80 bg-white/80 px-3 py-2 backdrop-blur-md dark:border-neutral-700/80 dark:bg-neutral-900/80"
  >
    <input
      id="repo-filter"
      bind:this={filterInput}
      type="search"
      placeholder="Filter repos…"
      maxlength="256"
      class="w-full rounded-md border border-neutral-200 bg-white px-2 py-1.5 text-sm outline-none ring-blue-500 focus:ring-2 dark:border-neutral-600 dark:bg-neutral-800"
      bind:value={filterQuery}
      onkeydown={onFilterKeydown}
    />
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto p-2">
    {#if error}
      <p class="text-sm text-red-600 dark:text-red-400">{error}</p>
    {:else if repos.length === 0}
      <p class="text-sm text-neutral-500">No repos indexed yet.</p>
    {:else if filterQuery.trim().length > 0 && displayRepos.length === 0}
      <p class="text-sm text-neutral-500">No repos match</p>
    {:else}
      <ul class="space-y-0.5" role="listbox">
        {#each displayRepos as repo, i (repo.path)}
          <li
            data-row-index={i}
            role="option"
            aria-selected={i === selectedIndex}
            class="rounded-md px-2 py-1.5 {i === selectedIndex
              ? 'bg-blue-600 text-white dark:bg-blue-500'
              : 'hover:bg-neutral-100 dark:hover:bg-neutral-800'}"
          >
            <div class="flex items-center gap-2">
              <span
                class="h-2 w-2 shrink-0 rounded-full {dirtyDotClass(repo)}"
                aria-hidden="true"
              ></span>
              <span class="truncate font-medium">{repo.name}</span>
              <span
                class="ml-auto truncate text-xs {i === selectedIndex
                  ? 'text-blue-100'
                  : 'text-neutral-500'}"
              >
                {repo.branch ?? "—"}
              </span>
            </div>
            {#if repo.parent_dir}
              <div
                class="mt-0.5 truncate pl-4 text-xs {i === selectedIndex
                  ? 'text-blue-100/90'
                  : 'text-neutral-500'}"
              >
                {repo.parent_dir}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</main>
