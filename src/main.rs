use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Plan Atlas: one local place to browse, search and track your Claude Code plans.
#[derive(Debug, Parser)]
#[command(name = "plan-atlas", version, about)]
#[command(args_conflicts_with_subcommands = true)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
    /// Options for the default `serve` command
    #[command(flatten)]
    serve: ServeArgs,
}

#[derive(Debug, Clone, clap::Args)]
struct ServeArgs {
    /// Port to listen on (default 7420, falls back up to 7430)
    #[arg(long, env = "PLAN_ATLAS_PORT")]
    port: Option<u16>,
    /// Do not open the browser
    #[arg(long)]
    no_open: bool,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the web UI (default when no subcommand is given)
    Serve(ServeArgs),
    /// Run the MCP server over stdio (started by Claude Code)
    Mcp,
    /// Handle a Claude Code hook event (called by Claude Code)
    Hook,
    /// Install the Skill, hooks and MCP server into Claude Code
    Setup {
        /// Do not ask for confirmation
        #[arg(long, short)]
        yes: bool,
    },
    /// Remove everything `setup` added to Claude Code
    UninstallClaude {
        /// Do not ask for confirmation
        #[arg(long, short)]
        yes: bool,
    },
    /// Import plans: from ~/.claude/plans, or from the given files and folders
    Import {
        /// Markdown files or folders (searched recursively for *.md).
        /// Without any, imports ~/.claude/plans.
        paths: Vec<std::path::PathBuf>,
        /// Put the imported plans in this project instead of guessing it
        #[arg(long, short)]
        project: Option<String>,
    },
    /// Update task status
    Task {
        #[command(subcommand)]
        action: TaskAction,
    },
}

#[derive(Debug, Subcommand)]
enum TaskAction {
    /// Mark a task as in progress
    Start { plan_id: String, task_id: String },
    /// Mark a task as done
    Done { plan_id: String, task_id: String },
}

fn main() -> ExitCode {
    // `hook` bypasses clap: a parse error would exit 2, which blocks the
    // tool call. Always exit 0 and print nothing.
    if std::env::args_os().nth(1).is_some_and(|a| a == "hook") {
        plan_atlas::events::run_hook(std::io::stdin(), &plan_atlas::config::Paths::resolve());
        return ExitCode::SUCCESS;
    }
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Serve(cli.serve));
    match run(command) {
        Ok(()) => ExitCode::SUCCESS,
        Err(msg) => {
            eprintln!("plan-atlas: {msg}");
            ExitCode::FAILURE
        }
    }
}

fn run(command: Command) -> Result<(), String> {
    // Stubs: each later task replaces one arm with a real implementation.
    match command {
        Command::Import { paths, project } => run_import(&paths, project.as_deref()),
        Command::Hook => Ok(()), // handled in `main` before parsing
        Command::Mcp => {
            plan_atlas::mcp::run(plan_atlas::config::Paths::resolve()).map_err(|e| format!("{e:#}"))
        }
        Command::Task { action } => run_task(action),
        Command::Setup { yes } => plan_atlas::setup::run_setup(yes),
        Command::UninstallClaude { yes } => plan_atlas::setup::run_uninstall(yes),
        Command::Serve(args) => run_serve(args),
    }
}

fn run_serve(args: ServeArgs) -> Result<(), String> {
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .map_err(|e| format!("cannot start runtime: {e}"))?;
    let result = rt.block_on(plan_atlas::launch::run(plan_atlas::launch::ServeOpts {
        port: args.port,
        no_open: args.no_open,
    }));
    // The background scan may still be running; do not wait for it.
    rt.shutdown_timeout(std::time::Duration::from_millis(100));
    result.map_err(|e| format!("{e:#}"))
}

fn run_import(sources: &[std::path::PathBuf], project: Option<&str>) -> Result<(), String> {
    use plan_atlas::import;
    let claude_home =
        plan_atlas::setup::claude_home().ok_or("cannot determine Claude Code config directory")?;
    let paths = plan_atlas::config::Paths::resolve();
    paths.ensure().map_err(|e| e.to_string())?;
    let r = if sources.is_empty() && project.is_none() {
        import::import_claude_plans(&claude_home, &paths)
    } else if sources.is_empty() {
        import::import_paths(&[claude_home.join("plans")], project, &claude_home, &paths)
    } else {
        import::import_paths(sources, project, &claude_home, &paths)
    }
    .map_err(|e| format!("{e:#}"))?;
    for f in &r.failures {
        eprintln!("plan-atlas: import failed for {}: {}", f.source, f.error);
    }
    println!("{}", r.summary());
    if r.failed > 0 {
        return Err(format!("{} file(s) could not be imported", r.failed));
    }
    Ok(())
}

fn run_task(action: TaskAction) -> Result<(), String> {
    use plan_atlas::plan::TaskStatus;
    let (plan_id, task_id, status, label) = match action {
        TaskAction::Start { plan_id, task_id } => {
            (plan_id, task_id, TaskStatus::InProgress, "in_progress")
        }
        TaskAction::Done { plan_id, task_id } => (plan_id, task_id, TaskStatus::Done, "done"),
    };
    let paths = plan_atlas::config::Paths::resolve();
    paths.ensure().map_err(|e| e.to_string())?;
    let tax = plan_atlas::taxonomy::Taxonomy::load(&paths);
    let store = plan_atlas::store::Store::open(&paths.db, std::sync::Arc::new(tax.clone()))
        .map_err(|e| format!("{e:#}"))?;
    plan_atlas::ops::set_task_status_op(&store, &tax, &paths, &plan_id, &task_id, status)
        .map_err(|e| e.to_string())?;
    println!("{task_id} \u{2192} {label}");
    Ok(())
}
