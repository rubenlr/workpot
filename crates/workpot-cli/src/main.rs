mod git_display;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::{Path, PathBuf};
use std::process::{ExitCode, exit};
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
    #[command(subcommand)]
    Repo(RepoCommands),
    #[command(subcommand)]
    Roots(RootsCommands),
    #[command(subcommand)]
    Excludes(ExcludesCommands),
    /// Add, remove, or list tags on a repository.
    #[command(subcommand)]
    Tag(TagAction),
}

#[derive(Subcommand)]
enum RepoCommands {
    /// Register a git worktree or bare repository path.
    Add { path: PathBuf },
    /// List registered repositories.
    List,
    /// Remove a registered repository.
    Remove { path: PathBuf },
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
        Commands::Repo(sub) => run_repo(sub),
        Commands::Excludes(sub) => run_excludes(sub),
        Commands::Roots(sub) => run_roots(sub),
        Commands::Tag(action) => run_tag(action),
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
        println!(
            "watch_roots: (first-run config may seed ~/code and ~/dev when those dirs exist)"
        );
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
        summary.added,
        summary.removed,
        summary.skipped,
        summary.git_refreshed,
        summary.git_errors
    );
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
                println!(
                    "{}  {}  {}",
                    repo.name,
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
            validate_tag_for_add(&tag)?;
            let path_key = resolve_repo_identifier(&ctx, &repo)?;
            ctx.add_tag(&path_key, tag.trim())?;
        }
        TagAction::Remove { repo, tag } => {
            let path_key = resolve_repo_identifier(&ctx, &repo)?;
            ctx.remove_tag(&path_key, &tag)?;
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

fn validate_tag_for_add(tag: &str) -> anyhow::Result<()> {
    let trimmed = tag.trim();
    if trimmed.is_empty() {
        eprintln!("tag cannot be empty");
        exit(1);
    }
    if trimmed.chars().count() > 64 {
        eprintln!("tag too long (max 64 chars)");
        exit(1);
    }
    if trimmed.contains('#') {
        eprintln!("tag may not contain '#'");
        exit(1);
    }
    Ok(())
}

/// Resolve CLI `repo` argument to SQLite `repos.path` (exact key, canonical path, or unique name).
fn resolve_repo_identifier(ctx: &AppContext, identifier: &str) -> anyhow::Result<String> {
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

    let matches: Vec<&RepoRecord> = repos.iter().filter(|r| r.name == identifier).collect();
    match matches.len() {
        0 => Err(anyhow::anyhow!("repo not found: {identifier}")),
        1 => Ok(matches[0].path.display().to_string()),
        _ => Err(anyhow::anyhow!(
            "ambiguous repo name '{identifier}'; use the absolute path from `workpot repo list`"
        )),
    }
}

fn match_repo_path_key(repos: &[RepoRecord], identifier: &str) -> Option<String> {
    repos
        .iter()
        .find(|r| r.path.display().to_string() == identifier)
        .map(|r| r.path.display().to_string())
}

fn map_roots_error(err: WorkpotError) -> anyhow::Error {
    match err {
        WorkpotError::LimitsExceeded(msg) | WorkpotError::WatchRootNotFound(msg) => {
            anyhow::anyhow!(msg)
        }
        other => other.into(),
    }
}
