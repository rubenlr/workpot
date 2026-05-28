CREATE TABLE repos (
  path TEXT NOT NULL PRIMARY KEY,
  name TEXT NOT NULL,
  registered_at INTEGER NOT NULL,
  source TEXT NOT NULL DEFAULT 'manual' CHECK (source IN ('manual', 'scan')),
  excluded INTEGER NOT NULL DEFAULT 0 CHECK (excluded IN (0, 1))
);

CREATE INDEX idx_repos_registered_at ON repos(registered_at);
