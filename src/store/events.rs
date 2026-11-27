//! Agent activity recorded by the Claude Code hook: events, session↔task
//! links, live agents and per-task logs.

use std::time::{SystemTime, UNIX_EPOCH};

use anyhow::Result;
use rusqlite::{OptionalExtension, params};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use super::Store;
use crate::events::Step;

/// An agent counts as live while its latest event is at most this old.
const LIVE_WINDOW_MS: i64 = 120_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EventKind {
    PreTool,
    SubagentStart,
    SubagentStop,
    SessionEnd,
}

impl EventKind {
    pub fn as_str(self) -> &'static str {
        match self {
            EventKind::PreTool => "pre_tool",
            EventKind::SubagentStart => "subagent_start",
            EventKind::SubagentStop => "subagent_stop",
            EventKind::SessionEnd => "session_end",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HookEvent {
    /// Unix milliseconds.
    pub at: i64,
    pub session_id: String,
    pub agent_id: String,
    pub role: String,
    pub kind: EventKind,
    pub tool: Option<String>,
    pub step: Option<Step>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct AgentView {
    pub agent_id: String,
    pub role: String,
    pub model: Option<String>,
    pub step: Step,
    /// Unix milliseconds of the agent's latest event.
    #[ts(type = "number")]
    pub at: i64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct LiveTask {
    pub plan_id: String,
    pub task_id: String,
    pub agents: Vec<AgentView>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct LogEntry {
    #[ts(type = "number")]
    pub at: i64,
    pub role: String,
    pub step: Step,
}

/// A linked `pre_tool` event, as broadcast by the live poller.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StepEvent {
    pub id: i64,
    pub plan_id: String,
    pub task_id: String,
    pub agent: AgentView,
}

pub fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn parse_step(json: &str) -> Option<Step> {
    serde_json::from_str(json).ok()
}

impl Store {
    /// Record an event, tagged with the session's current task link (if any).
    pub fn insert_event(&self, ev: &HookEvent) -> Result<i64> {
        let step = ev.step.as_ref().map(serde_json::to_string).transpose()?;
        let conn = self.conn();
        conn.execute(
            "INSERT INTO events (at, session_id, agent_id, role, kind, tool, step_json, plan_id, task_id)
             SELECT ?1, ?2, ?3, ?4, ?5, ?6, ?7, s.plan_id, s.task_id
             FROM (SELECT 1) LEFT JOIN sessions s ON s.session_id = ?2",
            params![
                ev.at,
                ev.session_id,
                ev.agent_id,
                ev.role,
                ev.kind.as_str(),
                ev.tool,
                step
            ],
        )?;
        Ok(conn.last_insert_rowid())
    }

    pub fn set_session_task(&self, session_id: &str, plan_id: &str, task_id: &str) -> Result<()> {
        self.conn().execute(
            "INSERT INTO sessions (session_id, plan_id, task_id, updated_at) VALUES (?1, ?2, ?3, ?4)
             ON CONFLICT(session_id) DO UPDATE SET
               plan_id = excluded.plan_id, task_id = excluded.task_id, updated_at = excluded.updated_at",
            params![session_id, plan_id, task_id, now_ms()],
        )?;
        Ok(())
    }

    pub fn clear_session(&self, session_id: &str) -> Result<()> {
        self.conn()
            .execute("DELETE FROM sessions WHERE session_id = ?1", [session_id])?;
        Ok(())
    }

    /// Agents of linked sessions whose latest event is recent and not a
    /// `SubagentStop`, grouped by task. Agents with no step yet are skipped.
    pub fn live_agents(&self) -> Result<Vec<LiveTask>> {
        let since = now_ms() - LIVE_WINDOW_MS;
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "WITH latest AS (
                 SELECT session_id, agent_id, MAX(id) AS last_id
                 FROM events WHERE at >= ?1 GROUP BY session_id, agent_id
             )
             SELECT s.plan_id, s.task_id, e.agent_id, e.role, e.at,
                    (SELECT p.step_json FROM events p
                     WHERE p.at >= ?1 AND p.id <= l.last_id
                       AND p.session_id = e.session_id AND p.agent_id = e.agent_id
                       AND p.kind = 'pre_tool' AND p.step_json IS NOT NULL
                     ORDER BY p.id DESC LIMIT 1)
             FROM latest l
             JOIN events e ON e.id = l.last_id
             JOIN sessions s ON s.session_id = e.session_id
             WHERE e.kind <> 'subagent_stop'
             ORDER BY s.plan_id, s.task_id, e.session_id, e.id",
        )?;
        let rows = stmt.query_map([since], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, i64>(4)?,
                r.get::<_, Option<String>>(5)?,
            ))
        })?;
        let mut out: Vec<LiveTask> = Vec::new();
        for row in rows {
            let (plan_id, task_id, agent_id, role, at, step) = row?;
            let Some(step) = step.as_deref().and_then(parse_step) else {
                continue;
            };
            let agent = AgentView {
                agent_id,
                role,
                model: None,
                step,
                at,
            };
            match out.last_mut() {
                Some(t) if t.plan_id == plan_id && t.task_id == task_id => t.agents.push(agent),
                _ => out.push(LiveTask {
                    plan_id,
                    task_id,
                    agents: vec![agent],
                }),
            }
        }
        Ok(out)
    }

    /// The task's steps, newest first.
    pub fn task_log(&self, plan_id: &str, task_id: &str, limit: u32) -> Result<Vec<LogEntry>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT at, role, step_json FROM events
             WHERE plan_id = ?1 AND task_id = ?2 AND kind = 'pre_tool' AND step_json IS NOT NULL
             ORDER BY id DESC LIMIT ?3",
        )?;
        let rows = stmt.query_map(params![plan_id, task_id, limit], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (at, role, step) = row?;
            if let Some(step) = parse_step(&step) {
                out.push(LogEntry { at, role, step });
            }
        }
        Ok(out)
    }

    /// Delete events (and session links) older than the given age. Returns
    /// the number of events deleted.
    pub fn prune_events(&self, older_than_days: u32) -> Result<usize> {
        let cutoff = now_ms() - i64::from(older_than_days) * 86_400_000;
        let conn = self.conn();
        let n = conn.execute("DELETE FROM events WHERE at < ?1", [cutoff])?;
        conn.execute("DELETE FROM sessions WHERE updated_at < ?1", [cutoff])?;
        Ok(n)
    }

    /// Highest event id so far (0 when there are none).
    pub fn max_event_id(&self) -> Result<i64> {
        Ok(self
            .conn()
            .query_row("SELECT MAX(id) FROM events", [], |r| {
                r.get::<_, Option<i64>>(0)
            })
            .optional()?
            .flatten()
            .unwrap_or(0))
    }

    /// Linked `pre_tool` events with id greater than `after_id`, oldest first.
    pub fn steps_after(&self, after_id: i64) -> Result<Vec<StepEvent>> {
        let conn = self.conn();
        let mut stmt = conn.prepare_cached(
            "SELECT id, plan_id, task_id, agent_id, role, at, step_json FROM events
             WHERE id > ?1 AND kind = 'pre_tool' AND plan_id IS NOT NULL
               AND task_id IS NOT NULL AND step_json IS NOT NULL
             ORDER BY id",
        )?;
        let rows = stmt.query_map([after_id], |r| {
            Ok((
                r.get::<_, i64>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
                r.get::<_, String>(3)?,
                r.get::<_, String>(4)?,
                r.get::<_, i64>(5)?,
                r.get::<_, String>(6)?,
            ))
        })?;
        let mut out = Vec::new();
        for row in rows {
            let (id, plan_id, task_id, agent_id, role, at, step) = row?;
            if let Some(step) = parse_step(&step) {
                out.push(StepEvent {
                    id,
                    plan_id,
                    task_id,
                    agent: AgentView {
                        agent_id,
                        role,
                        model: None,
                        step,
                        at,
                    },
                });
            }
        }
        Ok(out)
    }
}
