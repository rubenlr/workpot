ALTER TABLE index_runs RENAME TO local_catalog_sync_runs;
ALTER TABLE index_changes RENAME TO local_catalog_sync_changes;
DROP INDEX IF EXISTS idx_index_changes_run;
CREATE INDEX idx_local_catalog_sync_changes_run ON local_catalog_sync_changes(run_id);
