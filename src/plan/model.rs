use std::path::PathBuf;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub enum TaskStatus {
    #[default]
    Todo,
    InProgress,
    Review,
    Done,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub enum Model {
    Haiku,
    Sonnet,
    Opus,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub enum Effort {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub enum RelationKind {
    DependsOn,
    Extends,
    RelatesTo,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct Relation {
    pub to: String,
    pub kind: RelationKind,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct Task {
    pub id: String,
    pub name: String,
    pub complexity: Option<u8>,
    pub model: Option<Model>,
    pub effort: Option<Effort>,
    pub depends_on: Vec<String>,
    pub status: TaskStatus,
    pub tags: Vec<String>,
    pub body_md: String,
}

impl Task {
    /// Explicit model/effort win; missing parts come from the complexity table
    /// (missing complexity counts as 3).
    pub fn resolved_model(&self) -> (Model, Effort) {
        let (m, e) = match self.complexity.unwrap_or(3) {
            0 | 1 => (Model::Haiku, Effort::Low),
            2 => (Model::Sonnet, Effort::Low),
            3 => (Model::Sonnet, Effort::Medium),
            4 => (Model::Opus, Effort::Medium),
            _ => (Model::Opus, Effort::High),
        };
        (self.model.unwrap_or(m), self.effort.unwrap_or(e))
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct Section {
    pub level: u8,
    pub title: String,
    pub anchor: String,
    pub body_md: String,
    pub children: Vec<Section>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct Warning {
    pub code: String,
    pub message: String,
}

impl Warning {
    pub fn new(code: &str, message: impl Into<String>) -> Self {
        Warning {
            code: code.to_string(),
            message: message.into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct Plan {
    pub id: String,
    pub project: String,
    pub title: String,
    pub tags: Vec<String>,
    pub related: Vec<Relation>,
    pub created: Option<String>,
    pub sections: Vec<Section>,
    pub tasks: Vec<Task>,
    pub structured: bool,
    #[ts(type = "string")]
    pub source_path: PathBuf,
    pub content_hash: String,
    pub warnings: Vec<Warning>,
}
