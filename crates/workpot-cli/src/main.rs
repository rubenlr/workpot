mod git_display;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use std::process::ExitCode;
use workpot_core::{AppContext, WorkpotError};

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
        Err(e) if matches!(
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
        Commands::Paths => {
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
        }
        Commands::Index => {
            let ctx = AppContext::open().context("failed to open workpot")?;
            let summary = ctx.run_index()?;
            println!(
                "index: +{} -{} skipped {} / git: {} refreshed, {} errors",
                summary.added, summary.removed, summary.skipped,
                summary.git_refreshed, summary.git_errors
            );
        }
        Commands::Repo(sub) => match sub {
            RepoCommands::Add { path } => {
                let ctx = AppContext::open().context("failed to open workpot")?;
                let record = ctx.register_manual(&path).context("repo add failed")?;
                println!("registered: {}", record.path.display());
            }
            RepoCommands::List => {
                let ctx = AppContext::open().context("failed to open workpot")?;
                let repos = ctx.list_repos().context("repo list failed")?;
                for repo in repos {
                    println!("{}  {}  {}", repo.name, repo.path.display(), format_git_state(&repo));
                }
            }
            RepoCommands::Remove { path } => {
                let mut ctx = AppContext::open().context("failed to open workpot")?;
                ctx.remove_repo(&path).context("repo remove failed")?;
                println!("removed: {}", path.display());
            }
        },
        Commands::Excludes(sub) => {
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
        },
        Commands::Roots(sub) => {
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
        }
    }
    Ok(())
}

fn map_roots_error(err: WorkpotError) -> anyhow::Error {
    match err {
        WorkpotError::LimitsExceeded(msg) | WorkpotError::WatchRootNotFound(msg) => {
            anyhow::anyhow!(msg)
        }
        other => other.into(),
    }
}
