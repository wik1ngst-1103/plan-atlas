//! `plan-atlas setup` / `uninstall-claude`: install the Skill, merge our hook
//! handlers into Claude Code's `settings.json`, and register the MCP server.
//!
//! The user's settings are never lost: invalid JSON aborts before anything is
//! written, a timestamped backup is made before every write, and only handlers
//! whose command contains `plan-atlas hook` are ever touched.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::{Context, Result, bail};
use serde_json::{Map, Value, json};

use crate::plan::write_atomic;
use crate::taxonomy::Taxonomy;

pub const SKILL_TEMPLATE: &str = include_str!("../assets/skill/SKILL.md.tmpl");

const SERVER_NAME: &str = "plan-atlas";
const HOOK_TIMEOUT_SECS: u64 = 5;
/// Events we listen to; `true` = the event takes a tool matcher.
const HOOK_EVENTS: [(&str, bool); 4] = [
    ("PreToolUse", true),
    ("SubagentStart", false),
    ("SubagentStop", false),
    ("SessionEnd", false),
];
const GROUP_ORDER: [&str; 3] = ["type", "domain", "tech"];

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum McpStatus {
    Registered,
    AlreadyRegistered,
    /// Was registered with a different binary path; removed and re-added.
    Updated,
    /// Uninstall: `claude mcp remove` succeeded (or it was not registered).
    Removed,
    /// `claude` is not on PATH: the user should run `command`.
    Manual {
        command: String,
    },
    Failed {
        command: String,
        error: String,
    },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SetupReport {
    /// Skill file written (install) or skill directory removed (uninstall).
    pub skill_path: PathBuf,
    pub settings_changed: bool,
    pub backup: Option<PathBuf>,
    pub mcp: McpStatus,
}

/// `CLAUDE_CONFIG_DIR` if set, otherwise `~/.claude`.
pub fn claude_home() -> Option<PathBuf> {
    match std::env::var_os("CLAUDE_CONFIG_DIR") {
        Some(v) if !v.is_empty() => Some(PathBuf::from(v)),
        _ => dirs::home_dir().map(|h| h.join(".claude")),
    }
}

/// First executable `claude` on PATH.
pub fn find_claude_cli() -> Option<PathBuf> {
    let names: &[&str] = if cfg!(windows) {
        &["claude.exe", "claude.cmd", "claude"]
    } else {
        &["claude"]
    };
    let path = std::env::var_os("PATH")?;
    std::env::split_paths(&path)
        .flat_map(|dir| names.iter().map(move |n| dir.join(n)))
        .find(|p| is_executable(p))
}

fn is_executable(p: &Path) -> bool {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::metadata(p).is_ok_and(|m| m.is_file() && m.permissions().mode() & 0o111 != 0)
    }
    #[cfg(not(unix))]
    {
        p.is_file()
    }
}

pub fn settings_path(claude_home: &Path) -> PathBuf {
    claude_home.join("settings.json")
}

pub fn skill_dir(claude_home: &Path) -> PathBuf {
    claude_home.join("skills").join(SERVER_NAME)
}

/// `"<bin>" hook`; the path is quoted so spaces survive the shell.
pub fn hook_command(bin: &Path) -> String {
    format!("\"{}\" hook", bin.display())
}

/// Shell-ready `claude mcp add ...` line, for display.
pub fn mcp_add_command(claude: &str, bin: &Path) -> String {
    format!(
        "{claude} mcp add --transport stdio --scope user {SERVER_NAME} -- {} mcp",
        shell_quote(&bin.display().to_string())
    )
}

pub fn mcp_remove_command(claude: &str) -> String {
    format!("{claude} mcp remove --scope user {SERVER_NAME}")
}

fn shell_quote(s: &str) -> String {
    if s.contains([' ', '\t', '"', '\'']) {
        format!("\"{s}\"")
    } else {
        s.to_string()
    }
}

/// Ours = the command (quotes ignored) invokes `plan-atlas hook`.
fn is_our_command(cmd: &str) -> bool {
    let c = cmd.replace(['"', '\''], "");
    c.contains("plan-atlas hook") || c.contains("plan-atlas.exe hook")
}

fn is_our_handler(h: &Value) -> bool {
    h.get("command")
        .and_then(Value::as_str)
        .is_some_and(is_our_command)
}

/// Replace `{{TAXONOMY}}` with the tags as a Markdown list grouped by
/// type/domain/tech (any other group afterwards).
pub fn render_skill(tmpl: &str, tax: &Taxonomy) -> String {
    let mut groups: Vec<&str> = GROUP_ORDER.to_vec();
    for info in tax.tags.values() {
        if !groups.contains(&info.group.as_str()) {
            groups.push(&info.group);
        }
    }
    let mut out = String::new();
    for g in groups {
        let lines: Vec<String> = tax
            .tags
            .iter()
            .filter(|(_, i)| i.group == g)
            .map(|(tag, i)| format!("- `{tag}` — {}: {}", i.label_en, i.desc_en))
            .collect();
        if lines.is_empty() {
            continue;
        }
        if !out.is_empty() {
            out.push('\n');
        }
        out.push_str(&format!("### {g}\n\n{}\n", lines.join("\n")));
    }
    tmpl.replace("{{TAXONOMY}}", out.trim_end())
}

