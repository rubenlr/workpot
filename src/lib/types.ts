export interface TrayConfigDto {
  max_visible_rows: number;
  max_recent_days: number;
  min_recent_count: number;
  max_pinned: number;
  stale_dirty_days: number;
}

export interface IndexSummary {
  added: number;
  removed: number;
  skipped: number;
  git_refreshed: number;
  git_errors: number;
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

export type BranchPresence =
  | "checkout"
  | "local_only"
  | "remote_only"
  | "local_remote";

export interface BranchListItemDto {
  name: string;
  presence: BranchPresence;
  ahead: number | null;
  behind: number | null;
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
}
