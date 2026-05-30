export interface TrayConfigDto {
  max_visible_rows: number;
}

export interface RepoDto {
  path: string;
  name: string;
  branch: string | null;
  is_dirty: boolean | null;
  parent_dir: string;
  last_opened_at: number | null;
  git_state_error: string | null;
}
