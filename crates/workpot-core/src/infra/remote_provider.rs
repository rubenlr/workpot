//! Remote catalog listing through the `gh` / `glab` CLIs (no tokens handled by Workpot).

use crate::domain::config::{GitProtocol, RemoteProvider, RemoteSource};
use serde_json::Value;
use std::path::PathBuf;
use std::process::Command;
use thiserror::Error;

const PER_PAGE: &str = "per_page=100";

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteRepoInfo {
    /// Full path on the host, e.g. `acme/platform/api`.
    pub full_path: String,
    pub name: String,
    pub web_url: Option<String>,
    pub ssh_url: Option<String>,
    pub https_url: Option<String>,
    pub default_branch: Option<String>,
    pub archived: bool,
    pub description: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Error)]
pub enum RemoteError {
    #[error("`{cli}` CLI not found; install it and run `{cli} auth login`")]
    CliMissing { cli: String },
    #[error("not authenticated for {host} — run `{cli} auth login --hostname {host}`")]
    NotAuthenticated { cli: String, host: String },
    #[error("not found: {0}")]
    NotFound(String),
    #[error("remote request failed: {0}")]
    Failed(String),
}

#[derive(Debug, Clone)]
pub struct CommandOutput {
    pub success: bool,
    pub stdout: String,
    pub stderr: String,
}

/// Process boundary so tests never spawn `gh` / `glab`.
pub trait CommandRunner: Send + Sync {
    fn run(&self, program: &str, args: &[String]) -> std::io::Result<CommandOutput>;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemRunner;

impl CommandRunner for SystemRunner {
    fn run(&self, program: &str, args: &[String]) -> std::io::Result<CommandOutput> {
        let resolved = resolve_cli(program);
        log::debug!("remote provider: {resolved} {}", args.join(" "));
        let output = Command::new(&resolved)
            .args(args)
            .env("NO_COLOR", "1")
            .env("GH_PROMPT_DISABLED", "1")
            .env("GH_NO_UPDATE_NOTIFIER", "1")
            .env("GLAB_CHECK_UPDATE", "false")
            .output()?;
        Ok(CommandOutput {
            success: output.status.success(),
            stdout: String::from_utf8_lossy(&output.stdout).into_owned(),
            stderr: String::from_utf8_lossy(&output.stderr).into_owned(),
        })
    }
}

/// GUI apps (tray) inherit a minimal PATH; probe the usual install locations first.
/// `WORKPOT_CLI_DIR` (a directory holding `gh` / `glab`) takes precedence; used by tests.
fn resolve_cli(program: &str) -> String {
    let mut candidates: Vec<PathBuf> = Vec::new();
    if let Some(dir) = std::env::var_os("WORKPOT_CLI_DIR") {
        candidates.push(PathBuf::from(dir).join(program));
    }
    candidates.push(PathBuf::from("/opt/homebrew/bin").join(program));
    candidates.push(PathBuf::from("/usr/local/bin").join(program));
    if let Some(home) = std::env::var_os("HOME") {
        candidates.push(PathBuf::from(home).join(".local/bin").join(program));
    }
    for candidate in candidates {
        if candidate.is_file() {
            return candidate.display().to_string();
        }
    }
    program.to_string()
}

/// Provider operations the remote catalog needs.
pub trait RemoteApi: Send + Sync {
    /// Every repo under the source (GitLab: subgroups included). Archived repos are always
    /// returned so archiving is never mistaken for removal.
    fn list_repos(&self, source: &RemoteSource) -> Result<Vec<RemoteRepoInfo>, RemoteError>;

    /// `Ok(false)` only when the host confirms the repo does not exist (HTTP 404).
    fn repo_exists(&self, source: &RemoteSource, full_path: &str) -> Result<bool, RemoteError>;

