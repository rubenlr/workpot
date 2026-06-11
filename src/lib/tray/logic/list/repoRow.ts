import type { RepoDto } from "$lib/types";

/** Tailwind classes for per-row dirty/clean/error dot (Stitch Status Indicators). */
export function dirtyDotClass(repo: RepoDto): string {
  if (repo.git_state_error) {
    return "bg-git-error-neutral shadow-[var(--shadow-dot-error)]";
  }
  if (repo.is_dirty === true) {
    return "bg-dirty-amber shadow-[var(--shadow-dot-dirty)]";
  }
  if (repo.is_dirty === false) {
    return "bg-clean-emerald shadow-[var(--shadow-dot-clean)]";
  }
  return "bg-git-error-neutral shadow-[var(--shadow-dot-error)]";
}
