<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import { onMount } from "svelte";
  import { SvelteMap } from "svelte/reactivity";
  import RepoListRow from "./RepoListRow.svelte";
  import SectionHeader from "./SectionHeader.svelte";
  import {
    reorderPinned,
    toPinOrderPayload,
  } from "$lib/tray/logic/list/pinOrder";
  import type { SectionedRepos } from "$lib/tray/logic/list/sort";
  import type {
    ActiveConvert,
    ActiveSync,
    RepoDto,
    SyncDirection,
  } from "$lib/types";
  import { SECTION_META } from "$lib/tray/logic/handlers/constants";

  let {
    sectionedRepos,
    flatIndexByPath,
    selectedIndex = $bindable(0),
    onPinReorder,
    onSelectRow,
    onOpen,
    onDetail,
    activeSync = null,
    onSync,
    activeConvert = null,
  }: {
    sectionedRepos: SectionedRepos;
    flatIndexByPath: Map<string, number>;
    selectedIndex?: number;
    onPinReorder: (
      items: ReturnType<typeof toPinOrderPayload>,
    ) => void | Promise<void>;
    onSelectRow: (index: number) => void;
    onOpen: (index: number) => void;
    onDetail: (repo: RepoDto, index: number) => void;
    activeSync?: ActiveSync | null;
    onSync?: (
      repoPath: string,
      branch: string,
      direction: SyncDirection,
    ) => void;
    activeConvert?: ActiveConvert | null;
  } = $props();

  let hoveredRowIndex = $state<number | null>(null);

  let dragSourceIdx = $state<number | null>(null);

  let repoByPath = $derived(
    new SvelteMap(
      SECTION_META.flatMap(({ key }) =>
        sectionedRepos[key].map((repo) => [repo.path, repo] as const),
      ),
    ),
  );

  function rowFlatIndex(path: string): number {
    const idx = flatIndexByPath.get(path);
    if (idx === undefined) {
      throw new Error(`TrayRepoList: missing flat index for ${path}`);
    }
    return idx;
  }

  function showRepoContextMenu(repo: RepoDto, e: MouseEvent) {
    e.preventDefault();
    const payload = {
      repoPath: repo.path,
      isPinned: repo.pinned,
      tags: repo.tags,
      convertTo: repo.convert_to,
      convertBlockReason: repo.convert_block_reason,
      clientX: e.clientX,
      clientY: e.clientY,
    };
    void invoke("show_repo_context_menu", payload);
  }

  onMount(() => {
    const handleContextMenu = (e: MouseEvent) => {
      if ((e.target as Element)?.closest("[data-sync-action]")) {
        return;
      }
      const rowEl = (e.target as Element)?.closest("[data-row-index]");
      if (!rowEl) {
        return;
      }
      const repoPath = rowEl.getAttribute("data-repo-path");
      if (!repoPath) {
        return;
      }
      const repo = repoByPath.get(repoPath);
      if (!repo) {
        return;
      }
      showRepoContextMenu(repo, e);
    };

    document.addEventListener("contextmenu", handleContextMenu, {
      capture: true,
    });
    return () => {
      document.removeEventListener("contextmenu", handleContextMenu, {
        capture: true,
      });
    };
  });

  $effect(() => {
    const idx = selectedIndex;
    queueMicrotask(() => {
      const el = document.querySelector(`[data-row-index="${idx}"]`);
      el?.scrollIntoView?.({ block: "nearest" });
    });
  });

  $effect(() => {
    const idx = selectedIndex;
    if (hoveredRowIndex !== null && hoveredRowIndex !== idx) {
      hoveredRowIndex = null;
    }
  });

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
    await onPinReorder(toPinOrderPayload(newOrder));
  }
</script>

<div
  role="group"
  aria-label="Repositories"
  class="bg-inverse-surface px-1.5 py-1"
  onmouseleave={() => {
    hoveredRowIndex = null;
  }}
>
  {#each SECTION_META as { key, label, draggable } (key)}
    {#if sectionedRepos[key].length > 0}
      <section aria-label="{label} repositories">
        <SectionHeader {label} />
        <ul class="space-y-0.5" role="list">
          {#each sectionedRepos[key] as repo, i (repo.path)}
            {@const idx = rowFlatIndex(repo.path)}
            <RepoListRow
              {repo}
              rowIndex={idx}
              listRowDraggable={draggable}
              selected={idx === selectedIndex}
              hovered={hoveredRowIndex === idx}
              onRowMouseEnter={() => {
                hoveredRowIndex = idx;
                selectedIndex = idx;
              }}
              onRowMouseLeave={() => {
                if (hoveredRowIndex === idx) {
                  hoveredRowIndex = null;
                }
              }}
              onRowDragStart={draggable
                ? (e) => handleDragStart(e, i)
                : undefined}
              onRowDragOver={draggable ? (e) => e.preventDefault() : undefined}
              onRowDrop={draggable ? (e) => handleDrop(e, i) : undefined}
              onRowDragEnd={draggable ? clearDragSource : undefined}
              onOpen={() => {
                onSelectRow(idx);
                onOpen(idx);
              }}
              {activeSync}
              {onSync}
              {activeConvert}
              onDetail={() => {
                onSelectRow(idx);
                onDetail(repo, idx);
              }}
            />
          {/each}
        </ul>
      </section>
    {/if}
  {/each}
</div>
