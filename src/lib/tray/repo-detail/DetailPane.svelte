<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import {
    clientTagAddError,
    shouldSaveNotes,
    tagAlreadyOnRepo,
  } from "$lib/orgClient";
  import { isCheckoutable } from "$lib/branchStatus";
  import type {
    ActiveSync,
    BranchListItemDto,
    RepoDto,
    SyncDirection,
  } from "$lib/types";
  import { getCurrentWindow } from "@tauri-apps/api/window";
  import BranchListRow from "./BranchListRow.svelte";
  import DetailPaneHeader from "./DetailPaneHeader.svelte";
  import TagAutocomplete from "$lib/tray/commons/TagAutocomplete.svelte";
  import TagChip from "$lib/tray/commons/TagChip.svelte";

  let {
    repo,
    allTags = [],
    onClose,
    onMutated,
    requestTagFocus = false,
    onTagFocusDone,
    activeSync = null,
    onSync,
    branchRevision = 0,
  }: {
    repo: RepoDto;
    allTags?: string[];
    onClose: () => void;
    onMutated: () => void;
    requestTagFocus?: boolean;
    onTagFocusDone?: () => void;
    activeSync?: ActiveSync | null;
    onSync?: (
      repoPath: string,
      branch: string,
      direction: SyncDirection,
    ) => void;
    branchRevision?: number;
  } = $props();

  let tagSuggestTags = $derived.by(() => {
    const onRepo = new Set(repo.tags);
    return allTags.filter((t) => !onRepo.has(t));
  });

  let branches = $state<BranchListItemDto[]>([]);
  let branchError = $state<string | null>(null);
  let checkoutError = $state<string | null>(null);
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
    notesTextarea?.setAttribute("autocorrect", "off");
  });

  $effect(() => {
    const path = repo.path;
    branchRevision;
    branchError = null;
    let cancelled = false;
    void (async () => {
      try {
        const result = await invoke<BranchListItemDto[]>("list_branches", {
          repoPath: path,
        });
        if (!cancelled && repo.path === path) {
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

  async function activateBranch(b: BranchListItemDto) {
    checkoutError = null;
    if (b.presence === "checkout") {
      try {
        await invoke("open_in_cursor", { path: repo.path, background: false });
        await getCurrentWindow().hide();
      } catch (e) {
        checkoutError = String(e);
      }
      return;
    }
    if (!isCheckoutable(b.presence)) {
      return;
    }
    try {
      await invoke("checkout_repo_branch", {
        repoPath: repo.path,
        branch: b.name,
      });
      onMutated();
      await invoke("open_in_cursor", { path: repo.path, background: false });
      await getCurrentWindow().hide();
    } catch (e) {
      checkoutError = String(e);
    }
  }

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
  class="flex h-full flex-col overflow-y-auto bg-inverse-surface text-inverse-on-surface"
>
  <DetailPaneHeader
    {repo}
    {onClose}
    onPinToggle={() => void handlePinChange()}
  />

  <div class="flex flex-col gap-4 p-3">
    <section>
      <h3
        class="mb-2 text-xs font-semibold uppercase tracking-widest text-inverse-on-surface-variant"
      >
        Branches
      </h3>
      <div
        class="rounded-xl border border-card-border bg-card-surface p-2 space-y-1"
      >
        {#if checkoutError}
          <p class="px-2 py-1 text-sm text-error">{checkoutError}</p>
        {/if}
        {#if branchError}
          <p class="px-2 py-1 text-sm text-error">{branchError}</p>
        {:else if branches.length === 0}
          <p class="px-2 py-1 text-sm text-inverse-on-surface-variant">
            No branches
          </p>
        {:else}
          {#each branches as b (b.name)}
            <BranchListRow
              branch={b}
              repoPath={repo.path}
              {activeSync}
              {onSync}
              onActivate={(branch) => void activateBranch(branch)}
            />
          {/each}
        {/if}
      </div>
    </section>

    <section>
      <div class="mb-2 flex flex-wrap items-center gap-2">
        <h3
          class="text-xs font-semibold uppercase tracking-widest text-inverse-on-surface-variant"
        >
          Tags
        </h3>
        {#each repo.tags as tag (tag)}
          <TagChip
            {tag}
            variant="detail"
            onRemove={() => void handleRemoveTag(tag)}
          />
        {/each}
      </div>
      <div
        class="relative rounded-xl border border-card-border bg-card-surface p-2"
      >
        <input
          type="text"
          bind:this={tagInputEl}
          bind:value={tagInput}
          placeholder="Add tag…"
          autocomplete="off"
          autocapitalize="off"
          autocorrect="off"
          spellcheck="false"
          class="w-full rounded-lg border border-card-border bg-input-surface px-3 py-2 text-sm text-inverse-on-surface outline-none placeholder:text-inverse-on-surface-variant focus:border-primary/50 focus:ring-1 focus:ring-primary"
          onkeydown={onTagInputKeydown}
          onblur={() => {
            if (tagInput.trim()) {
              void handleAddTag(tagInput);
            }
          }}
        />
        <TagAutocomplete
          allTags={tagSuggestTags}
          visible={tagInput.trim().length > 0 && tagSuggestTags.length > 0}
          prefix={tagInput.trim()}
          onSelect={(tag) => {
            void handleAddTag(tag);
          }}
        />
        {#if tagError}
          <p class="mt-1 text-sm text-error">{tagError}</p>
        {/if}
      </div>
    </section>

    <section>
      <h3
        class="mb-2 text-xs font-semibold uppercase tracking-widest text-inverse-on-surface-variant"
      >
        Notes
      </h3>
      <div class="rounded-xl border border-card-border bg-card-surface p-2">
        <textarea
          bind:this={notesTextarea}
          rows="3"
          maxlength="500"
          bind:value={notesValue}
          placeholder="Add notes..."
          autocomplete="off"
          autocapitalize="off"
          spellcheck="false"
          class="max-h-[calc(5*1.5rem)] w-full resize-none rounded-lg border border-card-border bg-input-surface p-3 text-sm text-inverse-on-surface outline-none placeholder:text-inverse-on-surface-variant focus:border-primary/50 focus:ring-1 focus:ring-primary"
          onblur={() => void handleNotesSave()}
        ></textarea>
      </div>
    </section>
  </div>
</div>
