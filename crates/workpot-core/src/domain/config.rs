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

fn default_push_cmd() -> String {
    "git -C {path} push origin {branch}".to_string()
}

fn default_pull_cmd() -> String {
    "git -C {path} pull origin {branch}".to_string()
}

fn default_max_visible_rows() -> u32 {
    15
}

fn default_max_pinned() -> u32 {
    5
}

fn default_max_recent_days() -> u32 {
    14
}

fn default_min_recent_count() -> u32 {
    3
}

fn default_stale_dirty_days() -> u32 {
    7
}

fn default_temp_suffix() -> String {
    ".temp".to_string()
}

fn default_delete_original() -> bool {
    false
}

fn default_bare_repo_template() -> String {
    "{project}/bare.git".to_string()
}

fn default_worktree_template() -> String {
    "{project}/wtrees/{worktree}".to_string()
}

fn default_project_name_source() -> ProjectNameSource {
    ProjectNameSource::FolderName
}

fn default_allow_conversion_to_bare_repo() -> bool {
    false
}

fn default_allow_conversion_to_local_repo() -> bool {
    false
}

fn default_repo_layout() -> RepoLayout {
    RepoLayout::Local
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RepoLayout {
    Bare,
    Local,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ProjectNameSource {
    FolderName,
    Alias,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct MigrationConfig {
    #[serde(default = "default_temp_suffix")]
    pub temp_suffix: String,
    #[serde(default = "default_delete_original")]
    pub delete_original: bool,
    #[serde(default = "default_bare_repo_template")]
    pub bare_repo_template: String,
    #[serde(default = "default_worktree_template")]
    pub worktree_template: String,
    #[serde(default = "default_project_name_source")]
    pub project_name_source: ProjectNameSource,
    #[serde(default = "default_allow_conversion_to_bare_repo")]
    pub allow_conversion_to_bare_repo: bool,
    #[serde(default = "default_allow_conversion_to_local_repo")]
    pub allow_conversion_to_local_repo: bool,
    #[serde(default = "default_repo_layout")]
    pub default_repo_layout: RepoLayout,
}

impl Default for MigrationConfig {
    fn default() -> Self {
        Self {
            temp_suffix: default_temp_suffix(),
            delete_original: default_delete_original(),
            bare_repo_template: default_bare_repo_template(),
            worktree_template: default_worktree_template(),
            project_name_source: default_project_name_source(),
            allow_conversion_to_bare_repo: default_allow_conversion_to_bare_repo(),
            allow_conversion_to_local_repo: default_allow_conversion_to_local_repo(),
            default_repo_layout: default_repo_layout(),
        }
    }
}

impl MigrationConfig {
    pub(super) fn validate(&self) -> Result<(), String> {
        if self.temp_suffix.is_empty() {
            return Err("migration.temp_suffix must not be empty".into());
        }
        validate_shell_cmd(
            "migration.bare_repo_template",
            &self.bare_repo_template,
            &["{project}"],
        )?;
        validate_shell_cmd(
            "migration.worktree_template",
            &self.worktree_template,
            &["{project}", "{worktree}"],
        )?;
        Ok(())
    }
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
    /// Shell command template for pushing a branch. `{path}` and `{branch}` are substituted.
    #[serde(default = "default_push_cmd")]
    pub push_cmd: String,
    /// Shell command template for pulling a branch. `{path}` and `{branch}` are substituted.
    #[serde(default = "default_pull_cmd")]
    pub pull_cmd: String,
    /// Maximum repo rows visible in the tray panel before scrolling (D-12).
    #[serde(default = "default_max_visible_rows")]
    pub max_visible_rows: u32,
    /// Maximum pinned repos in the tray (Phase 5).
    #[serde(default = "default_max_pinned")]
    pub max_pinned: u32,
    /// Recency window for the Recent section, in days (Phase 5).
    #[serde(default = "default_max_recent_days")]
    pub max_recent_days: u32,
    /// Minimum Recent section size via padding (Phase 5).
    #[serde(default = "default_min_recent_count")]
    pub min_recent_count: u32,
    /// Threshold for stale-dirty tray icon: dirty repo last opened longer ago than this (06.2).
    #[serde(default = "default_stale_dirty_days")]
    pub stale_dirty_days: u32,
    #[serde(default)]
    pub migration: MigrationConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            watch_roots: Vec::new(),
            excludes: Vec::new(),
            limits: Limits::default(),
            launch_cmd: default_launch_cmd(),
            push_cmd: default_push_cmd(),
            pull_cmd: default_pull_cmd(),
            max_visible_rows: default_max_visible_rows(),
            max_pinned: default_max_pinned(),
            max_recent_days: default_max_recent_days(),
            min_recent_count: default_min_recent_count(),
            stale_dirty_days: default_stale_dirty_days(),
            migration: MigrationConfig::default(),
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
        validate_u32_range("max_visible_rows", self.max_visible_rows, 1, 100)?;
        validate_shell_cmd("launch_cmd", &self.launch_cmd, &["{path}"])?;
        validate_shell_cmd("push_cmd", &self.push_cmd, &["{path}", "{branch}"])?;
        validate_shell_cmd("pull_cmd", &self.pull_cmd, &["{path}", "{branch}"])?;
        validate_u32_range("max_pinned", self.max_pinned, 1, 20)?;
        validate_u32_range("max_recent_days", self.max_recent_days, 1, 365)?;
        if self.min_recent_count > self.max_pinned {
            return Err(format!(
                "min_recent_count {} must be <= max_pinned {}",
                self.min_recent_count, self.max_pinned
            ));
        }
        validate_u32_range("stale_dirty_days", self.stale_dirty_days, 1, 365)?;
        self.migration.validate()?;
        Ok(())
    }
}

fn validate_u32_range(name: &str, value: u32, min: u32, max: u32) -> Result<(), String> {
    if value < min || value > max {
        return Err(format!("{name} {value} must be between {min} and {max}"));
    }
    Ok(())
}

fn validate_shell_cmd(name: &str, cmd: &str, placeholders: &[&str]) -> Result<(), String> {
    if cmd.trim().is_empty() {
        return Err(format!("{name} must not be empty"));
    }
    for placeholder in placeholders {
        if !cmd.contains(placeholder) {
            return Err(if placeholders.len() == 1 {
                format!("{name} must contain {placeholder} placeholder")
            } else {
                format!(
                    "{name} must contain {} placeholders",
                    placeholders.join(" and ")
                )
            });
        }
    }
    Ok(())
}
