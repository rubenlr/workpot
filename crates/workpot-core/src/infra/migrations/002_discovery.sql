ALTER TABLE repos ADD COLUMN git_common_dir TEXT NOT NULL DEFAULT '';

CREATE TABLE index_runs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  started_at INTEGER NOT NULL,
  finished_at INTEGER,
  status TEXT NOT NULL CHECK (status IN ('ok', 'error', 'cap_exceeded')),
  added_count INTEGER NOT NULL DEFAULT 0,
  removed_count INTEGER NOT NULL DEFAULT 0,
  skipped_count INTEGER NOT NULL DEFAULT 0,
  message TEXT
);

CREATE TABLE index_changes (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  run_id INTEGER NOT NULL REFERENCES index_runs(id) ON DELETE CASCADE,
  path TEXT NOT NULL,
  action TEXT NOT NULL CHECK (action IN ('added', 'removed', 'skipped'))
);

CREATE INDEX idx_repos_git_common_dir ON repos(git_common_dir);
CREATE INDEX idx_index_changes_run ON index_changes(run_id);
