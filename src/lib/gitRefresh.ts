import type { GitRefreshSummary } from "./types";

/** User-facing error after background git refresh (D-27 / UAT #7). */
export function gitRefreshErrorMessage(
  summary: GitRefreshSummary,
): string | null {
  if (summary.errors > 0 && summary.refreshed === 0) {
    return "Git refresh failed for all repositories.";
  }
  if (summary.errors > 0 && summary.refreshed > 0) {
    return `Git refresh completed with ${summary.errors} error(s).`;
  }
  return null;
}

/** Whether `loadRepos` should clear the list error after refresh completes. */
export function shouldClearListErrorOnRefreshLoad(
  summary: GitRefreshSummary,
): boolean {
  return summary.errors === 0;
}
