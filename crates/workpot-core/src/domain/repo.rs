use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RepoRecord {
    pub path: PathBuf,
    pub name: String,
    pub registered_at: i64,
    pub source: String,
    pub git_common_dir: String,
    pub branch: Option<String>,
    pub is_dirty: Option<bool>,        // None = bare repo (D-13); false = clean; true = dirty
    pub ahead: Option<i64>,            // None = no upstream (D-04)
    pub behind: Option<i64>,           // None = no upstream (D-04)
    pub git_refreshed_at: Option<i64>, // None = never refreshed (D-06)
    pub git_state_error: Option<String>, // last failure message (D-09)
}
