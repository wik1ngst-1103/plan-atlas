//! SQLite index of plans. All methods are synchronous; async callers wrap
//! them in `spawn_blocking`.

mod plans;
mod search;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context, Result, bail};
use rusqlite::Connection;
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use crate::plan::{RelationKind, Task};
use crate::taxonomy::Taxonomy;

pub use search::fold;

const SCHEMA_V1: &str = include_str!("schema.sql");
const SCHEMA_VERSION: i64 = 1;

pub struct Store {
    conn: Mutex<Connection>,
    tax: Arc<Taxonomy>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct ProjectSummary {
    pub name: String,
    pub plan_count: u32,
    pub task_count: u32,
    pub done_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct PlanSummary {
    pub id: String,
    pub project: String,
    pub title: String,
    pub tags: Vec<String>,
    pub created: Option<String>,
    pub task_count: u32,
    pub done_count: u32,
    pub structured: bool,
    pub warnings_count: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct TaskRow {
    pub plan_id: String,
    pub plan_title: String,
    pub project: String,
    pub task: Task,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct TopicCount {
    pub tag: String,
    pub plans: u32,
    pub tasks: u32,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(tag = "type", content = "value", rename_all = "snake_case")]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub enum EdgeKind {
    Declared(RelationKind),
    SharedTopics(Vec<String>),
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct Edge {
    pub from: String,
    pub to: String,
    pub kind: EdgeKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct Graph {
    pub nodes: Vec<PlanSummary>,
    pub edges: Vec<Edge>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct SearchResults {
    pub plans: Vec<PlanSummary>,
    pub tasks: Vec<TaskRow>,
    pub topics: Vec<TopicCount>,
}

/// `tag` matches the plan's own tags or any of its tasks' tags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct PlanFilter {
    pub project: Option<String>,
    pub tag: Option<String>,
}

/// `tag` matches the task's own tags.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct TaskFilter {
    pub project: Option<String>,
    pub tag: Option<String>,
}

impl Store {
    pub fn open(path: &Path, tax: Arc<Taxonomy>) -> Result<Store> {
        let conn = Connection::open(path)
            .with_context(|| format!("opening database {}", path.display()))?;
        Self::init(conn, tax)
    }

    fn init(conn: Connection, tax: Arc<Taxonomy>) -> Result<Store> {
        // journal_mode returns a row; in-memory databases answer "memory".
        let _mode: String =
            conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get(0))?;
        conn.pragma_update(None, "busy_timeout", 2000)?;
        conn.pragma_update(None, "synchronous", "NORMAL")?; // safe with WAL; avoids an fsync per commit
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        match version {
            0 => {
                let tx = conn.unchecked_transaction()?;
                tx.execute_batch(SCHEMA_V1)?;
                tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
                tx.commit()?;
            }
            SCHEMA_VERSION => {}
            v => bail!("database schema version {v} is newer than supported ({SCHEMA_VERSION})"),
        }
        Ok(Store {
            conn: Mutex::new(conn),
            tax,
        })
    }

    pub(crate) fn conn(&self) -> MutexGuard<'_, Connection> {
        self.conn.lock().unwrap_or_else(|e| e.into_inner())
    }

    /// SQLite `PRAGMA data_version`: changes whenever another connection
    /// (e.g. the hook process) commits to the database.
    pub fn data_version(&self) -> Result<i64> {
        Ok(self
            .conn()
            .pragma_query_value(None, "data_version", |r| r.get(0))?)
    }

    /// Total rows modified by this connection so far (for tests/diagnostics).
    pub fn total_changes(&self) -> u64 {
        self.conn().total_changes()
    }
}