    /// Transport configured in the CLI for the source's host (`git_protocol`).
    fn cli_git_protocol(&self, source: &RemoteSource) -> Option<GitProtocol>;
}

pub struct GhGlabProvider<R: CommandRunner = SystemRunner> {
    runner: R,
}

impl GhGlabProvider<SystemRunner> {
    pub fn system() -> Self {
        Self {
            runner: SystemRunner,
        }
    }
}

impl<R: CommandRunner> GhGlabProvider<R> {
    pub fn with_runner(runner: R) -> Self {
        Self { runner }
    }

    fn cli(provider: RemoteProvider) -> &'static str {
        match provider {
            RemoteProvider::Github => "gh",
            RemoteProvider::Gitlab => "glab",
        }
    }

    fn api(
        &self,
        source: &RemoteSource,
        endpoint: &str,
        paginate: bool,
    ) -> Result<String, RemoteError> {
        let cli = Self::cli(source.provider);
        let host = source.effective_host();
        let mut args: Vec<String> = vec!["api".into(), "--hostname".into(), host.into()];
        if paginate {
            args.push("--paginate".into());
        }
        args.push(endpoint.to_string());
        let output = self.runner.run(cli, &args).map_err(|e| {
            if e.kind() == std::io::ErrorKind::NotFound {
                RemoteError::CliMissing { cli: cli.into() }
            } else {
                RemoteError::Failed(format!("failed to run {cli}: {e}"))
            }
        })?;
        if output.success {
            return Ok(output.stdout);
        }
        Err(classify_failure(
            cli,
            host,
            endpoint,
            &output.stderr,
            &output.stdout,
        ))
    }

    fn list_github(&self, source: &RemoteSource) -> Result<Vec<RemoteRepoInfo>, RemoteError> {
        let path = source.trimmed_path();
        let org = format!("orgs/{path}/repos?{PER_PAGE}&type=all");
        let stdout = match self.api(source, &org, true) {
            Ok(out) => out,
            Err(RemoteError::NotFound(_)) => {
                let user = format!("users/{path}/repos?{PER_PAGE}&type=owner");
                self.api(source, &user, true)?
            }
            Err(e) => return Err(e),
        };
        parse_github_repos(&stdout)
    }

    fn list_gitlab(&self, source: &RemoteSource) -> Result<Vec<RemoteRepoInfo>, RemoteError> {
        let group = url_encode_path(source.trimmed_path());
        let endpoint =
            format!("groups/{group}/projects?include_subgroups=true&with_shared=false&{PER_PAGE}");
        let stdout = self.api(source, &endpoint, true)?;
        parse_gitlab_projects(&stdout)
    }
}

impl<R: CommandRunner> RemoteApi for GhGlabProvider<R> {
    fn list_repos(&self, source: &RemoteSource) -> Result<Vec<RemoteRepoInfo>, RemoteError> {
        match source.provider {
            RemoteProvider::Github => self.list_github(source),
            RemoteProvider::Gitlab => self.list_gitlab(source),
        }
    }

    fn repo_exists(&self, source: &RemoteSource, full_path: &str) -> Result<bool, RemoteError> {
        let endpoint = match source.provider {
            RemoteProvider::Github => format!("repos/{full_path}"),
            RemoteProvider::Gitlab => format!("projects/{}", url_encode_path(full_path)),
        };
        match self.api(source, &endpoint, false) {
            Ok(_) => Ok(true),
            Err(RemoteError::NotFound(_)) => Ok(false),
            Err(e) => Err(e),
        }
    }

    fn cli_git_protocol(&self, source: &RemoteSource) -> Option<GitProtocol> {
        let cli = Self::cli(source.provider);
        let args: Vec<String> = vec![
            "config".into(),
            "get".into(),
            "git_protocol".into(),
            "--host".into(),
            source.effective_host().into(),
        ];
        let output = self.runner.run(cli, &args).ok()?;
        if !output.success {
            return None;
        }
        GitProtocol::parse(output.stdout.trim())
    }
}

