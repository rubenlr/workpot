use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    #[serde(default)]
    pub watch_roots: Vec<PathBuf>,
    #[serde(default)]
    pub excludes: Vec<String>,
}
