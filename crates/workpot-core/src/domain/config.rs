use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const HARD_MAX_WATCH_ROOTS: u32 = 5000;
const HARD_MAX_REPOS: u32 = 20000;
const HARD_MAX_REMOTE_REPOS: u32 = 50000;
/// One day; a longer remote listing TTL would hide upstream changes for too long.
const HARD_MAX_REMOTE_SYNC_TTL_MINUTES: u32 = 1440;
const HARD_MAX_VISIBLE_ROWS: u32 = 100;
const HARD_MAX_PINNED: u32 = 20;
const HARD_MAX_RECENT_DAYS: u32 = 365;
const HARD_MAX_STALE_DIRTY_DAYS: u32 = 365;

/// Default `stale_dirty_days`; also the tray's fallback when config is unreadable.
pub const DEFAULT_STALE_DIRTY_DAYS: u32 = 7;

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

fn default_fetch_cmd() -> String {
    "git -C {path} fetch --prune --no-tags".to_string()
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
    DEFAULT_STALE_DIRTY_DAYS
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
    fn validate(&self) -> Result<(), String> {
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

fn default_max_remote_repos() -> u32 {
    2000
}

fn default_remote_sync_ttl_minutes() -> u32 {
    15
}

fn default_clone_policy() -> ClonePolicy {
    ClonePolicy::Manual
}

fn default_include_archived() -> bool {
    true
}

/// Hosting provider for a remote catalog source.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RemoteProvider {
    Github,
    Gitlab,
}

impl RemoteProvider {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Github => "github",
            Self::Gitlab => "gitlab",
        }
    }

    fn default_host(self) -> &'static str {
        match self {
            Self::Github => "github.com",
            Self::Gitlab => "gitlab.com",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "github" => Some(Self::Github),
            "gitlab" => Some(Self::Gitlab),
            _ => None,
        }
    }
}

/// What Workpot does automatically with indexed-but-not-cloned repos.
///
/// The tray / CLI clone action is available under every policy.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum ClonePolicy {
    /// Nothing automatic; the user clones explicitly.
    Manual,
    /// Clone the first time the user tries to open the repo.
    OnOpen,
    /// Clone every indexed repo during sync.
    Auto,
}

impl ClonePolicy {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Manual => "manual",
            Self::OnOpen => "on_open",
            Self::Auto => "auto",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().replace('-', "_").as_str() {
            "manual" => Some(Self::Manual),
            "on_open" => Some(Self::OnOpen),
            "auto" => Some(Self::Auto),
            _ => None,
        }
    }
}

/// Git transport used to clone a remote repo.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum GitProtocol {
    Ssh,
    Https,
}

impl GitProtocol {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Ssh => "ssh",
            Self::Https => "https",
        }
    }

    pub fn parse(raw: &str) -> Option<Self> {
        match raw.trim().to_ascii_lowercase().as_str() {
            "ssh" => Some(Self::Ssh),
            "https" => Some(Self::Https),
            _ => None,
        }
    }
}

/// A GitHub org/user or GitLab group indexed into the catalog (`[[remotes]]` in config.toml).
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RemoteSource {
    /// Unique label; also the `source_id` in the catalog.
    pub name: String,
    pub provider: RemoteProvider,
    /// Hosting host (GHE / self-hosted). Defaults to the provider's public host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub host: Option<String>,
    /// GitHub org/user, or GitLab group path (subgroups are included recursively).
    pub path: String,
    /// Directory clones are placed under (`~` is expanded).
    pub clone_root: PathBuf,
    #[serde(default = "default_clone_policy")]
    pub clone_policy: ClonePolicy,
    /// Safe-only deletion of local clones whose upstream repo no longer exists.
    #[serde(default)]
    pub prune_removed: bool,
    #[serde(default = "default_include_archived")]
    pub include_archived: bool,
    /// Override of the CLI's configured `git_protocol` for this host.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub protocol: Option<GitProtocol>,
}

impl RemoteSource {
    pub fn effective_host(&self) -> &str {
        match self.host.as_deref().map(str::trim) {
            Some(host) if !host.is_empty() => host,
            _ => self.provider.default_host(),
        }
    }

