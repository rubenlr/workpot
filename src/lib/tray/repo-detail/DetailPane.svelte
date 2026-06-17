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
  import DetailSectionHeading from "./DetailSectionHeading.svelte";
  import TagAutocomplete from "$lib/tray/commons/TagAutocomplete.svelte";
  import TagChip from "$lib/tray/commons/TagChip.svelte";
  import MaterialIcon from "$lib/tray/commons/MaterialIcon.svelte";
  import { filterTagsForAutocomplete } from "$lib/tagAutocomplete";

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

  const displayPath = $derived.by(() => {
    const path = repo.path;
    const parentDir = repo.parent_dir;
    if (!parentDir) {
      return path;
    }
    const lastSlashIdx = Math.max(
      path.lastIndexOf("/"),
      path.lastIndexOf("\\"),
    );
    if (lastSlashIdx !== -1) {
      const absoluteParent = path.slice(0, lastSlashIdx);
      if (path.startsWith(absoluteParent)) {
        return parentDir + path.slice(lastSlashIdx);
      }
    }
    return path;
  });

  let branches = $state<BranchListItemDto[]>([]);
  let branchError = $state<string | null>(null);
  let checkoutError = $state<string | null>(null);
  let notesValue = $state("");
  let aliasValue = $state("");
  let tagInput = $state("");
  let tagAutocompleteVisible = $derived(
    tagInput.trim().length > 0 &&
      filterTagsForAutocomplete(tagSuggestTags, tagInput.trim(), "").length > 0,
  );
  let tagError = $state<string | null>(null);
  let aliasError = $state<string | null>(null);
  let notesTextarea = $state<HTMLTextAreaElement | undefined>(undefined);
  let tagInputEl = $state<HTMLInputElement | undefined>(undefined);
  let aliasInputEl = $state<HTMLInputElement | undefined>(undefined);

  $effect(() => {
    repo.path;
    if (document.activeElement !== notesTextarea) {
      notesValue = repo.notes ?? "";
    }
    if (document.activeElement !== aliasInputEl) {
      aliasValue = repo.alias ?? "";
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
    if (b.checked_out) {
      try {
        await invoke("open_in_cursor", { path: repo.path, background: false });
        await getCurrentWindow().hide();
      } catch (e) {
        checkoutError = String(e);
      }
      return;
    }
    if (!isCheckoutable(b.checked_out)) {
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

  async function handleAliasSave() {
    aliasError = null;
    const trimmed = aliasValue.trim() || null;
    if (trimmed === (repo.alias ?? null)) {
      return;
    }
    try {
      await invoke("set_alias", { repoPath: repo.path, alias: trimmed });
      onMutated();
    } catch (e) {
      aliasError = String(e);
    }
  }

  function onTagInputKeydown(e: KeyboardEvent) {
    if (e.key === "Enter") {
      e.preventDefault();
      void handleAddTag(tagInput);
      return;
    }
    if (e.key === "Backspace" && !tagInput && repo.tags.length > 0) {
      const lastTag = repo.tags.at(-1);
      if (lastTag) {
        e.preventDefault();
        void handleRemoveTag(lastTag);
      }
    }
  }

  async function openInFinder() {
    try {
      await invoke("open_in_finder", { path: repo.path });
    } catch (e) {
      console.error(e);
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
    <div
      class="flex items-center justify-between gap-1.5 rounded-lg bg-card-surface p-2.5 text-xs text-inverse-on-surface-variant font-mono"
    >
      <div class="flex items-center gap-1.5 min-w-0">
        <MaterialIcon name="folder" size={14} class="shrink-0" />
        <span class="break-all select-all">{displayPath}</span>
      </div>
      <button
        type="button"
        onclick={openInFinder}
        class="shrink-0 rounded-full bg-tag-blue-bg/10 border border-tag-blue-text/30 text-tag-blue-text px-2 py-0.5 text-[10px] font-medium hover:bg-tag-blue-bg/20 active:bg-tag-blue-bg/30 transition-colors uppercase tracking-wider cursor-pointer"
        aria-label="finder"
      >
        finder
      </button>
    </div>

    <section>
      <DetailSectionHeading>Alias</DetailSectionHeading>
      <input
        type="text"
        bind:this={aliasInputEl}
        bind:value={aliasValue}
        placeholder="Display name…"
        maxlength="64"
        autocomplete="off"
        autocapitalize="off"
        autocorrect="off"
        spellcheck="false"
        class="w-full rounded-lg border border-card-border bg-input-surface px-3 py-2 text-sm text-inverse-on-surface outline-none placeholder:text-inverse-on-surface-variant focus:border-primary/50 focus:ring-1 focus:ring-primary"
        onblur={() => void handleAliasSave()}
        onkeydown={(e) => {
          if (e.key === "Enter") {
            e.preventDefault();
            aliasInputEl?.blur();
          }
        }}
      />
      {#if aliasError}
        <p class="mt-1 text-sm text-error">{aliasError}</p>
      {/if}
    </section>

    <section>
      <DetailSectionHeading>Branches</DetailSectionHeading>

      <div class="space-y-1">
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
      <DetailSectionHeading>Tags</DetailSectionHeading>
      <!-- svelte-ignore a11y_click_events_have_key_events -->
      <!-- svelte-ignore a11y_no_static_element_interactions -->
      <div
        class="flex min-h-[2.5rem] flex-wrap items-center gap-1.5 rounded-lg border border-card-border bg-input-surface px-3 py-2 focus-within:border-primary/50 focus-within:ring-1 focus-within:ring-primary"
        onclick={() => tagInputEl?.focus()}
      >
        {#each repo.tags as tag (tag)}
          <TagChip
            {tag}
            variant="detail"
            onRemove={() => void handleRemoveTag(tag)}
          />
        {/each}
        <div class="relative min-w-[5rem] flex-1">
          <input
            type="text"
            bind:this={tagInputEl}
            bind:value={tagInput}
            placeholder="Add tag…"
            autocomplete="off"
            autocapitalize="off"
            autocorrect="off"
            spellcheck="false"
            class="w-full border-0 bg-transparent p-0 text-sm text-inverse-on-surface outline-none placeholder:text-inverse-on-surface-variant focus:ring-0"
            onkeydown={onTagInputKeydown}
            onblur={() => {
              if (tagInput.trim()) {
                void handleAddTag(tagInput);
              }
            }}
          />
          <TagAutocomplete
            allTags={tagSuggestTags}
            visible={tagAutocompleteVisible}
            prefix={tagInput.trim()}
            onSelect={(tag) => {
              void handleAddTag(tag);
            }}
          />
        </div>
      </div>
      {#if tagError}
        <p class="mt-1 text-sm text-error">{tagError}</p>
      {/if}
    </section>

    <section>
      <DetailSectionHeading>Notes</DetailSectionHeading>
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
    </section>
  </div>
</div>