/// Classify a failed `gh api` / `glab api` call from its output.
pub(crate) fn classify_failure(
    cli: &str,
    host: &str,
    endpoint: &str,
    stderr: &str,
    stdout: &str,
) -> RemoteError {
    let text = format!("{stderr}\n{stdout}").to_ascii_lowercase();
    if text.contains("http 404")
        || text.contains("404 not found")
        || text.contains("404 group not found")
        || text.contains("404 project not found")
    {
        return RemoteError::NotFound(endpoint.to_string());
    }
    if text.contains("http 401")
        || text.contains("http 403")
        || text.contains("401 unauthorized")
        || text.contains("auth login")
        || text.contains("not logged in")
        || text.contains("no token")
        || text.contains("authentication")
    {
        return RemoteError::NotAuthenticated {
            cli: cli.to_string(),
            host: host.to_string(),
        };
    }
    let detail = stderr.trim();
    let detail = if detail.is_empty() {
        stdout.trim()
    } else {
        detail
    };
    RemoteError::Failed(format!("{cli} api {endpoint}: {detail}"))
}

/// Percent-encode a path for use as a single URL segment (`a/b` → `a%2Fb`).
pub(crate) fn url_encode_path(path: &str) -> String {
    let mut out = String::with_capacity(path.len());
    for byte in path.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                out.push(char::from(byte));
            }
            other => out.push_str(&format!("%{other:02X}")),
        }
    }
    out
}

/// Flatten `--paginate` output: one array, or several arrays concatenated back to back.
pub(crate) fn parse_json_items(stdout: &str) -> Result<Vec<Value>, RemoteError> {
    let mut items = Vec::new();
    for value in serde_json::Deserializer::from_str(stdout).into_iter::<Value>() {
        match value.map_err(|e| RemoteError::Failed(format!("invalid JSON from CLI: {e}")))? {
            Value::Array(page) => items.extend(page),
            other => items.push(other),
        }
    }
    Ok(items)
}