    /// `path` without surrounding slashes.
    pub fn trimmed_path(&self) -> &str {
        self.path.trim().trim_matches('/')
    }

    /// `clone_root` with a leading `~` expanded to the user's home directory.
    pub fn resolved_clone_root(&self) -> PathBuf {
        expand_tilde(&self.clone_root)
    }

    fn validate(&self) -> Result<(), String> {
        let name = &self.name;
        if name.is_empty()
            || !name
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '.' | '_' | '-'))
        {
            return Err(format!(
                "remotes name {name:?} must be non-empty and use only letters, digits, '.', '_' or '-'"
            ));
        }
        let path = self.trimmed_path();
        if path.is_empty() || path.chars().any(char::is_whitespace) || path.contains("//") {
            return Err(format!("remotes[{name}].path {:?} is invalid", self.path));
        }
        if self.provider == RemoteProvider::Github && path.contains('/') {
            return Err(format!(
                "remotes[{name}].path must be a single org/user for github (got {path:?})"
            ));
        }
        if let Some(host) = self.host.as_deref()
            && (host.trim().is_empty() || host.chars().any(|c| c.is_whitespace() || c == '/'))
        {
            return Err(format!("remotes[{name}].host {host:?} is invalid"));
        }
        if !self.resolved_clone_root().is_absolute() {
            return Err(format!(
                "remotes[{name}].clone_root must be an absolute path (or start with ~)"
            ));
        }
        Ok(())
    }
}

/// Reject a clone root that nests inside (or contains) the root of an earlier source.
fn ensure_clone_root_disjoint(
    name: &str,
    root: &Path,
    seen: &[(&str, PathBuf)],
) -> Result<(), String> {
    match seen
        .iter()
        .find(|(_, other)| root.starts_with(other) || other.starts_with(root))
    {
        Some((other_name, _)) => Err(format!(
            "remotes[{name}].clone_root overlaps remotes[{other_name}].clone_root"
        )),
        None => Ok(()),
    }
}

