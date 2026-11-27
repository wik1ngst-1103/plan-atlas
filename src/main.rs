use std::process::ExitCode;

use clap::{Parser, Subcommand};

/// Plan Atlas: one local place to browse, search and track your Claude Code plans.
#[derive(Debug, Parser)]
#[command(name = "plan-atlas", version, about)]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Start the web UI (default when no subcommand is given)
    Serve {
        /// Port to listen on (default 7420, falls back up to 7430)
        #[arg(long, env = "PLAN_ATLAS_PORT")]
        port: Option<u16>,
        /// Do not open the browser
        #[arg(long)]
        no_open: bool,
    },
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
    UninstallClaude,
    /// Import plans from ~/.claude/plans
    Import,
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
    let cli = Cli::parse();
    let command = cli.command.unwrap_or(Command::Serve {
        port: None,
        no_open: false,
    });
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
        Command::Serve { .. }
        | Command::Mcp
        | Command::Hook
        | Command::Setup { .. }
        | Command::UninstallClaude
        | Command::Import
        | Command::Task { .. } => Err("not implemented yet".to_string()),
    }
}
