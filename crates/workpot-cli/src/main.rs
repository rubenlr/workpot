mod git_display;
mod list_display;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use workpot_core::services::launch::launch_repo;
use workpot_core::services::repo_convert::{ConvertResult, ConvertTarget};
use workpot_core::services::repo_fuzzy::fuzzy_match;
use workpot_core::{AppContext, RepoRecord, WorkpotError};

use git_display::format_git_state;

#[derive(Parser)]
#[command(name = "workpot", about = "Local git repo workspace launcher", version)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print resolved config and database paths (creates defaults on first run).
    Paths,
    /// Full rescan of configured watch roots.
    Index,
    /// List repositories in priority order (Pinned > Dirty > Recent > Rest).
    List,
    #[command(subcommand)]
    Repo(RepoCommands),
    #[command(subcommand)]
    Roots(RootsCommands),
    #[command(subcommand)]
    Excludes(ExcludesCommands),
    /// Add, remove, or list tags on a repository.
    #[command(subcommand)]
    Tag(TagAction),
    /// Fuzzy-filter repositories by query and print in priority order (Pinned > Dirty > Recent > Rest).
    ///
    /// Uses the same fuzzy match algorithm and row format as `workpot list`.
    /// Empty query prints the full list (identical to `workpot list`).
    ///
    /// Note: `#tag` syntax is NOT parsed; the `#` character is treated as plain text in the query.
    /// Use `workpot tag list <repo>` for tag inspection.
    ///
    /// Exits 0 regardless of match count; no matches → silent empty stdout (grep-friendly).
    Search {
        /// Fuzzy query to filter repositories (empty → all repos).
        query: String,
    },
    /// Open a repository in the configured IDE (default: Cursor).
    Open {
        /// Repository name, path key, or canonical path.
        repo: String,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum CliConvertTarget {
    Bare,
    Normal,
}

#[derive(Subcommand)]
enum RepoCommands {
    /// Register a git worktree or bare repository path.
    Add { path: PathBuf },
    /// List registered repositories.
    List,
    /// Remove a registered repository.
    Remove { path: PathBuf },
    /// Migrate a repository between normal checkout and bare+worktree layouts.
    Convert {
        /// Absolute or relative path to the repository to convert.
        path: PathBuf,
        /// Target layout: bare or normal.
        #[arg(long)]
        to: CliConvertTarget,
        /// Print resolved target paths and preflight gate results without making any changes.
        #[arg(long)]
        dry_run: bool,
    },
}

#[derive(Subcommand)]
enum ExcludesCommands {
    /// List configured exclude globs.
    List,
    /// Remove an exclude glob from config.
    Remove { glob: String },
}

#[derive(Subcommand)]
enum TagAction {
    /// Add a tag to a repository.
    Add {
        /// Repository path or name
        repo: String,
        /// Tag to add (max 64 chars, no #)
        tag: String,
    },
    /// Remove a tag from a repository.
    Remove {
        /// Repository path or name
        repo: String,
        /// Tag to remove
        tag: String,
    },
    /// List tags for a repository.
    List {
        /// Repository path or name
        repo: String,
    },
}

#[derive(Subcommand)]
enum RootsCommands {
    /// Add a watch root and scan it immediately.
    Add { path: PathBuf },
    /// List configured watch roots.
    List,
    /// Remove a watch root and prune indexed repos under it by default.
    Remove {
        path: PathBuf,
        /// Keep indexed repos under this root (orphan scan rows until `workpot index` or `repo remove`).
        #[arg(long)]
        skip_prune: bool,
    },
}

/// IDE launch failure (exit 2), distinct from repo-not-found (exit 1 via anyhow).
#[derive(Debug)]
struct LaunchFailed(String);

impl fmt::Display for LaunchFailed {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "launch failed: {}", self.0)
    }
}

impl std::error::Error for LaunchFailed {}

fn main() -> ExitCode {
    let _ = env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
        .try_init();
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e)
            if matches!(
                e.downcast_ref::<WorkpotError>(),
                Some(WorkpotError::IndexCapExceeded { .. })
            ) =>
        {
            eprintln!("{e:#}");
            ExitCode::from(1)
        }
        Err(e) if e.downcast_ref::<LaunchFailed>().is_some() => {
            eprintln!("{e:#}");
            ExitCode::from(2)
        }
        Err(e) => {
            eprintln!("{e:#}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Paths => run_paths(),
        Commands::Index => run_index(),
        Commands::List => run_list(),
        Commands::Repo(sub) => run_repo(sub),
        Commands::Excludes(sub) => run_excludes(sub),
        Commands::Roots(sub) => run_roots(sub),
        Commands::Tag(action) => run_tag(action),
        Commands::Search { query } => run_search(&query),
        Commands::Open { repo } => run_open(&repo),
    }
}

