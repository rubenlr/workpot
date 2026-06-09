<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import RepoListRow from "$lib/components/RepoListRow.svelte";
  import SectionHeader from "$lib/components/SectionHeader.svelte";
  import { reorderPinned, toPinOrderPayload } from "$lib/pinOrder";
  import type { SectionedRepos } from "$lib/sort";
  import type { RepoDto } from "$lib/types";
  import { SECTION_META } from "./constants";

  let {
    sectionedRepos,
    flatIndexByPath,
    selectedIndex = $bindable(0),
    onPinReorder,
    onSelectRow,
    onOpen,
    onDetail,
    onTagRemove,
    onTagFilter,
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
    onTagRemove: (repoPath: string, tag: string) => void | Promise<void>;
    onTagFilter: (tag: string) => void;
  } = $props();

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
      document
        .querySelector(`[data-row-index="${idx}"]`)
        ?.scrollIntoView({ block: "nearest" });
    });
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

<ul class="space-y-0.5" role="listbox">
  {#each SECTION_META as { key, label, draggable } (key)}
    {#if sectionedRepos[key].length > 0}
      <li role="presentation">
        <SectionHeader {label} />
      </li>
      {#each sectionedRepos[key] as repo, i (repo.path)}
        {@const idx = rowFlatIndex(repo.path)}
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
            onRowDragOver={draggable ? (e) => e.preventDefault() : undefined}
            onRowDrop={draggable ? (e) => handleDrop(e, i) : undefined}
            onRowDragEnd={draggable ? clearDragSource : undefined}
            onOpen={() => {
              onSelectRow(idx);
              onOpen(idx);
            }}
            onDetail={() => {
              onSelectRow(idx);
              onDetail(repo, idx);
            }}
            onTagRemove={(tag) => onTagRemove(repo.path, tag)}
            {onTagFilter}
          />
        </li>
      {/each}
    {/if}
  {/each}
</ul>
