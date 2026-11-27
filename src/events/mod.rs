//! Events pushed to the UI over the live channel.

use serde::Serialize;

use crate::plan::TaskStatus;

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
        agent: serde_json::Value,
    },
}
