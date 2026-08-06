-- Fresh catalog bootstrap (P3): projects / locations replace the old repos chain.

CREATE TABLE projects (
  id TEXT NOT NULL PRIMARY KEY,
  root_remote_normalized TEXT NOT NULL UNIQUE,
  root_remote_raw TEXT,
  name TEXT,
  created_at INTEGER NOT NULL
);

CREATE TABLE project_remotes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  project_id TEXT NOT NULL REFERENCES projects(id) ON DELETE CASCADE,
  url_normalized TEXT NOT NULL,
  url_raw TEXT NOT NULL,
  role TEXT NOT NULL CHECK (role IN ('root', 'fork')),
  remote_name TEXT,
  UNIQUE (project_id, url_raw)
);

CREATE INDEX idx_project_remotes_url_normalized ON project_remotes(url_normalized);

-- Catalog checkout paths (formerly `repos`). `RepoRecord` / list_repos still map here.
-- project_id stays NULL until Wave 3 attach/merge.
CREATE TABLE locations (
  path TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  registered_at INTEGER NOT NULL,
  source TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'scan')),
  excluded INTEGER NOT NULL DEFAULT 0 CHECK (excluded IN (0, 1)),
  git_common_dir TEXT NOT NULL DEFAULT '',
  branch TEXT,
  is_dirty INTEGER,
  ahead INTEGER,
  behind INTEGER,
  git_refreshed_at INTEGER,
  git_state_error TEXT,
  last_opened_at INTEGER,
  notes TEXT,
  pinned INTEGER NOT NULL DEFAULT 0 CHECK (pinned IN (0, 1)),
  pin_order INTEGER,
  alias TEXT,
  convert_block_reason TEXT,
  project_id TEXT REFERENCES projects(id)
);

CREATE INDEX idx_locations_registered_at ON locations(registered_at);
CREATE INDEX idx_locations_git_common_dir ON locations(git_common_dir);
CREATE INDEX idx_locations_source_excluded ON locations(source, excluded);
CREATE INDEX idx_locations_project_id ON locations(project_id);

CREATE TABLE worktrees (
  path TEXT NOT NULL PRIMARY KEY,
  location_path TEXT NOT NULL REFERENCES locations(path) ON DELETE CASCADE,
  head_branch TEXT
);

CREATE INDEX idx_worktrees_location ON worktrees(location_path);

-- remote_name DEFAULT '' so UNIQUE treats NULL locals distinctly from remotes.
CREATE TABLE branches (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  location_path TEXT NOT NULL REFERENCES locations(path) ON DELETE CASCADE,
  name TEXT NOT NULL,
  kind TEXT NOT NULL CHECK (kind IN ('local', 'remote')),
  tip_oid TEXT,
  remote_name TEXT NOT NULL DEFAULT '',
  UNIQUE (location_path, kind, name, remote_name)
);

CREATE INDEX idx_branches_location ON branches(location_path);

CREATE TABLE repo_tags (
  repo_path TEXT NOT NULL,
  tag TEXT NOT NULL COLLATE NOCASE,
  PRIMARY KEY (repo_path, tag),
  FOREIGN KEY (repo_path) REFERENCES locations(path) ON DELETE CASCADE
);

CREATE INDEX idx_repo_tags_path ON repo_tags(repo_path);
CREATE INDEX idx_repo_tags_tag ON repo_tags(tag);

CREATE TABLE repo_hidden_branches (
  repo_path TEXT NOT NULL,
  branch TEXT NOT NULL,
  PRIMARY KEY (repo_path, branch),
  FOREIGN KEY (repo_path) REFERENCES locations(path) ON DELETE CASCADE
);

CREATE INDEX idx_repo_hidden_branches_path ON repo_hidden_branches(repo_path);

CREATE TABLE local_catalog_sync_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  status TEXT NOT NULL CHECK (status IN ('ok', 'error', 'cap_exceeded')),
  added_count INTEGER NOT NULL DEFAULT 0,
  removed_count INTEGER NOT NULL DEFAULT 0,
  skipped_count INTEGER NOT NULL DEFAULT 0,
  message TEXT
);

CREATE TABLE local_catalog_sync_changes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id INTEGER NOT NULL REFERENCES local_catalog_sync_runs(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  action TEXT NOT NULL CHECK (action IN ('added', 'removed', 'skipped'))
);

CREATE INDEX idx_local_catalog_sync_changes_run ON local_catalog_sync_changes(run_id);
