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
    UninstallClaude {
        /// Do not ask for confirmation
        #[arg(long, short)]
        yes: bool,
    },
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
    // `hook` bypasses clap: a parse error would exit 2, which blocks the
    // tool call. Always exit 0 and print nothing.
    if std::env::args_os().nth(1).is_some_and(|a| a == "hook") {
        plan_atlas::events::run_hook(std::io::stdin(), &plan_atlas::config::Paths::resolve());
        return ExitCode::SUCCESS;
    }
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
        Command::Import => run_import(),
        Command::Hook => Ok(()), // handled in `main` before parsing
        Command::Mcp => {
            plan_atlas::mcp::run(plan_atlas::config::Paths::resolve()).map_err(|e| format!("{e:#}"))
        }
        Command::Task { action } => run_task(action),
        Command::Setup { yes } => run_setup(yes),
        Command::UninstallClaude { yes } => run_uninstall(yes),
        Command::Serve { .. } => Err("not implemented yet".to_string()),
    }
}

fn run_import() -> Result<(), String> {
    let claude_home = dirs::home_dir()
        .ok_or("cannot determine home directory")?
        .join(".claude");
    let paths = plan_atlas::config::Paths::resolve();
    paths.ensure().map_err(|e| e.to_string())?;
    let r = plan_atlas::import::import_claude_plans(&claude_home, &paths)
        .map_err(|e| format!("{e:#}"))?;
    let failed = if r.failed > 0 {
        format!(", {} failed", r.failed)
    } else {
        String::new()
    };
    println!(
        "Imported {}, skipped {} ({} unsorted{failed})",
        r.imported, r.skipped, r.unsorted
    );
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

/// Ask `Continue? [Y/n]` on a TTY; without a TTY require `--yes`.
fn confirm(yes: bool, cmd: &str) -> Result<bool, String> {
    use std::io::{BufRead, IsTerminal, Write};
    if yes {
        return Ok(true);
    }
    if !std::io::stdin().is_terminal() {
        return Err(format!(
            "stdin is not a terminal; re-run with `plan-atlas {cmd} --yes` to confirm"
        ));
    }
    print!("Continue? [Y/n] ");
    std::io::stdout().flush().map_err(|e| e.to_string())?;
    let mut answer = String::new();
    std::io::stdin()
        .lock()
        .read_line(&mut answer)
        .map_err(|e| e.to_string())?;
    let a = answer.trim().to_lowercase();
    Ok(a.is_empty() || a == "y" || a == "yes")
}

fn print_mcp(status: &plan_atlas::setup::McpStatus) {
    use plan_atlas::setup::McpStatus;
    match status {
        McpStatus::Registered => println!("  MCP server: registered"),
        McpStatus::AlreadyRegistered => println!("  MCP server: already registered"),
        McpStatus::Removed => println!("  MCP server: removed"),
        McpStatus::Manual { command } => {
            println!("  MCP server: `claude` not found on PATH; run this yourself:\n    {command}")
        }
        McpStatus::Failed { command, error } => println!(
            "  MCP server: WARNING, command failed ({error}); run it yourself:\n    {command}"
        ),
    }
}

fn print_settings(r: &plan_atlas::setup::SetupReport, settings: &std::path::Path) {
    if r.settings_changed {
        println!("  Hooks: updated {}", settings.display());
    } else {
        println!("  Hooks: {} already up to date", settings.display());
    }
    if let Some(b) = &r.backup {
        println!("  Backup: {}", b.display());
    }
}

fn run_setup(yes: bool) -> Result<(), String> {
    use plan_atlas::setup;
    let home = setup::claude_home().ok_or("cannot determine Claude Code config directory")?;
    let bin = std::env::current_exe()
        .and_then(|p| p.canonicalize())
        .map_err(|e| format!("cannot locate the plan-atlas binary: {e}"))?;
    let claude = setup::find_claude_cli();
    let settings = setup::settings_path(&home);
    println!("plan-atlas setup will:");
    println!(
        "  - write the Skill to {}",
        setup::skill_dir(&home).join("SKILL.md").display()
    );
    println!(
        "  - add hooks (PreToolUse, SubagentStart, SubagentStop, SessionEnd) running `{}` to {} (backed up first)",
        setup::hook_command(&bin),
        settings.display()
    );
    let claude_name = claude
        .as_deref()
        .map_or("claude".to_string(), |p| p.display().to_string());
    match &claude {
        Some(_) => println!(
            "  - register the MCP server: {}",
            setup::mcp_add_command(&claude_name, &bin)
        ),
        None => println!(
            "  - print the command to register the MCP server (`claude` not found on PATH)"
        ),
    }
    if !confirm(yes, "setup")? {
        println!("Aborted; nothing changed.");
        return Ok(());
    }
    let paths = plan_atlas::config::Paths::resolve();
    let tax = plan_atlas::taxonomy::Taxonomy::load(&paths);
    let r = setup::install(&home, &bin, &tax, claude.as_deref()).map_err(|e| format!("{e:#}"))?;
    println!("Done:");
    println!("  Skill: {}", r.skill_path.display());
    print_settings(&r, &settings);
    print_mcp(&r.mcp);
    let mut config = plan_atlas::config::Config::load(&paths);
    config.setup_done = true;
    config
        .save(&paths)
        .map_err(|e| format!("cannot save {}: {e}", paths.config.display()))?;
    Ok(())
}

fn run_uninstall(yes: bool) -> Result<(), String> {
    use plan_atlas::setup;
    let home = setup::claude_home().ok_or("cannot determine Claude Code config directory")?;
    let claude = setup::find_claude_cli();
    let settings = setup::settings_path(&home);
    let claude_name = claude
        .as_deref()
        .map_or("claude".to_string(), |p| p.display().to_string());
    println!("plan-atlas uninstall-claude will:");
    println!("  - delete {}", setup::skill_dir(&home).display());
    println!(
        "  - remove only plan-atlas hook handlers from {} (backed up first)",
        settings.display()
    );
    println!(
        "  - unregister the MCP server: {}",
        setup::mcp_remove_command(&claude_name)
    );
    if !confirm(yes, "uninstall-claude")? {
        println!("Aborted; nothing changed.");
        return Ok(());
    }
    let r = setup::uninstall(&home, claude.as_deref()).map_err(|e| format!("{e:#}"))?;
    println!("Done:");
    println!("  Skill: removed {}", r.skill_path.display());
    print_settings(&r, &settings);
    print_mcp(&r.mcp);
    let paths = plan_atlas::config::Paths::resolve();
    let mut config = plan_atlas::config::Config::load(&paths);
    if config.setup_done {
        config.setup_done = false;
        config
            .save(&paths)
            .map_err(|e| format!("cannot save {}: {e}", paths.config.display()))?;
    }
    Ok(())
}
