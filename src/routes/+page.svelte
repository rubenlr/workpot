<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import DetailPane from "$lib/components/DetailPane.svelte";
  import RepoListRow from "$lib/components/RepoListRow.svelte";
  import SectionHeader from "$lib/components/SectionHeader.svelte";
  import TagAutocomplete from "$lib/components/TagAutocomplete.svelte";
  import { shouldNavigateListOnFilterArrow } from "$lib/filterNavigation";
  import {
    gitRefreshErrorMessage,
    shouldClearListErrorOnRefreshLoad,
  } from "$lib/gitRefresh";
  import { trayListView } from "$lib/listState";
  import { resyncDetailIfOpen, resyncDetailRepo } from "$lib/detailRepoSync";
  import { DEFAULT_SECTION_CFG } from "$lib/openSelection";
  import { applyTrayNavigationKey } from "$lib/trayKeyboard";
  import { trayListMaxHeightPx } from "$lib/panelLayout";
  import { reorderPinned, toPinOrderPayload } from "$lib/pinOrder";
  import { clampSelectionIndex, moveSelectionIndex } from "$lib/selection";
  import type { SectionConfig } from "$lib/sort";
  import {
    appendTagToFilterQuery,
    replaceTrailingTagAutocomplete,
    trailingTagAutocompletePrefix,
  } from "$lib/tagFilter";
  import { filterAndSectionRepos, flatSectioned } from "$lib/trayList";
  import type { GitRefreshSummary, RepoDto, TrayConfigDto } from "$lib/types";

  const SECTION_META = [
    { key: "pinned" as const, label: "Pinned", draggable: true },
    { key: "dirty" as const, label: "Dirty", draggable: false },
    { key: "recent" as const, label: "Recent", draggable: false },
    { key: "rest" as const, label: "Rest", draggable: false },
  ] as const;

  let repos = $state<RepoDto[]>([]);
  let error = $state<string | null>(null);
  let filterQuery = $state("");
  let selectedIndex = $state(0);
  const DEFAULT_MAX_VISIBLE_ROWS = 15;
  let filterInput = $state<HTMLInputElement | null>(null);
  let launchError = $state<string | null>(null);
  let refreshing = $state(false);
  let detailRepo = $state<RepoDto | null>(null);
  let allTags = $state<string[]>([]);
  let dragSourceIdx = $state<number | null>(null);
  let focusTagOnDetailOpen = $state(false);
  let trayConfig = $state<TrayConfigDto | null>(null);

  let maxVisibleRows = $derived(
    trayConfig?.max_visible_rows ?? DEFAULT_MAX_VISIBLE_ROWS,
  );
  let listMaxHeightPx = $derived(trayListMaxHeightPx(maxVisibleRows));

  let sectionCfg = $derived<SectionConfig>({
    maxRecentDays:
      trayConfig?.max_recent_days ?? DEFAULT_SECTION_CFG.maxRecentDays,
    minRecentCount:
      trayConfig?.min_recent_count ?? DEFAULT_SECTION_CFG.minRecentCount,
  });

  let sectionedRepos = $derived(
    filterAndSectionRepos(repos, filterQuery, sectionCfg),
  );
  let flatVisible = $derived(flatSectioned(sectionedRepos));
  let flatIndexByPath = $derived(
    new Map(flatVisible.map((r, i) => [r.path, i] as const)),
  );
  let tagAutocompletePrefix = $derived(
    trailingTagAutocompletePrefix(filterQuery),
  );

  let listView = $derived(
    trayListView(error, repos.length, filterQuery, flatVisible.length),
  );

  $effect(() => {
    filterQuery;
    selectedIndex = 0;
  });

  $effect(() => {
    selectedIndex = clampSelectionIndex(selectedIndex, flatVisible.length);
  });

  $effect(() => {
    if (detailRepo !== null) {
      return;
    }
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
      flatVisible.length,
    );
  }

  async function hidePanel() {
    await getCurrentWindow().hide();
  }

  async function openSelected(background: boolean) {
    const repo = flatVisible[selectedIndex];
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
        await refreshReposAndDetail(false);
        selectedIndex = flatIndexByPath.get(openedPath) ?? 0;
      }
    } catch (e) {
      launchError = String(e);
    }
  }

  function applyTrayNav(e: KeyboardEvent, mode: "filter" | "panel") {
    return applyTrayNavigationKey(
      e,
      {
        detailRepo,
        getSelectedRepo: () => flatVisible[selectedIndex],
      },
      {
        onRefresh: () => void startBackgroundRefresh(),
        onCloseDetail: () => {
          detailRepo = null;
        },
        onHidePanel: () => void hidePanel(),
        onOpenDetailForSelection: () => {
          const repo = flatVisible[selectedIndex];
          if (repo) {
            detailRepo = repo;
          }
        },
        onMoveSelection: moveSelection,
        onOpenSelected: (background: boolean) => void openSelected(background),
      },
      mode,
    );
  }

  function setListError(e: unknown) {
    error = String(e);
  }

  function openDetailWithTagFocus(repo: RepoDto) {
    detailRepo = repo;
    focusTagOnDetailOpen = true;
  }

  async function removeTagFromRepo(repoPath: string, tag: string) {
    try {
      await invoke("remove_tag", { repoPath, tag });
      await refreshReposAndDetail();
    } catch (e) {
      setListError(e);
    }
  }

  function onFilterKeydown(e: KeyboardEvent) {
    if (applyTrayNav(e, "filter")) {
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
    }
  }

  function onPanelKeydown(e: KeyboardEvent) {
    if (e.target instanceof HTMLInputElement && e.target.id === "repo-filter") {
      return;
    }
    if (
      detailRepo !== null &&
      (e.target instanceof HTMLInputElement ||
        e.target instanceof HTMLTextAreaElement)
    ) {
      return;
    }
    applyTrayNav(e, "panel");
  }

  async function loadRepos(clearError = true) {
    try {
      repos = await invoke<RepoDto[]>("list_repos");
      if (clearError) {
        error = null;
      }
    } catch (e) {
      setListError(e);
    }
  }

  async function refreshReposAndDetail(clearError = true) {
    await loadRepos(clearError);
    await loadAllTags();
    detailRepo = resyncDetailIfOpen(repos, detailRepo);
  }

  async function loadAllTags() {
    try {
      allTags = await invoke<string[]>("list_all_tags");
    } catch (e) {
      console.warn("list_all_tags failed", e);
      allTags = [];
    }
  }

  async function startBackgroundRefresh() {
    refreshing = true;
    try {
      await invoke("refresh_all_git_state");
    } catch (e) {
      refreshing = false;
      setListError(e);
    }
  }

  function handleDragStart(e: DragEvent, idx: number) {
    if (!e.dataTransfer) {
      return;
    }
    dragSourceIdx = idx;
    e.dataTransfer.effectAllowed = "move";
  }

  function clearDragSource() {
    dragSourceIdx = null;
  }

  async function handleDrop(e: DragEvent, targetIdx: number) {
    e.preventDefault();
    if (dragSourceIdx === null || dragSourceIdx === targetIdx) {
      clearDragSource();
      return;
    }
    const newOrder = reorderPinned(
      sectionedRepos.pinned,
      dragSourceIdx,
      targetIdx,
    );
    clearDragSource();
    try {
      await invoke("set_pin_order", { items: toPinOrderPayload(newOrder) });
      await refreshReposAndDetail();
    } catch (e) {
      setListError(e);
    }
  }

  function appendTagFilter(tag: string) {
    filterQuery = appendTagToFilterQuery(filterQuery, tag);
  }

  function onTagAutocompleteSelect(tag: string) {
    filterQuery = replaceTrailingTagAutocomplete(filterQuery, tag);
  }

  onMount(() => {
    void loadRepos();
    void loadAllTags();

    invoke<TrayConfigDto>("get_tray_config")
      .then((cfg) => {
        trayConfig = cfg;
      })
      .catch((e) => {
        console.warn("get_tray_config failed", e);
      });

    const unlistenPanel = listen("panel-opened", () => {
      void refreshReposAndDetail();
      refreshing = true;
      focusFilter();
    });

    const unlistenRefresh = listen<GitRefreshSummary>(
      "git-refresh-complete",
      (event) => {
        refreshing = false;
        selectedIndex = 0;
        const summary = event.payload;
        void refreshReposAndDetail(
          shouldClearListErrorOnRefreshLoad(summary),
        ).then(() => {
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

    const unlistenContextAction = listen<{
      action: string;
      repo_path: string;
    }>("repo-context-action", async (event) => {
      const { action, repo_path } = event.payload;
      const repo = resyncDetailRepo(repos, repo_path);
      if (action === "pin") {
        if (repo) {
          await invoke("set_pin", {
            repoPath: repo_path,
            pinned: !repo.pinned,
          });
          await refreshReposAndDetail();
        }
      } else if (action === "remove_tag") {
        if (!repo) {
          return;
        }
        if (repo.tags.length === 1) {
          await removeTagFromRepo(repo_path, repo.tags[0]);
        } else {
          openDetailWithTagFocus(repo);
        }
      } else if (action === "add_tag" && repo) {
        openDetailWithTagFocus(repo);
      }
    });

    focusFilter();

    return () => {
      void unlistenPanel.then((fn) => fn());
      void unlistenRefresh.then((fn) => fn());
      void unlistenRefreshFailed.then((fn) => fn());
      void unlistenContextAction.then((fn) => fn());
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
    <div class="relative flex items-center gap-2">
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
      <TagAutocomplete
        {allTags}
        visible={filterQuery.includes("#")}
        prefix={tagAutocompletePrefix}
        onSelect={onTagAutocompleteSelect}
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

  <div class="min-h-0 flex-1 overflow-y-auto">
    {#if detailRepo}
      <DetailPane
        repo={detailRepo}
        {allTags}
        requestTagFocus={focusTagOnDetailOpen}
        onTagFocusDone={() => {
          focusTagOnDetailOpen = false;
        }}
        onClose={() => {
          detailRepo = null;
        }}
        onMutated={() => void refreshReposAndDetail()}
      />
    {:else if listView.kind === "error"}
      <p class="text-sm text-red-600 dark:text-red-400">{listView.message}</p>
    {:else if listView.kind === "empty-index"}
      <p class="text-sm text-neutral-500">No repos indexed yet.</p>
    {:else if listView.kind === "no-match"}
      <p class="text-sm text-neutral-500">No repos match</p>
    {:else}
      <ul class="space-y-0.5" role="listbox">
        {#each SECTION_META as { key, label, draggable } (key)}
          {#if sectionedRepos[key].length > 0}
            <li role="presentation">
              <SectionHeader {label} />
            </li>
            {#each sectionedRepos[key] as repo, i (repo.path)}
              {@const idx = flatIndexByPath.get(repo.path) ?? 0}
              <li role="presentation">
                <RepoListRow
                  {repo}
                  rowIndex={idx}
                  listRowDraggable={draggable}
                  selected={idx === selectedIndex}
                  onRowContextMenu={(e) => {
                    e.preventDefault();
                    void invoke("show_repo_context_menu", {
                      repoPath: repo.path,
                      isPinned: repo.pinned,
                      tags: repo.tags,
                    });
                  }}
                  onRowDragStart={draggable
                    ? (e) => handleDragStart(e, i)
                    : undefined}
                  onRowDragOver={draggable
                    ? (e) => e.preventDefault()
                    : undefined}
                  onRowDrop={draggable ? (e) => handleDrop(e, i) : undefined}
                  onRowDragEnd={draggable ? clearDragSource : undefined}
                  onOpen={() => {
                    selectedIndex = idx;
                    void openSelected(false);
                  }}
                  onDetail={() => {
                    selectedIndex = idx;
                    detailRepo = repo;
                  }}
                  onTagRemove={(tag) => removeTagFromRepo(repo.path, tag)}
                  onTagFilter={(tag) => appendTagFilter(tag)}
                />
              </li>
            {/each}
          {/if}
        {/each}
      </ul>
    {/if}
  </div>
</main>
