//! Agent activity: the Claude Code hook, plain-language steps and the live
//! events pushed to the UI.

pub mod hook;
pub mod humanize;
pub mod live;

use serde::Serialize;

use crate::plan::TaskStatus;
use crate::store::AgentView;

pub use hook::run_hook;
pub use humanize::{Step, humanize};

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum LiveEvent {
    PlanChanged {
        plan_id: String,
    },
    TaskStatus {
        plan_id: String,
        task_id: String,
        status: TaskStatus,
    },
    AgentStep {
        plan_id: String,
        task_id: String,
        agent: AgentView,
    },
}