/// Reject shapes we would have to overwrite to merge into.
fn check_shape(settings: &Value) -> Result<()> {
    let Some(obj) = settings.as_object() else {
        bail!("settings.json is not a JSON object");
    };
    let Some(hooks) = obj.get("hooks") else {
        return Ok(());
    };
    let Some(hooks) = hooks.as_object() else {
        bail!("settings.json: \"hooks\" is not an object");
    };
    for (event, _) in HOOK_EVENTS {
        if let Some(v) = hooks.get(event) {
            let Some(groups) = v.as_array() else {
                bail!("settings.json: hooks.{event} is not an array");
            };
            if groups.iter().any(|g| !g.is_object()) {
                bail!("settings.json: hooks.{event} contains a non-object entry");
            }
        }
    }
    Ok(())
}

/// Add our handler to each event unless one is already present (then its
/// command is updated to `bin`). Everything else is kept as is, in order.
/// Settings with an unexpected shape are returned unchanged.
pub fn merge_hooks(settings: Value, bin: &Path) -> Value {
    if check_shape(&settings).is_err() {
        return settings;
    }
    let mut settings = settings;
    let command = hook_command(bin);
    let obj = settings.as_object_mut().expect("checked");
    let hooks = obj
        .entry("hooks")
        .or_insert_with(|| Value::Object(Map::new()))
        .as_object_mut()
        .expect("checked");
    for (event, with_matcher) in HOOK_EVENTS {
        let groups = hooks
            .entry(event)
            .or_insert_with(|| Value::Array(Vec::new()))
            .as_array_mut()
            .expect("checked");
        let mut found = false;
        for h in groups
            .iter_mut()
            .filter_map(|g| g.get_mut("hooks").and_then(Value::as_array_mut))
            .flatten()
            .filter(|h| is_our_handler(h))
        {
            found = true;
            h["command"] = Value::String(command.clone());
        }
        if !found {
            let handler =
                json!({ "type": "command", "command": command, "timeout": HOOK_TIMEOUT_SECS });
            let mut group = Map::new();
            if with_matcher {
                group.insert("matcher".into(), json!("*"));
            }
            group.insert("hooks".into(), json!([handler]));
            groups.push(Value::Object(group));
        }
    }
    settings
}

/// Remove our handlers from every event, then drop matcher groups, event
/// arrays and the `hooks` object that this left empty.
pub fn remove_hooks(settings: Value) -> Value {
    let mut settings = settings;
    let Some(obj) = settings.as_object_mut() else {
        return settings;
    };
    let Some(hooks) = obj.get_mut("hooks").and_then(Value::as_object_mut) else {
        return settings;
    };
    let mut emptied_events = Vec::new();
    for (event, groups) in hooks.iter_mut() {
        let Some(groups) = groups.as_array_mut() else {
            continue;
        };
        let mut touched = false;
        groups.retain_mut(|g| {
            let Some(handlers) = g.get_mut("hooks").and_then(Value::as_array_mut) else {
                return true;
            };
            let n = handlers.len();
            handlers.retain(|h| !is_our_handler(h));
            if handlers.len() == n {
                return true;
            }
            touched = true;
            !handlers.is_empty()
        });
        if touched && groups.is_empty() {
            emptied_events.push(event.clone());
        }
    }
    for e in emptied_events {
        hooks.shift_remove(&e);
    }
    if hooks.is_empty() {
        obj.shift_remove("hooks");
    }
    settings
}

/// Parse `settings.json`; a missing file is `{}`, invalid JSON is an error.
fn read_settings(path: &Path) -> Result<Option<Value>> {
    let text = match fs::read_to_string(path) {
        Ok(t) => t,
        Err(e) if e.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(e) => return Err(e).with_context(|| format!("cannot read {}", path.display())),
    };
    let v: Value = serde_json::from_str(&text)
        .with_context(|| format!("{} is not valid JSON; nothing was changed", path.display()))?;
    check_shape(&v).with_context(|| format!("{}: nothing was changed", path.display()))?;
    Ok(Some(v))
}

/// Back up (if the file exists) and write `new` when it differs from `old`.
/// Returns (changed, backup).
fn write_settings(
    path: &Path,
    old: Option<&Value>,
    new: &Value,
) -> Result<(bool, Option<PathBuf>)> {
    if old == Some(new) {
        return Ok((false, None));
    }
    let backup = if old.is_some() {
        Some(backup_file(path)?)
    } else {
        None
    };
    if let Some(dir) = path.parent() {
        fs::create_dir_all(dir).with_context(|| format!("cannot create {}", dir.display()))?;
    }
    let mut text = serde_json::to_string_pretty(new)?;
    text.push('\n');
    write_atomic(path, text.as_bytes())
        .with_context(|| format!("cannot write {}", path.display()))?;
    Ok((true, backup))
}

