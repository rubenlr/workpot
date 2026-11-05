mod git_display;
mod list_display;

use anyhow::Context;
use clap::{Parser, Subcommand};
use std::ffi::OsStr;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::ExitCode;
use workpot_core::domain::config::{ClonePolicy, GitProtocol, RemoteProvider, RemoteSource};
use workpot_core::infra::paths;
use workpot_core::infra::store;
use workpot_core::services::launch::launch_repo;
use workpot_core::services::remote_catalog::{RemoteRepoRecord, RemoteSourceStatus};
use workpot_core::services::remotes::RemotePatch;
use workpot_core::services::repo_convert::{ConvertResult, ConvertTarget};
use workpot_core::services::repo_fuzzy::fuzzy_match;
use workpot_core::{
    AppContext, RepoRecord, WorkpotError, annotate_config_comments, init_config_file,
};

use git_display::format_git_state;

#[derive(Parser)]
#[command(
    name = "workpot",
    about = "Local git repo workspace launcher",
    long_about = "Local git repo workspace launcher.\n\nPrimary workflow: `list` / `search` to find repos, `open` to launch in Cursor.",
    version
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Print resolved config and database paths (creates defaults on first run).
    Paths,
    /// Sync catalog from remote sources and watch roots.
    ///
    /// `workpot sync` runs the full orchestrator (remotes, then local). `workpot sync local` syncs
    /// the local catalog only; `workpot sync remote` indexes remote sources only.
    /// To scan a single new root immediately, use `workpot roots add` instead.
    Sync {
        #[command(subcommand)]
        command: Option<SyncCommands>,
    },
    /// List repositories in priority order (Pinned > Dirty > Recent > Rest).
    List,
    /// Register, remove, convert, or inspect repositories in the catalog.
    #[command(subcommand)]
    Repo(RepoCommands),
    /// Add, list, or remove filesystem watch roots for automatic discovery.
    #[command(subcommand)]
    Roots(RootsCommands),
    /// Index GitHub orgs / GitLab groups into the catalog (uses the `gh` / `glab` CLIs).
    #[command(subcommand)]
    Remotes(RemotesCommands),
    /// Clone an indexed remote repo (works under every clone policy).
    ///
    /// Accepts the planned clone path, `source:full/path`, the full remote path, or a unique repo name.
    Clone {
        /// Remote repo identifier.
        repo: String,
    },
    /// List or remove path-exclusion globs applied during local catalog sync scans.
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
    ///
    /// Accepts repo name, alias, path key, or canonical path. Ambiguous names
    /// error with numbered matches. Exits 1 if not found, 2 if launch fails.
    Open {
        /// Repository name, alias, path key, or canonical path.
        repo: String,
    },
    /// Initialize or annotate the configuration file.
    Settings {
        #[command(flatten)]
        args: SettingsArgs,
    },
    /// Delete or inspect local SQLite database files.
    #[command(subcommand)]
    Db(DbCommands),
}

#[derive(Subcommand)]
enum DbCommands {
    /// Delete the local SQLite database and WAL/SHM sidecars.
    ///
    /// Idempotent: succeeds if the database is already absent.
    /// Quit the Workpot tray app before reset if the database may be locked.
    Reset,
}

#[derive(Parser)]
struct SettingsArgs {
    #[command(subcommand)]
    command: Option<SettingsCommands>,
    /// Add missing documentation comments to each setting in config.toml.
    #[arg(long)]
    add_comments: bool,
}

#[derive(Subcommand)]
enum SyncCommands {
    /// Sync local catalog only (watch-root rescan + git refresh).
    Local,
    /// Index remote sources only (list, auto-clone, prune per source policy).
    Remote,
}

#[derive(Subcommand)]
enum RemotesCommands {
    /// Add a remote source: every repo under a GitHub org/user or GitLab group (subgroups included).
    Add(RemoteAddArgs),
    /// List configured remote sources with index status.
    List,
    /// Remove a source and its index (local clones stay as ordinary repos).
    Remove {
        /// Source name.
        name: String,
    },
    /// Change settings of a source.
    Set(RemoteSetArgs),
}

