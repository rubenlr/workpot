-- Remote catalog: repos indexed from GitHub/GitLab groups (not necessarily cloned).
-- "Cloned" is derived by joining url_normalized against project_remotes of non-excluded
-- locations; remote-only repos never get a `projects` row.

CREATE TABLE remote_repos (
  source_id TEXT NOT NULL,
  full_path TEXT NOT NULL,
  name TEXT NOT NULL,
  web_url TEXT,
  ssh_url TEXT,
  https_url TEXT,
  url_normalized TEXT NOT NULL,
  default_branch TEXT,
  archived INTEGER NOT NULL DEFAULT 0 CHECK (archived IN (0, 1)),
  description TEXT,
  clone_path TEXT NOT NULL,
  state TEXT NOT NULL DEFAULT 'present' CHECK (state IN ('present', 'gone')),
  first_seen_at INTEGER NOT NULL,
  last_seen_at INTEGER NOT NULL,
  PRIMARY KEY (source_id, full_path)
);

CREATE INDEX idx_remote_repos_url_normalized ON remote_repos(url_normalized);
CREATE INDEX idx_remote_repos_clone_path ON remote_repos(clone_path);

CREATE TABLE remote_sync_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  source_id TEXT NOT NULL,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  status TEXT NOT NULL CHECK (status IN ('ok', 'error')),
  repo_count INTEGER NOT NULL DEFAULT 0,
  new_count INTEGER NOT NULL DEFAULT 0,
  gone_count INTEGER NOT NULL DEFAULT 0,
  cloned_count INTEGER NOT NULL DEFAULT 0,
  pruned_count INTEGER NOT NULL DEFAULT 0,
  message TEXT
);

CREATE INDEX idx_remote_sync_runs_source ON remote_sync_runs(source_id, id);
