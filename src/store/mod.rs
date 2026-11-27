//! SQLite index of plans. All methods are synchronous; async callers wrap
//! them in `spawn_blocking`.

mod events;
mod plans;
mod search;

use std::path::Path;
use std::sync::{Arc, Mutex, MutexGuard};

use anyhow::{Context, Result, bail};
use rusqlite::{Connection, Transaction, TransactionBehavior};
use serde::{Deserialize, Serialize};
use ts_rs::TS;

use std::collections::BTreeMap;

use crate::plan::{Plan, RelationKind, Task, TaskStatus};
use crate::taxonomy::Taxonomy;

pub use events::{AgentView, EventKind, HookEvent, LiveTask, LogEntry, StepEvent, now_ms};
pub use search::fold;

const SCHEMA_V1: &str = include_str!("schema.sql");
/// v1 → v2: agent activity. Idempotent, so concurrent openers (server and
/// hook process) can both run it.
const MIGRATE_V2: &str = include_str!("schema_v2.sql");
const SCHEMA_VERSION: i64 = 2;

/// A write transaction that takes SQLite's write lock up front, so it waits
/// on `busy_timeout` instead of failing to upgrade a read lock mid-way.
pub(crate) fn immediate(conn: &Connection) -> rusqlite::Result<Transaction<'_>> {
    Transaction::new_unchecked(conn, TransactionBehavior::Immediate)
}

/// Switch to WAL. Switching a fresh database can report SQLITE_BUSY without
/// waiting on `busy_timeout` while another process (server vs hook) does the
/// same, so retry briefly.
fn enable_wal(conn: &Connection) -> rusqlite::Result<()> {
    let mut tries = 0;
    loop {
        // journal_mode returns a row; in-memory databases answer "memory".
        match conn.pragma_update_and_check(None, "journal_mode", "WAL", |r| r.get::<_, String>(0)) {
            Ok(_) => return Ok(()),
            Err(rusqlite::Error::SqliteFailure(e, _))
                if e.code == rusqlite::ErrorCode::DatabaseBusy && tries < 100 =>
            {
                tries += 1;
                std::thread::sleep(std::time::Duration::from_millis(10));
            }
            Err(e) => return Err(e),
        }
    }
}

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
    /// The plan's tasks in file order, without bodies (cards, live activity).
    pub tasks: Vec<TaskBrief>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct TaskBrief {
    pub id: String,
    pub name: String,
    pub status: TaskStatus,
}

/// A declared relation pointing at a plan, from another plan.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct IncomingRelation {
    pub from_id: String,
    pub from_title: String,
    pub kind: RelationKind,
}

/// `GET /api/plans/{id}`: the plan plus what the plan page needs about its neighbours.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct PlanDetail {
    #[serde(flatten)]
    #[ts(flatten)]
    pub plan: Plan,
    pub incoming: Vec<IncomingRelation>,
    /// Title of each plan this plan's `related` entries point to (when indexed).
    pub outgoing_titles: BTreeMap<String, String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct TaskRow {
    pub plan_id: String,
    pub plan_title: String,
    pub project: String,
    /// Tags of the plan itself (board filters and groups by plan or task topics).
    pub plan_tags: Vec<String>,
    /// The task without its body (`body_md` is empty; bodies come from the plan).
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
        // First, so concurrent openers wait instead of failing with SQLITE_BUSY.
        conn.pragma_update(None, "busy_timeout", 2000)?;
        enable_wal(&conn).context("enabling WAL")?;
        conn.pragma_update(None, "synchronous", "NORMAL")?; // safe with WAL; avoids an fsync per commit
        conn.pragma_update(None, "foreign_keys", "ON")?;
        let version: i64 = conn.pragma_query_value(None, "user_version", |r| r.get(0))?;
        if version < SCHEMA_VERSION {
            // Server and hook process may both create the database: take the
            // write lock first, then re-read the version under it.
            let tx = immediate(&conn).context("locking for migration")?;
            let version: i64 = tx.pragma_query_value(None, "user_version", |r| r.get(0))?;
            if version < SCHEMA_VERSION {
                if version == 0 {
                    tx.execute_batch(SCHEMA_V1)?;
                }
                tx.execute_batch(MIGRATE_V2)?;
                tx.pragma_update(None, "user_version", SCHEMA_VERSION)?;
            }
            tx.commit()?;
        } else if version > SCHEMA_VERSION {
            bail!("database schema version {version} is newer than supported ({SCHEMA_VERSION})");
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
