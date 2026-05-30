<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import { shouldNavigateListOnFilterArrow } from "$lib/filterNavigation";
  import {
    gitRefreshErrorMessage,
    shouldClearListErrorOnRefreshLoad,
  } from "$lib/gitRefresh";
  import { trayListView } from "$lib/listState";
  import { selectionIndexAfterBackgroundOpen } from "$lib/openSelection";
  import { trayListMaxHeightPx } from "$lib/panelLayout";
  import { moveSelectionIndex } from "$lib/selection";
  import { dirtyDotClass } from "$lib/repoRow";
  import { filterAndSortRepos } from "$lib/trayList";
  import type { GitRefreshSummary, RepoDto, TrayConfigDto } from "$lib/types";

  let repos = $state<RepoDto[]>([]);
  let error = $state<string | null>(null);
  let filterQuery = $state("");
  let selectedIndex = $state(0);
  let maxVisibleRows = $state(15);
  let filterInput = $state<HTMLInputElement | null>(null);
  let launchError = $state<string | null>(null);
  let refreshing = $state(false);

  let listMaxHeightPx = $derived(trayListMaxHeightPx(maxVisibleRows));

  let displayRepos = $derived(filterAndSortRepos(repos, filterQuery));
  let listView = $derived(
    trayListView(error, repos.length, filterQuery, displayRepos.length),
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

  function focusFilter() {
    filterInput?.focus();
  }

  function moveSelection(delta: number) {
    selectedIndex = moveSelectionIndex(
      selectedIndex,
      delta,
      displayRepos.length,
    );
  }

  async function hidePanel() {
    await getCurrentWindow().hide();
  }

  async function openSelected(background: boolean) {
    const repo = displayRepos[selectedIndex];
    if (!repo) {
      return;
    }
    launchError = null;
    try {
      await invoke("open_in_cursor", { path: repo.path, background });
      if (!background) {
        await hidePanel();
      } else {
        const openedPath = repo.path;
        await loadRepos(false);
        selectedIndex = selectionIndexAfterBackgroundOpen(
          repos,
          filterQuery,
          openedPath,
        );
      }
    } catch (e) {
      launchError = String(e);
    }
  }

  function onFilterKeydown(e: KeyboardEvent) {
    if (e.metaKey && (e.key === "r" || e.key === "R")) {
      e.preventDefault();
      void startBackgroundRefresh();
      return;
    }
    if (e.key === "ArrowDown" || e.key === "ArrowUp") {
      const input = e.currentTarget as HTMLInputElement;
      const start = input.selectionStart ?? 0;
      const end = input.selectionEnd ?? 0;
      if (
        shouldNavigateListOnFilterArrow(
          e.key,
          filterQuery,
          start,
          end,
          input.value.length,
        )
      ) {
        e.preventDefault();
        moveSelection(e.key === "ArrowDown" ? 1 : -1);
      }
    } else if (e.key === "Escape") {
      e.preventDefault();
      void hidePanel();
    } else if (e.key === "Enter") {
      e.preventDefault();
      void openSelected(e.metaKey);
    }
  }

  function onPanelKeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement && e.target.id === "repo-filter") {
      return;
    }
    if (e.metaKey && (e.key === "r" || e.key === "R")) {
      e.preventDefault();
      void startBackgroundRefresh();
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
      void openSelected(e.metaKey);
    }
  }

  async function loadRepos(clearError = true) {
    try {
      repos = await invoke<RepoDto[]>("list_repos");
      if (clearError) {
        error = null;
      }
    } catch (e) {
      error = String(e);
    }
  }

  async function startBackgroundRefresh() {
    refreshing = true;
    try {
      await invoke("refresh_all_git_state");
    } catch (e) {
      refreshing = false;
      error = String(e);
    }
  }

  onMount(() => {
    void loadRepos();

    invoke<TrayConfigDto>("get_tray_config")
      .then((cfg) => {
        maxVisibleRows = cfg.max_visible_rows;
      })
      .catch((e) => {
        console.warn("get_tray_config failed", e);
        maxVisibleRows = 15;
      });

    const unlistenPanel = listen("panel-opened", () => {
      void loadRepos();
      refreshing = true;
      focusFilter();
    });

    const unlistenRefresh = listen<GitRefreshSummary>(
      "git-refresh-complete",
      (event) => {
        refreshing = false;
        selectedIndex = 0;
        const summary = event.payload;
        void loadRepos(shouldClearListErrorOnRefreshLoad(summary)).then(() => {
          error = gitRefreshErrorMessage(summary);
        });
      },
    );

    const unlistenRefreshFailed = listen<string>(
      "git-refresh-failed",
      (event) => {
        refreshing = false;
        error = event.payload;
      },
    );

    focusFilter();

    return () => {
      void unlistenPanel.then((fn) => fn());
      void unlistenRefresh.then((fn) => fn());
      void unlistenRefreshFailed.then((fn) => fn());
    };
  });