fn str_field(item: &Value, key: &str) -> Option<String> {
    item.get(key)
        .and_then(Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
}

pub(crate) fn parse_github_repos(stdout: &str) -> Result<Vec<RemoteRepoInfo>, RemoteError> {
    let mut repos = Vec::new();
    for item in parse_json_items(stdout)? {
        let Some(full_path) = str_field(&item, "full_name") else {
            return Err(RemoteError::Failed(
                "unexpected GitHub payload: repo without full_name".into(),
            ));
        };
        let name = str_field(&item, "name").unwrap_or_else(|| last_segment(&full_path));
        repos.push(RemoteRepoInfo {
            full_path,
            name,
            web_url: str_field(&item, "html_url"),
            ssh_url: str_field(&item, "ssh_url"),
            https_url: str_field(&item, "clone_url"),
            default_branch: str_field(&item, "default_branch"),
            archived: item
                .get("archived")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            description: str_field(&item, "description"),
        });
    }
    Ok(repos)
}

pub(crate) fn parse_gitlab_projects(stdout: &str) -> Result<Vec<RemoteRepoInfo>, RemoteError> {
    let mut repos = Vec::new();
    for item in parse_json_items(stdout)? {
        let Some(full_path) = str_field(&item, "path_with_namespace") else {
            return Err(RemoteError::Failed(
                "unexpected GitLab payload: project without path_with_namespace".into(),
            ));
        };
        let name = str_field(&item, "path").unwrap_or_else(|| last_segment(&full_path));
        repos.push(RemoteRepoInfo {
            full_path,
            name,
            web_url: str_field(&item, "web_url"),
            ssh_url: str_field(&item, "ssh_url_to_repo"),
            https_url: str_field(&item, "http_url_to_repo"),
            default_branch: str_field(&item, "default_branch"),
            archived: item
                .get("archived")
                .and_then(Value::as_bool)
                .unwrap_or(false),
            description: str_field(&item, "description"),
        });
    }
    Ok(repos)
}

fn last_segment(path: &str) -> String {
    path.rsplit('/').next().unwrap_or(path).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::config::ClonePolicy;
    use std::sync::Mutex;

    struct FakeRunner {
        calls: Mutex<Vec<Vec<String>>>,
        responses: Mutex<Vec<std::io::Result<CommandOutput>>>,
    }

    impl FakeRunner {
        fn new(responses: Vec<std::io::Result<CommandOutput>>) -> Self {
            Self {
                calls: Mutex::new(Vec::new()),
                responses: Mutex::new(responses),
            }
        }
    }

    impl CommandRunner for &FakeRunner {
        fn run(&self, program: &str, args: &[String]) -> std::io::Result<CommandOutput> {
            let mut call = vec![program.to_string()];
            call.extend(args.iter().cloned());
            if let Ok(mut calls) = self.calls.lock() {
                calls.push(call);
            }
            match self.responses.lock() {
                Ok(mut r) if !r.is_empty() => r.remove(0),
                _ => Err(std::io::Error::other("no canned response")),
            }
        }
    }

    fn ok(stdout: &str) -> std::io::Result<CommandOutput> {
        Ok(CommandOutput {
            success: true,
            stdout: stdout.into(),
            stderr: String::new(),
        })
    }

    fn fail(stderr: &str) -> std::io::Result<CommandOutput> {
        Ok(CommandOutput {
            success: false,
            stdout: String::new(),
            stderr: stderr.into(),
        })
    }

    fn source(provider: RemoteProvider, path: &str) -> RemoteSource {
        RemoteSource {
            name: "acme".into(),
            provider,
            host: Some("git.example.com".into()),
            path: path.into(),
            clone_root: "/tmp/acme".into(),
            clone_policy: ClonePolicy::Manual,
            prune_removed: false,
            include_archived: true,
            protocol: None,
        }
    }

    #[test]
    fn url_encode_path_encodes_slashes() {
        assert_eq!(
            url_encode_path("acme/platform/api"),
            "acme%2Fplatform%2Fapi"
        );
        assert_eq!(url_encode_path("a b"), "a%20b");
    }

    #[test]
    fn parse_json_items_handles_concatenated_pages() {
        let items = parse_json_items("[{\"a\":1},{\"a\":2}][{\"a\":3}]\n").expect("parse");
        assert_eq!(items.len(), 3);
        assert!(parse_json_items("[]").expect("empty").is_empty());
        assert!(parse_json_items("").expect("blank").is_empty());
        assert!(parse_json_items("[{").is_err());
    }

    #[test]
    fn parse_gitlab_projects_maps_fields() {
        let json = r#"[{"path_with_namespace":"acme/platform/api","path":"api",
            "web_url":"https://git.example.com/acme/platform/api",
            "ssh_url_to_repo":"git@git.example.com:acme/platform/api.git",
            "http_url_to_repo":"https://git.example.com/acme/platform/api.git",
            "default_branch":"main","archived":true,"description":""}]"#;
        let repos = parse_gitlab_projects(json).expect("parse");
        assert_eq!(repos.len(), 1);
        assert_eq!(repos[0].full_path, "acme/platform/api");
        assert_eq!(repos[0].name, "api");
        assert!(repos[0].archived);
        assert_eq!(repos[0].description, None);
        assert_eq!(
            repos[0].ssh_url.as_deref(),
            Some("git@git.example.com:acme/platform/api.git")
        );
    }

    #[test]
    fn parse_github_repos_maps_fields_and_rejects_bad_payload() {
        let json = r#"[{"full_name":"acme/web","name":"web","html_url":"https://github.com/acme/web",
            "ssh_url":"git@github.com:acme/web.git","clone_url":"https://github.com/acme/web.git",
            "default_branch":"main","archived":false}]"#;
        let repos = parse_github_repos(json).expect("parse");
        assert_eq!(
            repos[0].https_url.as_deref(),
            Some("https://github.com/acme/web.git")
        );
        assert!(parse_github_repos(r#"[{"name":"x"}]"#).is_err());
    }

    #[test]
    fn classify_failure_distinguishes_not_found_auth_and_other() {
        assert!(matches!(
            classify_failure(
                "gh",
                "github.com",
                "orgs/x/repos",
                "gh: Not Found (HTTP 404)",
                ""
            ),
            RemoteError::NotFound(_)
        ));
        assert!(matches!(
            classify_failure("glab", "h", "e", "glab: 404 Group Not Found (HTTP 404)", ""),
            RemoteError::NotFound(_)
        ));
        assert!(matches!(
            classify_failure(
                "gh",
                "github.com",
                "e",
                "To get started, run: gh auth login",
                ""
            ),
            RemoteError::NotAuthenticated { .. }
        ));
        assert!(matches!(
            classify_failure("glab", "h", "e", "glab: 401 Unauthorized (HTTP 401)", ""),
            RemoteError::NotAuthenticated { .. }
        ));
        assert!(matches!(
            classify_failure("gh", "h", "e", "connection reset", ""),
            RemoteError::Failed(_)
        ));
    }

    #[test]
    fn gitlab_listing_encodes_group_and_includes_subgroups() {
        let runner = FakeRunner::new(vec![ok("[]")]);
        let provider = GhGlabProvider::with_runner(&runner);
        let repos = provider
            .list_repos(&source(RemoteProvider::Gitlab, "/acme/platform/"))
            .expect("list");
        assert!(repos.is_empty());
        let calls = runner.calls.lock().expect("calls");
        let call = calls[0].join(" ");
        assert!(
            call.starts_with(
                "glab api --hostname git.example.com --paginate groups/acme%2Fplatform/projects?"
            ),
            "{call}"
        );
        assert!(call.contains("include_subgroups=true"));
    }

    #[test]
    fn github_listing_falls_back_to_user_endpoint_on_404() {
        let runner = FakeRunner::new(vec![
            fail("gh: Not Found (HTTP 404)"),
            ok(r#"[{"full_name":"me/tool","name":"tool"}]"#),
        ]);
        let provider = GhGlabProvider::with_runner(&runner);
        let repos = provider
            .list_repos(&source(RemoteProvider::Github, "me"))
            .expect("list");
        assert_eq!(repos[0].full_path, "me/tool");
        let calls = runner.calls.lock().expect("calls");
        assert!(calls[0].join(" ").contains("orgs/me/repos"));
        assert!(calls[1].join(" ").contains("users/me/repos"));
    }

    #[test]
    fn missing_cli_is_reported() {
        let runner = FakeRunner::new(vec![Err(std::io::Error::from(
            std::io::ErrorKind::NotFound,
        ))]);
        let provider = GhGlabProvider::with_runner(&runner);
        let err = provider
            .list_repos(&source(RemoteProvider::Gitlab, "acme"))
            .expect_err("missing");
        assert_eq!(err, RemoteError::CliMissing { cli: "glab".into() });
    }

    #[test]
    fn repo_exists_maps_404_to_false_and_other_errors_through() {
        let runner = FakeRunner::new(vec![
            fail("glab: 404 Project Not Found (HTTP 404)"),
            fail("glab: 500 boom"),
            ok("{}"),
        ]);
        let provider = GhGlabProvider::with_runner(&runner);
        let src = source(RemoteProvider::Gitlab, "acme");
        assert_eq!(provider.repo_exists(&src, "acme/gone"), Ok(false));
        assert!(matches!(
            provider.repo_exists(&src, "acme/x"),
            Err(RemoteError::Failed(_))
        ));
        assert_eq!(provider.repo_exists(&src, "acme/here"), Ok(true));
    }

    #[test]
    fn cli_git_protocol_reads_cli_config() {
        let runner = FakeRunner::new(vec![ok("ssh\n"), fail("unknown key")]);
        let provider = GhGlabProvider::with_runner(&runner);
        let src = source(RemoteProvider::Gitlab, "acme");
        assert_eq!(provider.cli_git_protocol(&src), Some(GitProtocol::Ssh));
        assert_eq!(provider.cli_git_protocol(&src), None);
    }
}
