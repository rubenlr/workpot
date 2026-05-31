<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    clientTagAddError,
    shouldSaveNotes,
    tagAlreadyOnRepo,
  } from "../orgClient";
  import type { RepoDto } from "../types";
  import TagChip from "./TagChip.svelte";

  let {
    repo,
    onClose,
    onMutated,
    requestTagFocus = false,
    onTagFocusDone,
  }: {
    repo: RepoDto;
    onClose: () => void;
    onMutated: () => void;
    requestTagFocus?: boolean;
    onTagFocusDone?: () => void;
  } = $props();

  let branches = $state<string[]>([]);
  let branchError = $state<string | null>(null);
  let notesValue = $state("");
  let tagInput = $state("");
  let tagError = $state<string | null>(null);
  let notesTextarea = $state<HTMLTextAreaElement | undefined>(undefined);
  let tagInputEl = $state<HTMLInputElement | undefined>(undefined);

  $effect(() => {
    repo.path;
    if (document.activeElement !== notesTextarea) {
      notesValue = repo.notes ?? "";
    }
  });

  $effect(() => {
    if (!requestTagFocus) {
      return;
    }
    queueMicrotask(() => {
      tagInputEl?.focus();
      onTagFocusDone?.();
    });
  });

  $effect(() => {
    const path = repo.path;
    branchError = null;
    let cancelled = false;
    void (async () => {
      try {
        const result = await invoke<string[]>("list_branches", {
          repoPath: path,
        });
        if (!cancelled) {
          branches = result;
        }
      } catch (e) {
        if (!cancelled) {
          branchError = String(e);
          branches = [];
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
    const clientErr = clientTagAddError(raw);
    if (clientErr) {
      tagError = clientErr;
      return;
    }
    const tag = raw.trim();
    if (!tag) {
      return;
    }
    if (tagAlreadyOnRepo(tag, repo.tags)) {
      tagError = "Tag already on this repo";
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
    if (!shouldSaveNotes(notesValue, repo.notes)) {
      return;
    }
    const trimmed = notesValue.trim() || null;
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

<svelte:window onkeydown={onPaneKeydown} />

<div
  role="region"
  aria-label="Repository details"
  class="flex h-full flex-col gap-4 overflow-y-auto p-4"
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
    <h2 class="min-w-0 flex-1 truncate text-lg font-semibold">
      {repo.alias ?? repo.name}
    </h2>
    <button
      type="button"
      class="shrink-0 text-lg leading-none"
      aria-label={repo.pinned ? "Unpin" : "Pin"}
      aria-pressed={repo.pinned}
      onclick={() => void handlePinChange()}
    >
      {repo.pinned ? "📌" : "📍"}
    </button>
  </div>

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
      bind:this={tagInputEl}
      bind:value={tagInput}
      placeholder="Add tag…"
      autocomplete="off"
      autocapitalize="off"
      autocorrect="off"
      spellcheck="false"
      class="mt-2 w-full rounded-md border border-neutral-200 bg-transparent px-2 py-1 text-sm dark:border-neutral-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
      onkeydown={onTagInputKeydown}
      onblur={() => {
        if (tagInput.trim()) {
          void handleAddTag(tagInput);
        }
      }}
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
      bind:this={notesTextarea}
      rows="3"
      maxlength="500"
      bind:value={notesValue}
      placeholder="Add notes..."
      autocomplete="off"
      autocapitalize="off"
      autocorrect="off"
      spellcheck="false"
      class="w-full resize-none rounded-md border border-neutral-200 bg-transparent p-2 text-sm dark:border-neutral-700 focus:outline-none focus:ring-1 focus:ring-blue-500"
      style="max-height: calc(5 * 1.5rem)"
      onblur={() => void handleNotesSave()}
    ></textarea>
  </section>
</div>
