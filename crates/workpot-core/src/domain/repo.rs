use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRecord {
    pub path: PathBuf,
    pub name: String,
    pub registered_at: i64,
    pub source: String,
}