fn run_paths() -> anyhow::Result<()> {
    let ctx = AppContext::open().context("failed to open workpot")?;
    println!("config: {}", ctx.config_path().display());
    println!("database: {}", ctx.database_path().display());
    let roots = ctx.roots_list();
    if roots.is_empty() {
        println!("watch_roots: (none)");
    } else {
        println!("watch_roots: (first-run config may seed ~/code and ~/dev when those dirs exist)");
        for root in roots {
            println!("  {}", root.display());
        }
    }
    Ok(())
}

fn run_index() -> anyhow::Result<()> {
    let ctx = AppContext::open().context("failed to open workpot")?;
    let summary = ctx.run_index()?;
    println!(
        "index: +{} -{} skipped {} / git: {} refreshed, {} errors",
        summary.added, summary.removed, summary.skipped, summary.git_refreshed, summary.git_errors
    );
    Ok(())
}

fn run_list() -> anyhow::Result<()> {
    let ctx = AppContext::open().context("failed to open workpot")?;
    let repos = ctx.list_repos().context("list failed")?;
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let ordered = list_display::flat_tray_ordered_with_icons(repos, ctx.config(), now_secs);
    for (repo, icon) in &ordered {
        println!("{}", list_display::format_list_row(repo, icon));
    }
    Ok(())
}

fn run_search(query: &str) -> anyhow::Result<()> {
    let ctx = AppContext::open().context("failed to open workpot")?;
    let mut repos = ctx.list_repos().context("list failed")?;
    // Trim query; empty (or whitespace-only) → retain all (D-05, RESEARCH pitfall 6).
    // fuzzy_match already handles empty query as "match all", but retaining explicitly
    // keeps the intent clear and avoids the filter allocation on the common no-query path.
    let trimmed = query.trim();
    if !trimmed.is_empty() {
        repos.retain(|r| fuzzy_match(trimmed, r));
    }
    let now_secs = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;
    let ordered = list_display::flat_tray_ordered_with_icons(repos, ctx.config(), now_secs);
    for (repo, icon) in &ordered {
        println!("{}", list_display::format_list_row(repo, icon));
    }
    Ok(())
}

fn run_repo(sub: RepoCommands) -> anyhow::Result<()> {
    match sub {
        RepoCommands::Add { path } => {
            let ctx = AppContext::open().context("failed to open workpot")?;
            let record = ctx.register_manual(&path).context("repo add failed")?;
            println!("registered: {}", record.path.display());
        }
        RepoCommands::List => {
            let ctx = AppContext::open().context("failed to open workpot")?;
            let repos = ctx.list_repos().context("repo list failed")?;
            for repo in repos {
                let display_name = repo.alias.as_deref().unwrap_or(&repo.name);
                println!(
                    "{}  {}  {}",
                    display_name,
                    repo.path.display(),
                    format_git_state(&repo)
                );
            }
        }
        RepoCommands::Remove { path } => {
            let mut ctx = AppContext::open().context("failed to open workpot")?;
            ctx.remove_repo(&path).context("repo remove failed")?;
            println!("removed: {}", path.display());
        }
        RepoCommands::Convert { path, to, dry_run } => {
            let ctx = AppContext::open().context("failed to open workpot")?;
            let core_target = match to {
                CliConvertTarget::Bare => ConvertTarget::Bare,
                CliConvertTarget::Normal => ConvertTarget::Normal,
            };
            match ctx
                .convert_repo(&path, core_target, dry_run)
                .map_err(map_convert_error)?
            {
                ConvertResult::Converted { from, to } => {
                    println!("converted: {} -> {}", from.display(), to.display());
                }
                ConvertResult::DryRun {
                    preflight,
                    resolved_paths,
                } => {
                    for (label, resolved) in resolved_paths {
                        println!("  {}: {}", label, resolved.display());
                    }
                    println!("preflight: {preflight:?}");
                }
            }
        }
    }
    Ok(())
}

