//! Manage `[[remotes]]` sources in config.toml (add / set / remove).

use crate::AppState;
use crate::domain::config::{ClonePolicy, GitProtocol, RemoteSource};
use crate::error::{Result, WorkpotError};
use crate::save_config;
use crate::services::remote_catalog_store::purge_source;
use std::path::PathBuf;

/// Partial update of a source; `None` leaves the field unchanged.
#[derive(Debug, Clone, Default)]
pub struct RemotePatch {
    pub clone_policy: Option<ClonePolicy>,
    pub prune_removed: Option<bool>,
    pub include_archived: Option<bool>,
    pub clone_root: Option<PathBuf>,
    /// `Some(None)` clears the override (use the CLI's `git_protocol`).
    pub protocol: Option<Option<GitProtocol>>,
}

/// Apply `mutate` to the config, validate, persist; restore the previous config on failure.
fn update_config(
    state: &AppState,
    mutate: impl FnOnce(&mut Vec<RemoteSource>) -> Result<()>,
) -> Result<()> {
    let mut config = state.config_mut()?;
    let previous = config.remotes.clone();
    mutate(&mut config.remotes)?;
    let result = config
        .validate()
        .map_err(WorkpotError::Config)
        .and_then(|()| save_config(state.config_path(), &config));
    if result.is_err() {
        config.remotes = previous;
    }
    result
}

pub(crate) fn add_remote(state: &AppState, source: RemoteSource) -> Result<()> {
    update_config(state, |remotes| {
        if remotes.iter().any(|r| r.name == source.name) {
            return Err(WorkpotError::InvalidInput(format!(
                "remote source {:?} already exists",
                source.name
            )));
        }
        remotes.push(source);
        Ok(())
    })
}

pub(crate) fn set_remote(state: &AppState, name: &str, patch: RemotePatch) -> Result<()> {
    update_config(state, |remotes| {
        let source = remotes
            .iter_mut()
            .find(|r| r.name == name)
            .ok_or_else(|| WorkpotError::NotFound(format!("remote source {name:?}")))?;
        if let Some(policy) = patch.clone_policy {
            source.clone_policy = policy;
        }
        if let Some(prune) = patch.prune_removed {
            source.prune_removed = prune;
        }
        if let Some(include) = patch.include_archived {
            source.include_archived = include;
        }
        if let Some(root) = patch.clone_root {
            source.clone_root = root;
        }
        if let Some(protocol) = patch.protocol {
            source.protocol = protocol;
        }
        Ok(())
    })
}

/// Remove a source and its index. Existing local clones stay on disk and in the catalog as
/// ordinary repos.
pub(crate) fn remove_remote(state: &AppState, name: &str) -> Result<()> {
    update_config(state, |remotes| {
        let before = remotes.len();
        remotes.retain(|r| r.name != name);
        if remotes.len() == before {
            return Err(WorkpotError::NotFound(format!("remote source {name:?}")));
        }
        Ok(())
    })?;
    state.with_write_connection(|conn| purge_source(conn, name))
}
