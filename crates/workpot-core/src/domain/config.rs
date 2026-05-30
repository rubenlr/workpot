use serde::{Deserialize, Serialize};
use std::path::PathBuf;

const HARD_MAX_WATCH_ROOTS: u32 = 5000;
const HARD_MAX_REPOS: u32 = 20000;

fn default_max_watch_roots() -> u32 {
    100
}

fn default_max_repos() -> u32 {
    1000
}

fn default_launch_cmd() -> String {
    "cursor --new-window {path}".to_string()
}

fn default_max_visible_rows() -> u32 {
    15
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Limits {
    #[serde(default = "default_max_watch_roots")]
    pub max_watch_roots: u32,
    #[serde(default = "default_max_repos")]
    pub max_repos: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_watch_roots: default_max_watch_roots(),
            max_repos: default_max_repos(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    /// Watch roots for auto-discovery (consumed in Phase 2).
    #[serde(default)]
    pub watch_roots: Vec<PathBuf>,
    /// Path patterns excluded from indexing (consumed in Phase 2).
    #[serde(default)]
    pub excludes: Vec<String>,
    #[serde(default)]
    pub limits: Limits,
    /// Shell command template for opening a repo in the IDE (D-33). `{path}` is substituted.
    #[serde(default = "default_launch_cmd")]
    pub launch_cmd: String,
    /// Maximum repo rows visible in the tray panel before scrolling (D-12).
    #[serde(default = "default_max_visible_rows")]
    pub max_visible_rows: u32,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watch_roots: Vec::new(),
            excludes: Vec::new(),
            limits: Limits::default(),
            launch_cmd: default_launch_cmd(),
            max_visible_rows: default_max_visible_rows(),
        }
    }
}

impl Config {
    /// Reject pathological limit values (D-22, D-23).
    pub fn validate(&self) -> Result<(), String> {
        if self.limits.max_watch_roots > HARD_MAX_WATCH_ROOTS {
            return Err(format!(
                "max_watch_roots {} exceeds hard max {HARD_MAX_WATCH_ROOTS}",
                self.limits.max_watch_roots
            ));
        }
        if self.limits.max_repos > HARD_MAX_REPOS {
            return Err(format!(
                "max_repos {} exceeds hard max {HARD_MAX_REPOS}",
                self.limits.max_repos
            ));
        }
        if self.watch_roots.len() > self.limits.max_watch_roots as usize {
            return Err(format!(
                "watch_roots count {} exceeds max_watch_roots {}",
                self.watch_roots.len(),
                self.limits.max_watch_roots
            ));
        }
        if self.max_visible_rows < 1 || self.max_visible_rows > 100 {
            return Err(format!(
                "max_visible_rows {} must be between 1 and 100",
                self.max_visible_rows
            ));
        }
        if self.launch_cmd.trim().is_empty() {
            return Err("launch_cmd must not be empty".into());
        }
        if !self.launch_cmd.contains("{path}") {
            return Err("launch_cmd must contain {path} placeholder".into());
        }
        Ok(())
    }
}
