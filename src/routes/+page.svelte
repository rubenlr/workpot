<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { RepoDto } from "$lib/types";

  let repos = $state<RepoDto[]>([]);
  let error = $state<string | null>(null);

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

  $effect(() => {
    invoke<RepoDto[]>("list_repos")
      .then((rows) => {
        repos = rows;
        error = null;
      })
      .catch((e) => {
        error = String(e);
      });
  });
</script>

<main
  class="flex h-screen max-h-[500px] flex-col bg-white text-neutral-900 dark:bg-neutral-900 dark:text-neutral-100"
>
  <header class="border-b border-neutral-200 px-3 py-2 dark:border-neutral-700">
    <h1 class="text-sm font-semibold">Workpot</h1>
  </header>

  <div class="min-h-0 flex-1 overflow-y-auto p-2">
    {#if error}
      <p class="text-sm text-red-600 dark:text-red-400">{error}</p>
    {:else if repos.length === 0}
      <p class="text-sm text-neutral-500">No repos indexed yet.</p>
    {:else}
      <ul class="space-y-1">
        {#each repos as repo}
          <li
            class="rounded-md px-2 py-1.5 hover:bg-neutral-100 dark:hover:bg-neutral-800"
          >
            <div class="flex items-center gap-2">
              <span
                class="h-2 w-2 shrink-0 rounded-full {dirtyDotClass(repo)}"
                aria-hidden="true"
              ></span>
              <span class="truncate font-medium">{repo.name}</span>
              <span class="ml-auto truncate text-xs text-neutral-500">
                {repo.branch ?? "—"}
              </span>
            </div>
            {#if repo.parent_dir}
              <div class="mt-0.5 truncate pl-4 text-xs text-neutral-500">
                {repo.parent_dir}
              </div>
            {/if}
          </li>
        {/each}
      </ul>
    {/if}
  </div>
</main>
