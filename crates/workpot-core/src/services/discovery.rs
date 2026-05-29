use crate::error::Result;
use globset::GlobSet;
use std::path::{Path, PathBuf};

/// Walk a watch root and return candidate repo paths (implemented in 02-02).
pub fn scan_root(_root: &Path, _exclude_set: &GlobSet) -> Result<Vec<PathBuf>> {
    todo!("DiscoveryService::scan_root — plan 02-02")
}
