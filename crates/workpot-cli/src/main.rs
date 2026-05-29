use anyhow::Context;
use clap::{Parser, Subcommand};
use std::path::PathBuf;
use workpot_core::AppContext;

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

fn main() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Paths => {
            let ctx = AppContext::open().context("failed to open workpot")?;
            println!("config: {}", ctx.config_path().display());
            println!("database: {}", ctx.database_path().display());
        }
        Commands::Index => {
            let ctx = AppContext::open().context("failed to open workpot")?;
            let summary = ctx.run_index().context("index failed")?;
            println!(
                "index: +{} -{} skipped {}",
                summary.added, summary.removed, summary.skipped
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
                    println!("{}  {}", repo.name, repo.path.display());
                }
            }
            RepoCommands::Remove { path } => {
                let ctx = AppContext::open().context("failed to open workpot")?;
                ctx.remove_repo(&path).context("repo remove failed")?;
                println!("removed: {}", path.display());
            }
        },
    }
    Ok(())
}
