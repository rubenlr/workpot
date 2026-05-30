<script lang="ts">
  import { invoke } from "@tauri-apps/api/core";
  import type { RepoDto } from "$lib/types";

  let repos = $state<RepoDto[]>([]);
  let error = $state<string | null>(null);

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

<main class="p-3 text-sm">
  {#if error}
    <p class="text-red-600 dark:text-red-400">{error}</p>
  {:else if repos.length === 0}
    <p class="text-neutral-500">No repos indexed yet.</p>
  {:else}
    <ul class="space-y-2">
      {#each repos as repo}
        <li>
          <div class="font-medium">{repo.name}</div>
          <div class="text-neutral-500 text-xs">
            {repo.branch ?? "—"} · {repo.parent_dir}
          </div>
        </li>
      {/each}
    </ul>
  {/if}
</main>