fn expand_tilde(path: &Path) -> PathBuf {
    let Ok(rest) = path.strip_prefix("~") else {
        return path.to_path_buf();
    };
    match directories::BaseDirs::new() {
        Some(dirs) => dirs.home_dir().join(rest),
        None => path.to_path_buf(),
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Limits {
    #[serde(default = "default_max_watch_roots")]
    pub max_watch_roots: u32,
    #[serde(default = "default_max_repos")]
    pub max_repos: u32,
    /// Maximum remote repos indexed per source.
    #[serde(default = "default_max_remote_repos")]
    pub max_remote_repos: u32,
}

impl Default for Limits {
    fn default() -> Self {
        Self {
            max_watch_roots: default_max_watch_roots(),
            max_repos: default_max_repos(),
            max_remote_repos: default_max_remote_repos(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct Config {
    /// Watch roots for auto-discovery.
    #[serde(default)]
    pub watch_roots: Vec<PathBuf>,
    /// Path patterns excluded from local catalog sync discovery.
    #[serde(default)]
    pub excludes: Vec<String>,
    #[serde(default)]
    pub limits: Limits,
    /// Shell command template for opening a repo in the IDE. `{path}` is substituted.
    #[serde(default = "default_launch_cmd")]
    pub launch_cmd: String,
    /// Shell command template for pushing a branch. `{path}` and `{branch}` are substituted.
    #[serde(default = "default_push_cmd")]
    pub push_cmd: String,
    /// Shell command template for pulling a branch. `{path}` and `{branch}` are substituted.
    #[serde(default = "default_pull_cmd")]
    pub pull_cmd: String,
    /// Shell command template for fetching remotes before git-state refresh.
    /// Empty string disables fetch. Non-empty must contain `{path}`.
    #[serde(default = "default_fetch_cmd")]
    pub fetch: String,
    /// Maximum repo rows visible in the tray panel before scrolling.
    #[serde(default = "default_max_visible_rows")]
    pub max_visible_rows: u32,
    /// Maximum pinned repos in the tray.
    #[serde(default = "default_max_pinned")]
    pub max_pinned: u32,
    /// Recency window for the Recent section, in days.
    #[serde(default = "default_max_recent_days")]
    pub max_recent_days: u32,
    /// Minimum Recent section size via padding.
    #[serde(default = "default_min_recent_count")]
    pub min_recent_count: u32,
    /// Threshold for stale-dirty tray icon: dirty repo last opened longer ago than this.
    #[serde(default = "default_stale_dirty_days")]
    pub stale_dirty_days: u32,
    /// Minimum minutes between remote catalog listings triggered by panel open (0 = always).
    #[serde(default = "default_remote_sync_ttl_minutes")]
    pub remote_sync_ttl_minutes: u32,
    #[serde(default)]
    pub migration: MigrationConfig,
    /// Remote catalog sources (`[[remotes]]`).
    #[serde(default)]
    pub remotes: Vec<RemoteSource>,
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
            fetch: default_fetch_cmd(),
            max_visible_rows: default_max_visible_rows(),
            max_pinned: default_max_pinned(),
            max_recent_days: default_max_recent_days(),
            min_recent_count: default_min_recent_count(),
            stale_dirty_days: default_stale_dirty_days(),
            remote_sync_ttl_minutes: default_remote_sync_ttl_minutes(),
            migration: MigrationConfig::default(),
            remotes: Vec::new(),
        }
    }
}

impl Config {
    /// Validate limits, shell command templates, tray settings, migration and remote sources.
    pub fn validate(&self) -> Result<(), String> {
        self.validate_index_limits()?;
        self.validate_shell_commands()?;
        self.validate_tray_settings()?;
        self.migration.validate()?;
        self.validate_remotes()?;
        Ok(())
    }

    /// Source by `name`, if configured.
    pub fn remote(&self, name: &str) -> Option<&RemoteSource> {
        self.remotes.iter().find(|r| r.name == name)
    }

    fn validate_remotes(&self) -> Result<(), String> {
        validate_u32_range(
            "limits.max_remote_repos",
            self.limits.max_remote_repos,
            1,
            HARD_MAX_REMOTE_REPOS,
        )?;
        validate_u32_range(
            "remote_sync_ttl_minutes",
            self.remote_sync_ttl_minutes,
            0,
            HARD_MAX_REMOTE_SYNC_TTL_MINUTES,
        )?;
        let mut roots: Vec<(&str, PathBuf)> = Vec::new();
        for (idx, source) in self.remotes.iter().enumerate() {
            source.validate()?;
            if self.remotes[..idx].iter().any(|r| r.name == source.name) {
                return Err(format!("duplicate remotes name {:?}", source.name));
            }
            let root = source.resolved_clone_root();
            ensure_clone_root_disjoint(&source.name, &root, &roots)?;
            roots.push((source.name.as_str(), root));
        }
        Ok(())
    }

    fn validate_index_limits(&self) -> Result<(), String> {
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
        Ok(())
    }

    fn validate_shell_commands(&self) -> Result<(), String> {
        validate_shell_cmd("launch_cmd", &self.launch_cmd, &["{path}"])?;
        validate_shell_cmd("push_cmd", &self.push_cmd, &["{path}", "{branch}"])?;
        validate_shell_cmd("pull_cmd", &self.pull_cmd, &["{path}", "{branch}"])?;
        // Empty/whitespace fetch disables remote fetch during catalog sync.
        if !self.fetch.trim().is_empty() {
            validate_shell_cmd("fetch", &self.fetch, &["{path}"])?;
        }
        Ok(())
    }

    fn validate_tray_settings(&self) -> Result<(), String> {
        validate_u32_range(
            "max_visible_rows",
            self.max_visible_rows,
            1,
            HARD_MAX_VISIBLE_ROWS,
        )?;
        validate_u32_range("max_pinned", self.max_pinned, 1, HARD_MAX_PINNED)?;
        validate_u32_range(
            "max_recent_days",
            self.max_recent_days,
            1,
            HARD_MAX_RECENT_DAYS,
        )?;
        if self.min_recent_count > self.max_pinned {
            return Err(format!(
                "min_recent_count {} must be <= max_pinned {}",
                self.min_recent_count, self.max_pinned
            ));
        }
        validate_u32_range(
            "stale_dirty_days",
            self.stale_dirty_days,
            1,
            HARD_MAX_STALE_DIRTY_DAYS,
        )?;
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
