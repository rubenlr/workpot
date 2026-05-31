export interface TrayConfigDto {
  max_visible_rows: number;
  max_recent_days: number;
  min_recent_count: number;
  max_pinned: number;
}

export interface GitRefreshSummary {
  refreshed: number;
  errors: number;
  any_dirty: boolean;
}

export interface RepoDto {
  path: string;
  name: string;
  branch: string | null;
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
