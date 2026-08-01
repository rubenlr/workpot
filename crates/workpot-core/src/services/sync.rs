use crate::domain::Config;
use crate::error::Result;
use crate::infra::db::DbPool;
use crate::services::local_catalog_sync::{self, LocalCatalogSyncSummary};

/// Full sync orchestrator.
///
/// v1 runs local catalog sync only. Remote catalog sync will be composed here later.
pub fn run_full(pool: &DbPool, config: &Config) -> Result<LocalCatalogSyncSummary> {
    // sync-remote-catalog is future scope — not invoked in v1.
    local_catalog_sync::run_phased(pool, config)
}