/// Copy to `settings.json.bak-<unix_ts>` (suffixed `-N` if taken).
fn backup_file(path: &Path) -> Result<PathBuf> {
    let ts = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_secs())
        .unwrap_or(0);
    let name = path.file_name().unwrap_or_default().to_string_lossy();
    let mut n = 0;
    let backup = loop {
        let suffix = if n == 0 {
            String::new()
        } else {
            format!("-{n}")
        };
        let p = path.with_file_name(format!("{name}.bak-{ts}{suffix}"));
        if !p.exists() {
            break p;
        }
        n += 1;
    };
    fs::copy(path, &backup).with_context(|| format!("cannot back up {}", path.display()))?;
    Ok(backup)
}

/// Run `claude <args>`; stdout on success.
fn run_claude(claude: &Path, args: &[&str]) -> Result<String, String> {
    match Command::new(claude).args(args).output() {
        Ok(out) if out.status.success() => Ok(String::from_utf8_lossy(&out.stdout).into_owned()),
        Ok(out) => {
            let stderr = String::from_utf8_lossy(&out.stderr).trim().to_string();
            let stdout = String::from_utf8_lossy(&out.stdout).trim().to_string();
            let msg = if stderr.is_empty() { stdout } else { stderr };
            Err(format!("{} {msg}", out.status).trim().to_string())
        }
        Err(e) => Err(e.to_string()),
    }
}

fn register_mcp(claude_cli: Option<&Path>, bin: &Path) -> McpStatus {
    let Some(claude) = claude_cli else {
        return McpStatus::Manual {
            command: mcp_add_command("claude", bin),
        };
    };
    let bin_s = bin.display().to_string();
    let mut replaced = false;
    if let Ok(details) = run_claude(claude, &["mcp", "get", SERVER_NAME]) {
        if details.contains(&bin_s) {
            return McpStatus::AlreadyRegistered;
        }
        // Registered with another (e.g. pre-upgrade) binary: replace it.
        if let Err(error) = run_claude(claude, &["mcp", "remove", "--scope", "user", SERVER_NAME]) {
            return McpStatus::Failed {
                command: format!(
                    "{} && {}",
                    mcp_remove_command(&claude.display().to_string()),
                    mcp_add_command(&claude.display().to_string(), bin)
                ),
                error,
            };
        }
        replaced = true;
    }
    let args = [
        "mcp",
        "add",
        "--transport",
        "stdio",
        "--scope",
        "user",
        SERVER_NAME,
        "--",
        &bin_s,
        "mcp",
    ];
    match run_claude(claude, &args) {
        Ok(_) if replaced => McpStatus::Updated,
        Ok(_) => McpStatus::Registered,
        Err(error) => McpStatus::Failed {
            command: mcp_add_command(&claude.display().to_string(), bin),
            error,
        },
    }
}

fn unregister_mcp(claude_cli: Option<&Path>) -> McpStatus {
    let Some(claude) = claude_cli else {
        return McpStatus::Manual {
            command: mcp_remove_command("claude"),
        };
    };
    match run_claude(claude, &["mcp", "remove", "--scope", "user", SERVER_NAME]) {
        Ok(_) => McpStatus::Removed,
        Err(e) if e.to_lowercase().contains("not found") => McpStatus::Removed,
        Err(error) => McpStatus::Failed {
            command: mcp_remove_command(&claude.display().to_string()),
            error,
        },
    }
}

/// Install everything. Settings are validated before any file is written.
pub fn install(
    claude_home: &Path,
    bin: &Path,
    tax: &Taxonomy,
    claude_cli: Option<&Path>,
) -> Result<SetupReport> {
    let settings_file = settings_path(claude_home);
    let old = read_settings(&settings_file)?;
    let new = merge_hooks(old.clone().unwrap_or_else(|| json!({})), bin);

    let dir = skill_dir(claude_home);
    fs::create_dir_all(&dir).with_context(|| format!("cannot create {}", dir.display()))?;
    let skill_path = dir.join("SKILL.md");
    write_atomic(&skill_path, render_skill(SKILL_TEMPLATE, tax).as_bytes())
        .with_context(|| format!("cannot write {}", skill_path.display()))?;

    let (settings_changed, backup) = write_settings(&settings_file, old.as_ref(), &new)?;
    let mcp = register_mcp(claude_cli, bin);
    Ok(SetupReport {
        skill_path,
        settings_changed,
        backup,
        mcp,
    })
}

/// Remove our hook handlers, the skill directory and the MCP registration.
pub fn uninstall(claude_home: &Path, claude_cli: Option<&Path>) -> Result<SetupReport> {
    let settings_file = settings_path(claude_home);
    let old = read_settings(&settings_file)?;
    let (settings_changed, backup) = match &old {
        Some(v) => write_settings(&settings_file, Some(v), &remove_hooks(v.clone()))?,
        None => (false, None),
    };
    let dir = skill_dir(claude_home);
    match fs::remove_dir_all(&dir) {
        Ok(()) => {}
        Err(e) if e.kind() == io::ErrorKind::NotFound => {}
        Err(e) => return Err(e).with_context(|| format!("cannot remove {}", dir.display())),
    }
    let mcp = unregister_mcp(claude_cli);
    Ok(SetupReport {
        skill_path: dir,
        settings_changed,
        backup,
        mcp,
    })
}
