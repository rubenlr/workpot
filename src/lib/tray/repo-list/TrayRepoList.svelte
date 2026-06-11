<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import RepoListRow from "./RepoListRow.svelte";
  import SectionHeader from "./SectionHeader.svelte";
  import {
    reorderPinned,
    toPinOrderPayload,
  } from "$lib/tray/logic/list/pinOrder";
  import type { SectionedRepos } from "$lib/tray/logic/list/sort";
  import type { ActiveSync, RepoDto, SyncDirection } from "$lib/types";
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
  } = $props();

  let hoveredRowIndex = $state<number | null>(null);
  let syncHoveredRowIndex = $state<number | null>(null);

  let dragSourceIdx = $state<number | null>(null);

  function rowFlatIndex(path: string): number {
    const idx = flatIndexByPath.get(path);
    if (idx === undefined) {
      throw new Error(`TrayRepoList: missing flat index for ${path}`);
    }
    return idx;
  }

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
    syncHoveredRowIndex = null;
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
              syncHovered={syncHoveredRowIndex === idx}
              onRowMouseEnter={() => {
                hoveredRowIndex = idx;
                selectedIndex = idx;
              }}
              onRowMouseLeave={() => {
                if (hoveredRowIndex === idx) {
                  hoveredRowIndex = null;
                }
              }}
              onSyncHoverChange={(hovered) => {
                syncHoveredRowIndex = hovered ? idx : null;
              }}
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
              onRowDragOver={draggable ? (e) => e.preventDefault() : undefined}
              onRowDrop={draggable ? (e) => handleDrop(e, i) : undefined}
              onRowDragEnd={draggable ? clearDragSource : undefined}
              onOpen={() => {
                onSelectRow(idx);
                onOpen(idx);
              }}
              {activeSync}
              {onSync}
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
