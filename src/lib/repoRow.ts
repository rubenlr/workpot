import type { RepoDto } from "./types";

/** Tailwind classes for per-row dirty/clean/error dot (UI-02). */
export function dirtyDotClass(repo: RepoDto): string {
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
