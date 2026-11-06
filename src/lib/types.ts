export interface TrayConfigDto {
  max_visible_rows: number;
  max_recent_days: number;
  min_recent_count: number;
  max_pinned: number;
  stale_dirty_days: number;
}

export interface SyncSummary {
  added: number;
  removed: number;
  skipped: number;
  git_refreshed: number;
  git_errors: number;
  remote_indexed?: number;
  remote_added?: number;
  remote_cloned?: number;
  remote_pruned?: number;
  /** Soft per-source errors from the remote catalog pass. */
  remote_errors?: string[];
}

export interface GitRefreshSummary {
  refreshed: number;
  errors: number;
  any_dirty: boolean;
}

export type SyncDirection = "push" | "pull";

export interface RepoSyncEvent {
  repo_path: string;
  branch: string;
  direction: SyncDirection;
  error?: string;
}

export interface ActiveSync {
  repoPath: string;
  branch: string;
  direction: SyncDirection;
}

export interface ActiveConvert {
  repoPath: string;
}

export interface RepoCloneEvent {
  repo_path: string;
  new_path?: string;
  opened?: boolean;
  error?: string;
}

export type ClonePolicy = "manual" | "on_open" | "auto";

export interface RemoteSourceDto {
  name: string;
  provider: "github" | "gitlab";
  host: string;
  path: string;
  clone_root: string;
  clone_policy: ClonePolicy;
  prune_removed: boolean;
  repo_count: number;
  cloned_count: number;
  removed_upstream_count: number;
  last_sync_status: "ok" | "error" | null;
  last_sync_at: number | null;
  last_sync_message: string | null;
}

export interface RepoConvertEvent {
  repo_path: string;
  new_path?: string;
  error?: string;
}

export type BranchTracking = "local_only" | "remote_only" | "local_remote";

export interface BranchListItemDto {
  name: string;
  checked_out: boolean;
  tracking: BranchTracking;
  ahead: number | null;
  behind: number | null;
  hidden: boolean;
}

export interface RepoDto {
  path: string;
  name: string;
  alias: string | null;
  branch: string | null;
  ahead: number | null;
  behind: number | null;
  is_dirty: boolean | null;
  parent_dir: string;
  last_opened_at: number | null;
  git_state_error: string | null;
  pinned: boolean;
  pin_order: number | null;
  notes: string | null;
  tags: string[];
  branches: string[];
  is_bare: boolean;
  convert_to: "bare" | "local" | null;
  convert_block_reason: string | null;
  /**
   * `false` for an indexed remote repo with no local clone (`path` is then the planned clone
   * path). Absent/`true` for ordinary catalog repos.
   */
  cloned?: boolean;
  remote_url?: string | null;
  /** Full path on the host, e.g. `acme/platform/api`. */
  remote_full_path?: string | null;
  remote_source?: string | null;
  clone_policy?: ClonePolicy | null;
  /** Local clone whose upstream repo no longer exists. */
  removed_upstream?: boolean;
}

/** True unless the repo is an indexed remote entry that has not been cloned yet. */
export function isCloned(repo: RepoDto): boolean {
  return repo.cloned !== false;
}
