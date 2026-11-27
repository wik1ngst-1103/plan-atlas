//! `plan-atlas hook`: record a Claude Code hook event.
//!
//! Runs inside every Claude Code tool call, so it must never block, slow or
//! alter Claude Code: all errors are swallowed, panics are caught, and the
//! caller gets control back within [`DEADLINE`] whatever the worker is doing.

use std::io::Read;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::Arc;
use std::sync::mpsc;
use std::thread;
use std::time::Duration;

use anyhow::Result;
use serde::Deserialize;
use serde_json::Value;

use super::humanize::humanize;
use crate::config::Paths;
use crate::store::{EventKind, HookEvent, Store, now_ms};
use crate::taxonomy::Taxonomy;

const DEADLINE: Duration = Duration::from_millis(200);
const MAX_STDIN: u64 = 1024 * 1024;

const MCP_START: &str = "mcp__plan-atlas__start_task";
const MCP_COMPLETE: &str = "mcp__plan-atlas__complete_task";

#[derive(Deserialize)]
struct Payload {
    session_id: String,
    hook_event_name: String,
    tool_name: Option<String>,
    #[serde(default)]
    tool_input: Value,
    agent_id: Option<String>,
    agent_type: Option<String>,
}

/// What a tool call means for the session's task link.
#[derive(Debug, PartialEq, Eq)]
enum Link {
    Start(String, String),
    Done,
    None,
}

/// Handle one hook payload. Never panics, never writes to stdout, and
/// returns within [`DEADLINE`]; work still running then is abandoned (the
/// process exits right after).
pub fn run_hook(stdin: impl Read + Send + 'static, paths: &Paths) {
    let paths = paths.clone();
    let (tx, rx) = mpsc::channel();
    let spawned = thread::Builder::new()
        .name("plan-atlas-hook".into())
        .spawn(move || {
            let _ = catch_unwind(AssertUnwindSafe(|| {
                let _ = process(stdin, &paths);
            }));
            let _ = tx.send(());
        });
    if spawned.is_ok() {
        let _ = rx.recv_timeout(DEADLINE);
    }
}

fn process(stdin: impl Read, paths: &Paths) -> Result<()> {
    let mut buf = Vec::new();
    stdin.take(MAX_STDIN).read_to_end(&mut buf)?;
    let p: Payload = serde_json::from_slice(&buf)?;
    let kind = match p.hook_event_name.as_str() {
        "PreToolUse" => EventKind::PreTool,
        "SubagentStart" => EventKind::SubagentStart,
        "SubagentStop" => EventKind::SubagentStop,
        "SessionEnd" => EventKind::SessionEnd,
        _ => return Ok(()),
    };
    if !paths.home.exists() {
        let _ = paths.ensure();
    }
    let store = Store::open(&paths.db, Arc::new(Taxonomy::embedded_default()))?;

    let tool = p.tool_name.filter(|_| kind == EventKind::PreTool);
    let step = tool.as_deref().map(|t| humanize(t, &p.tool_input));
    let link = tool
        .as_deref()
        .map_or(Link::None, |t| link_of(t, &p.tool_input));
    let ev = HookEvent {
        at: now_ms(),
        session_id: p.session_id,
        agent_id: p.agent_id.unwrap_or_else(|| "main".into()),
        role: p.agent_type.unwrap_or_else(|| "main".into()),
        kind,
        tool,
        step,
    };
    // The start call itself stays unlinked; the completing call is the
    // task's last step.
    store.insert_event(&ev)?;
    match link {
        Link::Start(plan, task) => store.set_session_task(&ev.session_id, &plan, &task)?,
        Link::Done => store.clear_session(&ev.session_id)?,
        Link::None if kind == EventKind::SessionEnd => store.clear_session(&ev.session_id)?,
        Link::None => {}
    }
    Ok(())
}

fn link_of(tool: &str, input: &Value) -> Link {
    let field = |k: &str| input.get(k).and_then(Value::as_str).filter(|v| valid_id(v));
    match tool {
        MCP_START => match (field("plan_id"), field("task_id")) {
            (Some(p), Some(t)) => Link::Start(p.into(), t.into()),
            _ => Link::None,
        },
        MCP_COMPLETE => Link::Done,
        "Bash" => input
            .get("command")
            .and_then(Value::as_str)
            .map_or(Link::None, bash_link),
        _ => Link::None,
    }
}

/// Finds `plan-atlas task start <plan> <task>` / `plan-atlas task done …`
/// anywhere in a shell command (the binary may be given by path).
fn bash_link(cmd: &str) -> Link {
    let words: Vec<&str> = cmd
        .split_whitespace()
        .map(|w| w.trim_matches(|c| c == '"' || c == '\''))
        .collect();
    for (i, w) in words.iter().enumerate() {
        let bin = w.rsplit(['/', '\\']).next().unwrap_or(w);
        if !matches!(bin, "plan-atlas" | "plan-atlas.exe") || words.get(i + 1) != Some(&"task") {
            continue;
        }
        match words.get(i + 2) {
            Some(&"start") => {
                if let (Some(p), Some(t)) = (words.get(i + 3), words.get(i + 4))
                    && valid_id(p)
                    && valid_id(t)
                {
                    return Link::Start(p.to_string(), t.to_string());
                }
            }
            Some(&"done") => return Link::Done,
            _ => {}
        }
    }
    Link::None
}

fn valid_id(id: &str) -> bool {
    (1..=100).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bash_link_forms() {
        let start = |p: &str, t: &str| Link::Start(p.into(), t.into());
        assert_eq!(
            bash_link("plan-atlas task start PLAN-a-001 TASK-002"),
            start("PLAN-a-001", "TASK-002")
        );
        assert_eq!(
            bash_link("cd x && \"/opt/bin/plan-atlas\" task start 'PLAN-a-001' TASK-1"),
            start("PLAN-a-001", "TASK-1")
        );
        assert_eq!(
            bash_link("plan-atlas task done PLAN-a-001 TASK-002"),
            Link::Done
        );
        assert_eq!(bash_link("plan-atlas task start PLAN-a-001"), Link::None);
        assert_eq!(bash_link("plan-atlas task start ../x TASK-1"), Link::None);
        assert_eq!(bash_link("echo plan-atlas-task start a b"), Link::None);
    }
}