fn run_excludes(sub: ExcludesCommands) -> anyhow::Result<()> {
    let mut ctx = AppContext::open().context("failed to open workpot")?;
    match sub {
        ExcludesCommands::List => {
            for glob in ctx.excludes_list() {
                println!("{glob}");
            }
        }
        ExcludesCommands::Remove { glob } => {
            ctx.excludes_remove(&glob)
                .context("excludes remove failed")?;
            println!("removed exclude: {glob}");
        }
    }
    Ok(())
}

fn run_roots(sub: RootsCommands) -> anyhow::Result<()> {
    let mut ctx = AppContext::open().context("failed to open workpot")?;
    match sub {
        RootsCommands::Add { path } => {
            ctx.roots_add(&path).map_err(map_roots_error)?;
            println!("watch root added: {}", path.display());
        }
        RootsCommands::List => {
            for root in ctx.roots_list() {
                println!("{}", root.display());
            }
        }
        RootsCommands::Remove { path, skip_prune } => {
            ctx.roots_remove(&path, skip_prune)
                .map_err(map_roots_error)?;
            println!("watch root removed: {}", path.display());
        }
    }
    Ok(())
}

fn run_tag(action: TagAction) -> anyhow::Result<()> {
    let ctx = AppContext::open().context("failed to open workpot")?;
    match action {
        TagAction::Add { repo, tag } => {
            let path_key = resolve_repo_identifier(&ctx, &repo)?;
            ctx.add_tag(&path_key, &tag).map_err(map_tag_error)?;
        }
        TagAction::Remove { repo, tag } => {
            let path_key = resolve_repo_identifier(&ctx, &repo)?;
            ctx.remove_tag(&path_key, &tag).map_err(map_tag_error)?;
        }
        TagAction::List { repo } => {
            let path_key = resolve_repo_identifier(&ctx, &repo)?;
            let tags = ctx.list_tags_for_repo(&path_key)?;
            if tags.is_empty() {
                println!("(no tags)");
            } else {
                for tag in tags {
                    println!("{tag}");
                }
            }
        }
    }
    Ok(())
}

fn run_open(identifier: &str) -> anyhow::Result<()> {
    let ctx = AppContext::open().context("failed to open workpot")?;
    // resolve_repo_identifier handles D-09 (ambiguous) and D-11 (not found) exits via Err
    let path_key = resolve_repo_identifier(&ctx, identifier)?;
    // D-10: print full canonical path before launch
    println!("opening: {path_key}");
    launch_repo(&ctx, &path_key).map_err(LaunchFailed)?;
    Ok(())
}

/// Resolve CLI `repo` argument to SQLite `repos.path` (exact key, canonical path, or unique name).
fn resolve_repo_identifier(ctx: &AppContext, identifier: &str) -> anyhow::Result<String> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        return Err(anyhow::anyhow!("repo not found: (empty identifier)"));
    }
    let repos = ctx.list_repos().context("failed to list repos")?;

    if let Some(path_key) = match_repo_path_key(&repos, identifier) {
        return Ok(path_key);
    }

    let path = Path::new(identifier);
    if (path.is_absolute() || identifier.contains(std::path::MAIN_SEPARATOR))
        && let Ok(canon) = path.canonicalize()
        && let Some(path_key) = repos
            .iter()
            .find(|r| r.path == canon)
            .map(|r| r.path.display().to_string())
    {
        return Ok(path_key);
    }

    let alias_matches: Vec<&RepoRecord> = repos
        .iter()
        .filter(|r| r.alias.as_deref() == Some(identifier))
        .collect();
    match alias_matches.len() {
        0 => {}
        1 => return Ok(alias_matches[0].path.display().to_string()),
        _ => {
            let mut msg = format!("error: ambiguous repo alias '{identifier}'; matches:\n");
            for (i, r) in alias_matches.iter().enumerate() {
                msg.push_str(&format!("{}. {}\n", i + 1, r.path.display()));
            }
            msg.push_str("use the full path from 'workpot list'");
            return Err(anyhow::anyhow!("{msg}"));
        }
    }

    let matches: Vec<&RepoRecord> = repos.iter().filter(|r| r.name == identifier).collect();
    match matches.len() {
        0 => Err(anyhow::anyhow!("repo not found: {identifier}")),
        1 => Ok(matches[0].path.display().to_string()),
        _ => {
            let mut msg = format!("error: ambiguous repo name '{identifier}'; matches:\n");
            for (i, r) in matches.iter().enumerate() {
                msg.push_str(&format!("{}. {}\n", i + 1, r.path.display()));
            }
            msg.push_str("use the full path from 'workpot list'");
            Err(anyhow::anyhow!("{msg}"))
        }
    }
}

