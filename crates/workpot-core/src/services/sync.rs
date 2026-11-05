use crate::domain::Config;
use crate::error::Result;
use crate::infra::db::DbPool;
use crate::infra::remote_provider::RemoteApi;
use crate::services::local_catalog_sync::{self, LocalCatalogSyncSummary};
use crate::services::remote_catalog_sync::{self, RemoteSourceError, RemoteSyncSummary};

/// Result of the full sync orchestrator: remote catalog index, then local catalog sync.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct FullSyncSummary {
    pub local: LocalCatalogSyncSummary,
    pub remote: RemoteSyncSummary,
}

/// Full sync orchestrator.
///
/// Remote catalog first (so fresh clones are picked up by the local pass), then local catalog
/// sync. Remote failures are soft: they land in `remote.source_errors` and never block the
/// local pass. `force_remote` bypasses the remote TTL.
pub fn run_full(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    force_remote: bool,
) -> Result<FullSyncSummary> {
    let remote = run_remote(pool, config, api, force_remote);
    let local = local_catalog_sync::run_phased(pool, config)?;
    Ok(FullSyncSummary { local, remote })
}

/// Remote catalog pass only; never fails (errors are folded into the summary).
pub fn run_remote(
    pool: &DbPool,
    config: &Config,
    api: &dyn RemoteApi,
    force: bool,
) -> RemoteSyncSummary {
    if config.remotes.is_empty() {
        return RemoteSyncSummary::default();
    }
    match remote_catalog_sync::run_remote_sync(pool, config, api, force) {
        Ok(summary) => summary,
        Err(e) => {
            log::warn!("remote catalog sync failed: {e}");
            RemoteSyncSummary {
                sources: u32::try_from(config.remotes.len()).unwrap_or(u32::MAX),
                errors: 1,
                source_errors: vec![RemoteSourceError {
                    source: "*".to_string(),
                    message: e.to_string(),
                }],
                ..RemoteSyncSummary::default()
            }
        }
    }
}