</script>

<svelte:window onkeydown={onPanelKeydown} />

<main
  class="panel-shell flex h-screen flex-col overflow-hidden rounded-xl text-neutral-900 shadow-2xl dark:text-neutral-100"
  style="max-height: {listMaxHeightPx}px"
>
  {#if launchError}
    <div
      class="flex items-start gap-2 border-b border-red-200 bg-red-50 px-3 py-2 text-sm text-red-800 dark:border-red-900 dark:bg-red-950/80 dark:text-red-200"
      role="alert"
    >
      <span class="min-w-0 flex-1 break-words">{launchError}</span>
      <button
        type="button"
        class="shrink-0 rounded px-1.5 py-0.5 text-xs font-medium hover:bg-red-100 dark:hover:bg-red-900"
        onclick={() => {
          launchError = null;
        }}
      >
        Dismiss
      </button>
    </div>
  {/if}
  <div
    class="border-b border-neutral-500/20 px-3 py-2 dark:border-neutral-400/15"
  >
    <div class="flex items-center gap-2">
      <input
        id="repo-filter"
        bind:this={filterInput}
        type="search"
        placeholder="Filter repos…"
        maxlength="256"
        class="min-w-0 flex-1 rounded-md border border-neutral-500/25 bg-white/40 px-2 py-1.5 text-sm outline-none ring-blue-500 backdrop-blur-sm focus:ring-2 dark:border-neutral-400/20 dark:bg-black/25"
        bind:value={filterQuery}
        onkeydown={onFilterKeydown}
      />
      {#if refreshing}
        <span
          class="h-4 w-4 shrink-0 animate-spin rounded-full border-2 border-neutral-300 border-t-blue-600 dark:border-neutral-600 dark:border-t-blue-400"
          role="status"
          aria-label="Refreshing git state"
        ></span>
      {/if}
    </div>
  </div>

  <div class="min-h-0 flex-1 overflow-y-auto p-2">
    {#if listView.kind === "error"}
      <p class="text-sm text-red-600 dark:text-red-400">{listView.message}</p>
    {:else if listView.kind === "empty-index"}
      <p class="text-sm text-neutral-500">No repos indexed yet.</p>
    {:else if listView.kind === "no-match"}
      <p class="text-sm text-neutral-500">No repos match</p>
    {:else}
      <ul class="space-y-0.5" role="listbox">
        {#each displayRepos as repo, i (repo.path)}
          <li role="presentation">
            <button
              type="button"
              data-row-index={i}
              role="option"
              aria-selected={i === selectedIndex}
              class="w-full cursor-pointer rounded-md px-2 py-1.5 text-left {i ===
              selectedIndex
                ? 'bg-blue-600 text-white dark:bg-blue-500'
                : 'hover:bg-black/5 dark:hover:bg-white/10'}"
              onclick={(e) => {
                selectedIndex = i;
                if (e.metaKey) {
                  void openSelected(true);
                }
              }}
              ondblclick={() => {
                selectedIndex = i;
                void openSelected(false);
              }}
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
            </button>
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</main>
