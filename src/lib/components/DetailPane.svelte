<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { RepoDto } from "../types";
  import TagChip from "./TagChip.svelte";

  let {
    repo,
    onClose,
    onMutated,
  }: {
    repo: RepoDto;
    onClose: () => void;
    onMutated: () => void;
  } = $props();

  let branches = $state<string[]>([]);
  let branchError = $state<string | null>(null);
  let notesValue = $state("");
  let tagInput = $state("");
  let tagError = $state<string | null>(null);
  let allTags = $state<string[]>([]);

  $effect(() => {
    const n = repo.notes ?? "";
    notesValue = n;
  });

  $effect(() => {
    const path = repo.path;
    branchError = null;
    let cancelled = false;
    void (async () => {
      try {
        const result = await invoke<string[]>("list_branches", { repoPath: path });
        if (!cancelled) {
          branches = result;
        }
      } catch (e) {
        if (!cancelled) {
          branchError = String(e);
          branches = [];
        }
      }
      try {
        const tags = await invoke<string[]>("list_all_tags");
        if (!cancelled) {
          allTags = tags;
        }
      } catch {
        if (!cancelled) {
          allTags = [];
        }
      }
    })();
    return () => {
      cancelled = true;
    };
  });

  async function handlePinChange() {
    try {
      await invoke("set_pin", { repoPath: repo.path, pinned: !repo.pinned });
      onMutated();
    } catch (e) {
      tagError = String(e);
    }
  }

  async function handleAddTag(raw: string) {
    tagError = null;
    const tag = raw.trim();
    if (!tag) {
      return;
    }
    if (tag.startsWith("#")) {
      tagError = "Tag cannot start with #";
      return;
    }
    try {
      await invoke("add_tag", { repoPath: repo.path, tag });
      tagInput = "";
      onMutated();
    } catch (e) {
      tagError = String(e);
    }
  }

  async function handleRemoveTag(tag: string) {
    tagError = null;
    try {
      await invoke("remove_tag", { repoPath: repo.path, tag });
      onMutated();
    } catch (e) {
      tagError = String(e);
    }
  }

  async function handleNotesSave() {
    const trimmed = notesValue.trim() || null;
    const previous = repo.notes?.trim() || null;
    if (trimmed === previous) {
      return;
    }
    try {
      await invoke("set_notes", { repoPath: repo.path, notes: trimmed });
      onMutated();
    } catch (e) {
      tagError = String(e);
    }
  }

  function onTagInputKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      void handleAddTag(tagInput);
    }
  }

  function onPaneKeydown(e: KeyboardEvent) {
    if (e.key !== "Escape") {
      return;
    }
    if (e.target instanceof HTMLTextAreaElement) {
      return;
    }
    e.preventDefault();
    onClose();
  }
</script>

<div
  role="region"
  aria-label="Repository details"
  class="flex h-full flex-col gap-4 overflow-y-auto p-4"
  tabindex="-1"
  onkeydown={onPaneKeydown}
>
  <div class="flex items-center gap-2">
    <button
      type="button"
      class="rounded px-2 py-1 text-sm text-neutral-600 hover:bg-neutral-100 dark:text-neutral-300 dark:hover:bg-neutral-800"
      aria-label="Close detail pane"
      onclick={onClose}
    >
      ←
    </button>
    <h2 class="truncate text-lg font-semibold">{repo.name}</h2>
  </div>

  <label class="flex cursor-pointer items-center gap-2 text-sm">
    <input
      type="checkbox"
      checked={repo.pinned}
      onchange={() => void handlePinChange()}
    />
    Pinned
  </label>

  <section>
    <h3 class="mb-1 text-sm font-medium text-neutral-600 dark:text-neutral-400">
      Branches
    </h3>
    {#if branchError}
      <p class="text-sm text-red-600 dark:text-red-400">{branchError}</p>
    {:else if branches.length === 0}
      <p class="text-sm text-neutral-500">No branches</p>
    {:else}
      <ul class="flex flex-col gap-0.5 text-sm">
        {#each branches as b (b)}
          <li>
            <span
              class={b === repo.branch
                ? "font-semibold text-blue-600 dark:text-blue-400"
                : "text-neutral-700 dark:text-neutral-300"}
            >
              {b}
            </span>
          </li>
        {/each}
      </ul>
    {/if}
  </section>

  <section>
    <h3 class="mb-1 text-sm font-medium text-neutral-600 dark:text-neutral-400">
      Tags
    </h3>
    <div class="flex flex-wrap gap-1">
      {#each repo.tags as tag (tag)}
        <TagChip {tag} onRemove={() => void handleRemoveTag(tag)} />
      {/each}
    </div>
    <input
      type="text"
      bind:value={tagInput}
      placeholder="Add tag…"
      class="mt-2 w-full rounded-md border border-neutral-200 bg-transparent px-2 py-1 text-sm dark:border-neutral-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
      onkeydown={onTagInputKeydown}
    />
    {#if tagError}
      <p class="mt-1 text-sm text-red-600 dark:text-red-400">{tagError}</p>
    {/if}
  </section>

  <section>
    <h3 class="mb-1 text-sm font-medium text-neutral-600 dark:text-neutral-400">
      Notes
    </h3>
    <textarea
      rows="3"
      maxlength="500"
      bind:value={notesValue}
      placeholder="Add notes..."
      class="w-full resize-none rounded-md border border-neutral-200 bg-transparent p-2 text-sm dark:border-neutral-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
      style="max-height: calc(5 * 1.5rem)"
      onblur={() => void handleNotesSave()}
    ></textarea>
  </section>
</div>
