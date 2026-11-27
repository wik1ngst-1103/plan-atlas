//! Operations shared by the REST API, the MCP server and the CLI.

use std::fmt;
use std::path::{Path, PathBuf};

use crate::config::Paths;
use crate::index::index_file;
use crate::plan::{Plan, TaskStatus, WriteError, set_task_status, write_atomic};
use crate::store::Store;
use crate::taxonomy::Taxonomy;

#[derive(Debug)]
pub enum OpError {
    /// Unknown or invalid plan/task id, or a plan file outside `paths.plans`.
    NotFound(String),
    /// Input that cannot be processed (e.g. unparseable frontmatter).
    Invalid(String),
    Internal(String),
}

impl OpError {
    pub fn internal(e: impl fmt::Display) -> Self {
        OpError::Internal(e.to_string())
    }
}

impl fmt::Display for OpError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            OpError::NotFound(m) | OpError::Invalid(m) => write!(f, "{m}"),
            OpError::Internal(m) => write!(f, "internal error: {m}"),
        }
    }
}

impl std::error::Error for OpError {}

/// Ids are short `[A-Za-z0-9_-]` strings.
pub fn valid_id(id: &str) -> bool {
    (1..=100).contains(&id.len())
        && id
            .bytes()
            .all(|b| b.is_ascii_alphanumeric() || b == b'_' || b == b'-')
}

fn plan_not_found(plan_id: &str) -> OpError {
    OpError::NotFound(format!("plan not found: {plan_id}"))
}

/// The plan's source file, only if it resolves to somewhere under `paths.plans`.
pub fn safe_plan_path(store: &Store, paths: &Paths, plan_id: &str) -> Result<PathBuf, OpError> {
    if !valid_id(plan_id) {
        return Err(plan_not_found(plan_id));
    }
    let raw = store
        .find_task_path(plan_id)
        .map_err(OpError::internal)?
        .ok_or_else(|| plan_not_found(plan_id))?;
    let root = std::fs::canonicalize(&paths.plans).map_err(OpError::internal)?;
    let path = std::fs::canonicalize(raw).map_err(|_| plan_not_found(plan_id))?;
    if path.starts_with(&root) {
        Ok(path)
    } else {
        Err(plan_not_found(plan_id))
    }
}

const LOCK_FILE: &str = ".plan-atlas.lock";

/// Exclusive advisory lock on `<dir>/.plan-atlas.lock`, held until the
/// returned file is dropped. Serializes id assignment and plan writes across
/// threads and across processes (server, `plan-atlas mcp` per Claude
/// session, CLI).
pub fn lock_dir(dir: &Path) -> Result<std::fs::File, OpError> {
    std::fs::create_dir_all(dir).map_err(OpError::internal)?;
    let f = std::fs::OpenOptions::new()
        .create(true)
        .write(true)
        .truncate(false)
        .open(dir.join(LOCK_FILE))
        .map_err(OpError::internal)?;
    f.lock().map_err(OpError::internal)?;
    Ok(f)
}

/// Under the plan directory's lock: read → set the task's status line →
/// atomic write → reindex; returns the freshly indexed plan.
pub fn set_task_status_op(
    store: &Store,
    tax: &Taxonomy,
    paths: &Paths,
    plan_id: &str,
    task_id: &str,
    status: TaskStatus,
) -> Result<Plan, OpError> {
    if !valid_id(task_id) {
        return Err(OpError::NotFound(format!("task not found: {task_id}")));
    }
    let path = safe_plan_path(store, paths, plan_id)?;
    let dir = path
        .parent()
        .ok_or_else(|| OpError::internal("plan path has no parent"))?;
    // Held across read → write → reindex so concurrent changes don't lose updates.
    let _lock = lock_dir(dir)?;
    let bytes = std::fs::read(&path).map_err(OpError::internal)?;
    let new = set_task_status(&bytes, task_id, status).map_err(|e| match e {
        WriteError::TaskNotFound(_) => {
            OpError::NotFound(format!("task not found: {task_id} in {plan_id}"))
        }
    })?;
    write_atomic(&path, &new).map_err(OpError::internal)?;
    let id = index_file(store, tax, &path)
        .map_err(OpError::internal)?
        .unwrap_or_else(|| plan_id.to_string());
    store
        .get_plan(&id)
        .map_err(OpError::internal)?
        .ok_or_else(|| plan_not_found(&id))
}