fn match_repo_path_key(repos: &[RepoRecord], identifier: &str) -> Option<String> {
    let id = OsStr::new(identifier);
    repos
        .iter()
        .find(|r| r.path.as_os_str() == id)
        .map(|r| r.path.display().to_string())
}

fn map_tag_error(err: WorkpotError) -> anyhow::Error {
    match err {
        WorkpotError::InvalidInput(ref msg) => {
            let cli_msg = if msg.contains("must not contain '#'") {
                "tag may not contain '#'"
            } else if msg.contains("exceeds 64 characters") {
                "tag too long (max 64 chars)"
            } else if msg.contains("must not be empty") {
                "tag cannot be empty"
            } else {
                return err.into();
            };
            anyhow::anyhow!(cli_msg)
        }
        other => other.into(),
    }
}

fn map_convert_error(err: WorkpotError) -> anyhow::Error {
    match err {
        WorkpotError::ConversionPreflight(msg) => anyhow::anyhow!("preflight failed: {msg}"),
        WorkpotError::ConversionFailed(msg) => anyhow::anyhow!("conversion failed: {msg}"),
        other => other.into(),
    }
}

fn map_roots_error(err: WorkpotError) -> anyhow::Error {
    match err {
        WorkpotError::LimitsExceeded(msg) | WorkpotError::WatchRootNotFound(msg) => {
            anyhow::anyhow!(msg)
        }
        other => other.into(),
    }
}

#[cfg(test)]
mod cli_parse_tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn parses_top_level_subcommands() {
        assert!(matches!(
            Cli::try_parse_from(["workpot", "paths"]).unwrap().command,
            Commands::Paths
        ));
        assert!(matches!(
            Cli::try_parse_from(["workpot", "index"]).unwrap().command,
            Commands::Index
        ));
        assert!(matches!(
            Cli::try_parse_from(["workpot", "list"]).unwrap().command,
            Commands::List
        ));
        assert!(matches!(
            Cli::try_parse_from(["workpot", "search", "foo"])
                .unwrap()
                .command,
            Commands::Search { .. }
        ));
        assert!(matches!(
            Cli::try_parse_from(["workpot", "open", "myrepo"])
                .unwrap()
                .command,
            Commands::Open { .. }
        ));
    }

    #[test]
    fn parses_repo_roots_and_tag_subcommands() {
        assert!(matches!(
            Cli::try_parse_from(["workpot", "repo", "add", "/tmp/r"])
                .unwrap()
                .command,
            Commands::Repo(RepoCommands::Add { .. })
        ));
        assert!(matches!(
            Cli::try_parse_from(["workpot", "repo", "remove", "/tmp/r"])
                .unwrap()
                .command,
            Commands::Repo(RepoCommands::Remove { .. })
        ));
        assert!(matches!(
            Cli::try_parse_from(["workpot", "roots", "remove", "/tmp/root", "--skip-prune"])
                .unwrap()
                .command,
            Commands::Roots(RootsCommands::Remove {
                skip_prune: true,
                ..
            })
        ));
        assert!(matches!(
            Cli::try_parse_from(["workpot", "tag", "add", "repo", "work"])
                .unwrap()
                .command,
            Commands::Tag(TagAction::Add { .. })
        ));
        assert!(matches!(
            Cli::try_parse_from(["workpot", "tag", "list", "repo"])
                .unwrap()
                .command,
            Commands::Tag(TagAction::List { .. })
        ));
    }

    #[test]
    fn parses_repo_convert_bare() {
        assert!(matches!(
            Cli::try_parse_from(["workpot", "repo", "convert", "/tmp/r", "--to", "bare"])
                .unwrap()
                .command,
            Commands::Repo(RepoCommands::Convert { dry_run: false, .. })
        ));
    }

    #[test]
    fn parses_repo_convert_dry_run() {
        assert!(matches!(
            Cli::try_parse_from([
                "workpot",
                "repo",
                "convert",
                "/tmp/r",
                "--to",
                "normal",
                "--dry-run"
            ])
            .unwrap()
            .command,
            Commands::Repo(RepoCommands::Convert { dry_run: true, .. })
        ));
    }
}