#[derive(clap::Args)]
struct RemoteAddArgs {
    /// GitHub org/user, or GitLab group path (e.g. `acme/platform`).
    path: String,
    /// Hosting provider.
    #[arg(long, value_enum)]
    provider: CliProvider,
    /// Directory clones are placed under (`~` allowed).
    #[arg(long)]
    clone_root: PathBuf,
    /// Source label (default: last segment of PATH).
    #[arg(long)]
    name: Option<String>,
    /// Host for GitHub Enterprise / self-hosted GitLab (default: provider's public host).
    #[arg(long)]
    host: Option<String>,
    /// What happens automatically to repos that are not cloned.
    #[arg(long, value_enum, default_value = "manual")]
    policy: CliClonePolicy,
    /// Safe-only delete of clean, fully pushed clones whose upstream repo was removed.
    #[arg(long)]
    prune_removed: bool,
    /// Hide archived repos.
    #[arg(long)]
    exclude_archived: bool,
    /// Override the CLI's configured `git_protocol` for this source.
    #[arg(long, value_enum)]
    protocol: Option<CliProtocol>,
    /// Do not index right after adding.
    #[arg(long)]
    no_sync: bool,
}

#[derive(clap::Args)]
struct RemoteSetArgs {
    /// Source name.
    name: String,
    #[arg(long, value_enum)]
    policy: Option<CliClonePolicy>,
    #[arg(long)]
    prune_removed: Option<bool>,
    #[arg(long)]
    include_archived: Option<bool>,
    #[arg(long)]
    clone_root: Option<PathBuf>,
    /// `ssh`, `https`, or `default` to follow the CLI's `git_protocol`.
    #[arg(long)]
    protocol: Option<String>,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum CliProvider {
    Github,
    Gitlab,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum CliClonePolicy {
    Manual,
    OnOpen,
    Auto,
}

#[derive(Clone, Copy, Debug, clap::ValueEnum)]
enum CliProtocol {
    Ssh,
    Https,
}

impl From<CliProvider> for RemoteProvider {
    fn from(p: CliProvider) -> Self {
        match p {
            CliProvider::Github => Self::Github,
            CliProvider::Gitlab => Self::Gitlab,
        }
    }
}

impl From<CliClonePolicy> for ClonePolicy {
    fn from(p: CliClonePolicy) -> Self {
        match p {
            CliClonePolicy::Manual => Self::Manual,
            CliClonePolicy::OnOpen => Self::OnOpen,
            CliClonePolicy::Auto => Self::Auto,
        }
    }
}

impl From<CliProtocol> for GitProtocol {
    fn from(p: CliProtocol) -> Self {
        match p {
            CliProtocol::Ssh => Self::Ssh,
            CliProtocol::Https => Self::Https,
        }
    }
}

#[derive(Subcommand)]
enum SettingsCommands {
    /// Write a documented default config.toml.
    Init {
        /// Overwrite existing config.toml.
        #[arg(long)]
        force: bool,
    },
}

#[derive(Clone, Debug, clap::ValueEnum)]
enum CliConvertTarget {
    Bare,
    Local,
}

#[derive(Subcommand)]
enum RepoCommands {
    /// Register a git worktree or bare repository path.
    ///
    /// Manual registration bypasses scan exclude globs.
    Add { path: PathBuf },
    /// Flat catalog dump with git-state columns (admin/debug).
    ///
    /// Unlike `workpot list`, does not apply tray priority order or icons.
    List,
    /// Remove a registered repository.
    Remove { path: PathBuf },
    /// Migrate a repository between local checkout and bare+worktree layouts.
    Convert {
        /// Absolute or relative path to the repository to convert.
        path: PathBuf,
        /// Target layout: bare or local.
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
    ///
    /// Globs are also added automatically by `workpot repo remove`.
    List,
    /// Remove an exclude glob from config.
    ///
    /// Re-index may re-discover repos under matching paths unless the watch root is also removed.
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
    /// Remove a watch root and prune cataloged repos under it by default.
    Remove {
        path: PathBuf,
        /// Keep cataloged repos under this root (orphan scan rows until `workpot sync` or `repo remove`).
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
    // Fails only when a logger is already installed; the logger itself is not usable to say so.
    if let Err(e) =
        env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
            .try_init()
    {
        eprintln!("warning: logger not initialised: {e}");
    }
    match run() {
        Ok(()) => ExitCode::SUCCESS,
        Err(e)
            if matches!(
                e.downcast_ref::<WorkpotError>(),
                Some(WorkpotError::LocalCatalogSyncCapExceeded { .. })
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

fn open_context() -> anyhow::Result<AppContext> {
    AppContext::open().context("failed to open workpot")
}

fn unix_now_secs() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_secs() as i64)
        .unwrap_or(0)
}

fn print_ordered_repos(ctx: &AppContext, repos: Vec<RepoRecord>) -> anyhow::Result<()> {
    let now_secs = unix_now_secs();
    let ordered = list_display::flat_tray_ordered_with_icons(repos, &*ctx.config()?, now_secs);
    for (repo, icon) in &ordered {
        println!("{}", list_display::format_list_row(repo, icon));
    }
    Ok(())
}

fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Paths => run_paths(),
        Commands::Sync { command } => run_sync_cmd(command),
        Commands::List => run_list(),
        Commands::Repo(sub) => run_repo(sub),
        Commands::Excludes(sub) => run_excludes(sub),
        Commands::Roots(sub) => run_roots(sub),
        Commands::Remotes(sub) => run_remotes(sub),
        Commands::Clone { repo } => run_clone(&repo),
        Commands::Tag(action) => run_tag(action),
        Commands::Search { query } => run_search(&query),
        Commands::Open { repo } => run_open(&repo),
        Commands::Settings { args } => run_settings(args),
        Commands::Db(sub) => run_db(sub),
    }
}

fn run_db(sub: DbCommands) -> anyhow::Result<()> {
    match sub {
        DbCommands::Reset => run_db_reset(),
    }
}

fn run_db_reset() -> anyhow::Result<()> {
    let db_path = paths::database_file().context("failed to resolve database path")?;
    let report = store::reset_database(&db_path).with_context(|| {
        format!(
            "failed to delete database at {} — quit the Workpot tray app if the DB is locked",
            db_path.display()
        )
    })?;
    if report.deleted.is_empty() {
        println!("already absent: {}", report.database.display());
    } else {
        for path in &report.deleted {
            println!("deleted: {}", path.display());
        }
    }
    Ok(())
}

fn run_settings(args: SettingsArgs) -> anyhow::Result<()> {
    if args.add_comments && args.command.is_some() {
        anyhow::bail!("settings: use either a subcommand or --add-comments, not both");
    }
    match args.command {
        Some(SettingsCommands::Init { force }) => run_settings_init(force),
        None if args.add_comments => run_settings_add_comments(),
        None => Err(anyhow::anyhow!(
            "settings requires a subcommand or --add-comments (try `workpot settings init` or `workpot settings --help`)"
        )),
    }
}

fn run_settings_init(force: bool) -> anyhow::Result<()> {
    let config_path = paths::config_file().context("failed to resolve config path")?;
    let home = directories::BaseDirs::new()
        .map(|b| b.home_dir().to_path_buf())
        .ok_or_else(|| anyhow::anyhow!("could not resolve home directory"))?;
    init_config_file(&config_path, &home, force).map_err(|e| match e {
        WorkpotError::Config(msg) => anyhow::anyhow!(msg),
        other => other.into(),
    })?;
    println!("wrote {}", config_path.display());
    Ok(())
}

fn run_settings_add_comments() -> anyhow::Result<()> {
    let config_path = paths::config_file().context("failed to resolve config path")?;
    let added = annotate_config_comments(&config_path)?;
    if added == 0 {
        println!("no missing comments");
    } else {
        println!(
            "added {added} comment block(s) to {}",
            config_path.display()
        );
    }
    Ok(())
}

fn run_paths() -> anyhow::Result<()> {
    let ctx = open_context()?;
    println!("config: {}", ctx.config_path().display());
    println!("database: {}", ctx.database_path().display());
    let roots = ctx.roots_list()?;
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

fn print_remote_summary(summary: &workpot_core::services::remote_catalog_sync::RemoteSyncSummary) {
    if summary.sources == 0 {
        return;
    }
    println!(
        "remote: {} indexed, +{} new, {} gone, {} cloned, {} pruned, {} errors{}",
        summary.indexed,
        summary.added,
        summary.gone,
        summary.cloned,
        summary.pruned,
        summary.errors,
        if summary.skipped_ttl > 0 {
            format!(
                " ({} source(s) skipped: synced recently)",
                summary.skipped_ttl
            )
        } else {
            String::new()
        }
    );
    for err in &summary.source_errors {
        eprintln!("warning: remote {}: {}", err.source, err.message);
    }
}

fn run_sync_cmd(scope: Option<SyncCommands>) -> anyhow::Result<()> {
    let ctx = open_context()?;
    let (label, summary) = match scope {
        None => {
            let full = ctx.run_sync_full(true)?;
            print_remote_summary(&full.remote);
            ("sync", full.local)
        }
        Some(SyncCommands::Local) => ("sync local", ctx.run_local_catalog_sync()?),
        Some(SyncCommands::Remote) => {
            let remote = ctx.run_remote_catalog_sync(true)?;
            if remote.sources == 0 {
                println!("sync remote: no remote sources configured (see `workpot remotes add`)");
            }
            print_remote_summary(&remote);
            return Ok(());
        }
    };
    println!(
        "{label}: +{} -{} skipped {} / git: {} refreshed, {} errors",
        summary.added, summary.removed, summary.skipped, summary.git_refreshed, summary.git_errors
    );
    Ok(())
}

fn run_remotes(sub: RemotesCommands) -> anyhow::Result<()> {
    let ctx = open_context()?;
    match sub {
        RemotesCommands::Add(args) => run_remotes_add(&ctx, args),
        RemotesCommands::List => run_remotes_list(&ctx),
        RemotesCommands::Remove { name } => run_remotes_remove(&ctx, &name),
        RemotesCommands::Set(args) => run_remotes_set(&ctx, args),
    }
}

fn run_remotes_add(ctx: &AppContext, args: RemoteAddArgs) -> anyhow::Result<()> {
    let trimmed = args.path.trim().trim_matches('/').to_string();
    let name = args
        .name
        .unwrap_or_else(|| trimmed.rsplit('/').next().unwrap_or(&trimmed).to_string());
    let source = RemoteSource {
        name: name.clone(),
        provider: args.provider.into(),
        host: args.host,
        path: trimmed,
        clone_root: args.clone_root,
        clone_policy: args.policy.into(),
        prune_removed: args.prune_removed,
        include_archived: !args.exclude_archived,
        protocol: args.protocol.map(Into::into),
    };
    ctx.remotes_add(source).map_err(map_remote_error)?;
    println!("added remote source {name}");
    if !args.no_sync {
        let summary = ctx.run_remote_catalog_sync(true)?;
        print_remote_summary(&summary);
    }
    Ok(())
}

fn run_remotes_list(ctx: &AppContext) -> anyhow::Result<()> {
    let statuses = ctx.remote_source_statuses()?;
    if statuses.is_empty() {
        println!("no remote sources configured (see `workpot remotes add`)");
    }
    let now = unix_now_secs();
    for status in &statuses {
        println!("{}", format_remote_source_line(status));
        println!("{}", format_last_sync_line(status, now));
    }
    Ok(())
}

fn format_remote_source_line(s: &RemoteSourceStatus) -> String {
    let removed_upstream = if s.gone_count > 0 {
        format!(" removed-upstream={}", s.gone_count)
    } else {
        String::new()
    };
    format!(
        "{} {}:{}/{} -> {} policy={} prune={} repos={} cloned={}{}",
        s.name,
        s.provider.as_str(),
        s.host,
        s.path,
        s.clone_root.display(),
        s.clone_policy.as_str(),
        if s.prune_removed { "on" } else { "off" },
        s.repo_count,
        s.cloned_count,
        removed_upstream,
    )
}

fn format_last_sync_line(s: &RemoteSourceStatus, now: i64) -> String {
    match (s.last_run_status.as_deref(), s.last_run_at) {
        (Some(status), Some(at)) => {
            let ago = format_ago(now.saturating_sub(at));
            match s.last_run_message.as_deref() {
                Some(msg) if status == "error" => format!("  last sync: error {ago} ago: {msg}"),
                _ => format!("  last sync: {status} {ago} ago"),
            }
        }
        _ => "  last sync: never (run `workpot sync remote`)".to_string(),
    }
}

fn run_remotes_remove(ctx: &AppContext, name: &str) -> anyhow::Result<()> {
    ctx.remotes_remove(name).map_err(map_remote_error)?;
    println!("removed remote source {name}");
    Ok(())
}

/// `None` = leave unchanged, `Some(None)` = follow the CLI's `git_protocol`, `Some(Some(p))` = pin `p`.
fn parse_protocol_override(raw: Option<&str>) -> anyhow::Result<Option<Option<GitProtocol>>> {
    match raw {
        None => Ok(None),
        Some("default") => Ok(Some(None)),
        Some(raw) => GitProtocol::parse(raw)
            .map(|protocol| Some(Some(protocol)))
            .ok_or_else(|| anyhow::anyhow!("invalid protocol {raw:?}: use ssh, https, or default")),
    }
}

fn run_remotes_set(ctx: &AppContext, args: RemoteSetArgs) -> anyhow::Result<()> {
    let protocol = parse_protocol_override(args.protocol.as_deref())?;
    ctx.remotes_set(
        &args.name,
        RemotePatch {
            clone_policy: args.policy.map(Into::into),
            prune_removed: args.prune_removed,
            include_archived: args.include_archived,
            clone_root: args.clone_root,
            protocol,
        },
    )
    .map_err(map_remote_error)?;
    println!("updated remote source {}", args.name);
    Ok(())
}

const SECS_PER_MINUTE: i64 = 60;
const SECS_PER_HOUR: i64 = 60 * SECS_PER_MINUTE;
const SECS_PER_DAY: i64 = 24 * SECS_PER_HOUR;

fn format_ago(secs: i64) -> String {
    match secs {
        s if s < SECS_PER_MINUTE => format!("{s}s"),
        s if s < SECS_PER_HOUR => format!("{}m", s / SECS_PER_MINUTE),
        s if s < SECS_PER_DAY => format!("{}h", s / SECS_PER_HOUR),
        s => format!("{}d", s / SECS_PER_DAY),
    }
}

fn map_remote_error(err: WorkpotError) -> anyhow::Error {
    match err {
        WorkpotError::Config(msg) | WorkpotError::InvalidInput(msg) => {
            anyhow::anyhow!("error: {msg}")
        }
        WorkpotError::NotFound(what) => anyhow::anyhow!("error: not found: {what}"),
        other => anyhow::Error::new(other),
    }
}

fn run_clone(identifier: &str) -> anyhow::Result<()> {
    let ctx = open_context()?;
    let outcome = ctx.clone_remote(identifier).map_err(|e| match e {
        WorkpotError::NotFound(_) => anyhow::anyhow!(
            "remote repo not found: {identifier} (run `workpot sync remote` to refresh the index)"
        ),
        other => anyhow::Error::new(other),
    })?;
    if outcome.already_cloned {
        println!("already cloned: {}", outcome.local_path.display());
    } else {
        println!("cloned: {}", outcome.local_path.display());
    }
    Ok(())
}

/// Row-compatible stand-in so remote-only repos reuse the local fuzzy matcher and row format.
fn remote_as_repo_record(record: &RemoteRepoRecord) -> RepoRecord {
    RepoRecord {
        path: record.clone_path.clone(),
        name: record.name.clone(),
        registered_at: record.first_seen_at,
        source: "remote".to_string(),
        git_common_dir: String::new(),
        branch: None,
        is_dirty: None,
        ahead: None,
        behind: None,
        git_refreshed_at: None,
        git_state_error: None,
        last_opened_at: None,
        pinned: false,
        pin_order: None,
        notes: None,
        tags: vec![record.full_path.clone()],
        alias: None,
        convert_block_reason: None,
    }
}

fn print_remote_only(ctx: &AppContext, query: &str) -> anyhow::Result<()> {
    for record in ctx.list_remote_only()? {
        let stand_in = remote_as_repo_record(&record);
        if !query.is_empty() && !fuzzy_match(query, &stand_in) {
            continue;
        }
        println!(
            "☁️ {} {} (not cloned)",
            list_display::shorten_parent_dir(&record.clone_path),
            record.name
        );
    }
    Ok(())
}

fn run_list() -> anyhow::Result<()> {
    let ctx = open_context()?;
    let repos = ctx.list_repos().context("list failed")?;
    print_ordered_repos(&ctx, repos)?;
    print_remote_only(&ctx, "")
}

fn run_search(query: &str) -> anyhow::Result<()> {
    let ctx = open_context()?;
    let mut repos = ctx.list_repos().context("list failed")?;
    // An empty (or whitespace-only) query keeps every repo; skipping the filter also avoids
    // an allocation on the common no-query path.
    let trimmed = query.trim();
    if !trimmed.is_empty() {
        repos.retain(|r| fuzzy_match(trimmed, r));
    }
    print_ordered_repos(&ctx, repos)?;
    print_remote_only(&ctx, trimmed)
}

fn run_repo(sub: RepoCommands) -> anyhow::Result<()> {
    match sub {
        RepoCommands::Add { path } => {
            let ctx = open_context()?;
            let record = ctx.register_manual(&path).context("repo add failed")?;
            println!("registered: {}", record.path.display());
        }
        RepoCommands::List => run_repo_list()?,
        RepoCommands::Remove { path } => {
            let ctx = open_context()?;
            ctx.remove_repo(&path).context("repo remove failed")?;
            println!("removed: {}", path.display());
        }
        RepoCommands::Convert { path, to, dry_run } => run_repo_convert(&path, to, dry_run)?,
    }
    Ok(())
}

fn run_repo_list() -> anyhow::Result<()> {
    let ctx = open_context()?;
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
    Ok(())
}

fn run_repo_convert(path: &Path, to: CliConvertTarget, dry_run: bool) -> anyhow::Result<()> {
    let ctx = open_context()?;
    let core_target = match to {
        CliConvertTarget::Bare => ConvertTarget::Bare,
        CliConvertTarget::Local => ConvertTarget::Local,
    };
    match ctx
        .convert_repo(path, core_target, dry_run)
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
    Ok(())
}

fn run_excludes(sub: ExcludesCommands) -> anyhow::Result<()> {
    let ctx = open_context()?;
    match sub {
        ExcludesCommands::List => {
            for glob in ctx.excludes_list()? {
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
    let ctx = open_context()?;
    match sub {
        RootsCommands::Add { path } => {
            ctx.roots_add(&path).map_err(map_roots_error)?;
            println!("watch root added: {}", path.display());
        }
        RootsCommands::List => {
            for root in ctx.roots_list()? {
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
    let ctx = open_context()?;
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
    let ctx = open_context()?;
    // Ambiguous and not-found identifiers come back as errors.
    let path_key = resolve_repo_identifier(&ctx, identifier)?;
    // Print the full canonical path before launch.
    println!("opening: {path_key}");
    launch_repo(&ctx, &path_key).map_err(LaunchFailed)?;
    Ok(())
}

/// Resolve CLI `repo` argument to SQLite `locations.path` (exact key, canonical path, or unique name).
fn resolve_repo_identifier(ctx: &AppContext, identifier: &str) -> anyhow::Result<String> {
    let identifier = identifier.trim();
    if identifier.is_empty() {
        anyhow::bail!("repo not found: (empty identifier)");
    }
    let repos = ctx.list_repos().context("failed to list repos")?;

    if let Some(path_key) = match_repo_path_key(&repos, identifier)
        .or_else(|| match_canonical_path_key(&repos, identifier))
    {
        return Ok(path_key);
    }

    let alias_matches: Vec<&RepoRecord> = repos
        .iter()
        .filter(|r| r.alias.as_deref() == Some(identifier))
        .collect();
    if let Some(path_key) = unique_match("alias", identifier, &alias_matches)? {
        return Ok(path_key);
    }

    let name_matches: Vec<&RepoRecord> = repos.iter().filter(|r| r.name == identifier).collect();
    if let Some(path_key) = unique_match("name", identifier, &name_matches)? {
        return Ok(path_key);
    }

    // Not cloned yet: `launch_repo` clones first when the source policy allows it.
    match ctx.resolve_remote_only(identifier)? {
        Some(record) => Ok(record.clone_path.display().to_string()),
        None => Err(anyhow::anyhow!("repo not found: {identifier}")),
    }
}

/// Path key of the single repo in `matches`: `None` when empty, an error listing every
/// candidate when ambiguous. `kind` names the matched field in the message.
fn unique_match(
    kind: &str,
    identifier: &str,
    matches: &[&RepoRecord],
) -> anyhow::Result<Option<String>> {
    match matches {
        [] => Ok(None),
        [only] => Ok(Some(only.path.display().to_string())),
        many => {
            let mut msg = format!("error: ambiguous repo {kind} '{identifier}'; matches:\n");
            for (i, r) in many.iter().enumerate() {
                msg.push_str(&format!("{}. {}\n", i + 1, r.path.display()));
            }
            msg.push_str("use the full path from 'workpot list'");
            Err(anyhow::anyhow!("{msg}"))
        }
    }
}

/// Path key of the repo whose canonical path is `identifier` (absolute, or relative with a separator).
fn match_canonical_path_key(repos: &[RepoRecord], identifier: &str) -> Option<String> {
    let path = Path::new(identifier);
    if !(path.is_absolute() || identifier.contains(std::path::MAIN_SEPARATOR)) {
        return None;
    }
    let canon = path.canonicalize().ok()?;
    repos
        .iter()
        .find(|r| r.path == canon)
        .map(|r| r.path.display().to_string())
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
            Cli::try_parse_from(["workpot", "sync"]).unwrap().command,
            Commands::Sync { command: None }
        ));
        assert!(matches!(
            Cli::try_parse_from(["workpot", "sync", "local"])
                .unwrap()
                .command,
            Commands::Sync {
                command: Some(SyncCommands::Local)
            }
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
        assert!(matches!(
            Cli::try_parse_from(["workpot", "db", "reset"])
                .unwrap()
                .command,
            Commands::Db(DbCommands::Reset)
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
                "local",
                "--dry-run"
            ])
            .unwrap()
            .command,
            Commands::Repo(RepoCommands::Convert { dry_run: true, .. })
        ));
    }

    #[test]
    fn parses_settings_init() {
        assert!(matches!(
            Cli::try_parse_from(["workpot", "settings", "init"])
                .unwrap()
                .command,
            Commands::Settings {
                args: SettingsArgs {
                    command: Some(SettingsCommands::Init { force: false }),
                    add_comments: false,
                },
            }
        ));
    }

    #[test]
    fn parses_settings_init_force() {
        assert!(matches!(
            Cli::try_parse_from(["workpot", "settings", "init", "--force"])
                .unwrap()
                .command,
            Commands::Settings {
                args: SettingsArgs {
                    command: Some(SettingsCommands::Init { force: true }),
                    add_comments: false,
                },
            }
        ));
    }

    #[test]
    fn parses_settings_add_comments() {
        assert!(matches!(
            Cli::try_parse_from(["workpot", "settings", "--add-comments"])
                .unwrap()
                .command,
            Commands::Settings {
                args: SettingsArgs {
                    command: None,
                    add_comments: true,
                },
            }
        ));
    }
}
