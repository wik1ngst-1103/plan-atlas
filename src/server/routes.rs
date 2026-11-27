//! REST routes and the SSE stream.

use std::convert::Infallible;
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::{Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;

use super::AppState;
use crate::events::LiveEvent;
use crate::ops::{OpError, set_task_status_op, valid_id};
use crate::plan::{Plan, TaskStatus};
use crate::store::{PlanFilter, TaskFilter};

pub(super) fn error_response(status: StatusCode, msg: &str) -> Response {
    (status, Json(serde_json::json!({ "error": msg }))).into_response()
}

#[derive(Debug)]
enum ApiError {
    BadRequest(String),
    NotFound,
    Internal,
}

impl ApiError {
    fn internal(e: impl std::fmt::Display) -> Self {
        eprintln!("plan-atlas: internal error: {e}");
        ApiError::Internal
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> Response {
        match self {
            ApiError::BadRequest(m) => error_response(StatusCode::BAD_REQUEST, &m),
            ApiError::NotFound => error_response(StatusCode::NOT_FOUND, "not found"),
            ApiError::Internal => {
                error_response(StatusCode::INTERNAL_SERVER_ERROR, "internal error")
            }
        }
    }
}

type ApiResult<T> = Result<Json<T>, ApiError>;

pub(super) fn router() -> Router<AppState> {
    Router::new()
        .route("/api/health", get(health))
        .route("/api/projects", get(projects))
        .route("/api/plans", get(plans))
        .route("/api/plans/{id}", get(plan))
        .route("/api/plans/{id}/tasks/{task}", patch(patch_task))
        .route("/api/plans/{id}/tasks/{task}/log", get(task_log))
        .route("/api/activity", get(activity))
        .route("/api/tasks", get(tasks))
        .route("/api/topics", get(topics))
        .route("/api/graph", get(graph))
        .route("/api/search", get(search))
        .route("/api/stream", get(stream))
}

/// Run a blocking store closure off the async runtime.
async fn blocking<T, F>(f: F) -> Result<T, ApiError>
where
    T: Send + 'static,
    F: FnOnce() -> Result<T, ApiError> + Send + 'static,
{
    tokio::task::spawn_blocking(f)
        .await
        .map_err(ApiError::internal)?
}

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({ "app": "plan-atlas", "version": env!("CARGO_PKG_VERSION") }))
}

async fn projects(State(s): State<AppState>) -> ApiResult<Vec<crate::store::ProjectSummary>> {
    blocking(move || {
        s.store
            .list_projects()
            .map(Json)
            .map_err(ApiError::internal)
    })
    .await
}

#[derive(Deserialize)]
struct Filter {
    project: Option<String>,
    tag: Option<String>,
}

async fn plans(
    State(s): State<AppState>,
    Query(q): Query<Filter>,
) -> ApiResult<Vec<crate::store::PlanSummary>> {
    blocking(move || {
        let f = PlanFilter {
            project: q.project,
            tag: q.tag,
        };
        s.store.list_plans(&f).map(Json).map_err(ApiError::internal)
    })
    .await
}

async fn plan(
    State(s): State<AppState>,
    Path(id): Path<String>,
) -> ApiResult<crate::store::PlanDetail> {
    if !valid_id(&id) {
        return Err(ApiError::NotFound);
    }
    blocking(move || match s.store.plan_detail(&id) {
        Ok(Some(p)) => Ok(Json(p)),
        Ok(None) => Err(ApiError::NotFound),
        Err(e) => Err(ApiError::internal(e)),
    })
    .await
}

async fn tasks(
    State(s): State<AppState>,
    Query(q): Query<Filter>,
) -> ApiResult<Vec<crate::store::TaskRow>> {
    blocking(move || {
        let f = TaskFilter {
            project: q.project,
            tag: q.tag,
        };
        s.store.list_tasks(&f).map(Json).map_err(ApiError::internal)
    })
    .await
}

#[derive(Deserialize)]
struct TopicQuery {
    locale: Option<String>,
}

#[derive(Serialize)]
struct Topic {
    tag: String,
    group: String,
    label: String,
    desc: String,
    plans: u32,
    tasks: u32,
}

async fn topics(State(s): State<AppState>, Query(q): Query<TopicQuery>) -> ApiResult<Vec<Topic>> {
    let vi = q.locale.as_deref() == Some("vi");
    blocking(move || {
        let counts = s.store.topics().map_err(ApiError::internal)?;
        let out = counts
            .into_iter()
            .map(|c| match s.tax.tags.get(&c.tag) {
                Some(info) => Topic {
                    group: info.group.clone(),
                    label: if vi { &info.label_vi } else { &info.label_en }.clone(),
                    desc: if vi { &info.desc_vi } else { &info.desc_en }.clone(),
                    tag: c.tag,
                    plans: c.plans,
                    tasks: c.tasks,
                },
                None => Topic {
                    group: String::new(),
                    label: c.tag.clone(),
                    desc: String::new(),
                    tag: c.tag,
                    plans: c.plans,
                    tasks: c.tasks,
                },
            })
            .collect();
        Ok(Json(out))
    })
    .await
}

async fn graph(State(s): State<AppState>) -> ApiResult<crate::store::Graph> {
    blocking(move || s.store.graph().map(Json).map_err(ApiError::internal)).await
}

#[derive(Deserialize)]
struct SearchQuery {
    q: Option<String>,
    limit: Option<usize>,
}

async fn search(
    State(s): State<AppState>,
    Query(q): Query<SearchQuery>,
) -> ApiResult<crate::store::SearchResults> {
    let limit = q.limit.unwrap_or(8).clamp(1, 50);
    let text = q.q.unwrap_or_default();
    blocking(move || {
        s.store
            .search(&text, limit)
            .map(Json)
            .map_err(ApiError::internal)
    })
    .await
}

#[derive(Deserialize)]
struct StatusBody {
    status: TaskStatus,
}

async fn patch_task(
    State(s): State<AppState>,
    Path((plan_id, task_id)): Path<(String, String)>,
    body: Bytes,
) -> ApiResult<Plan> {
    if !valid_id(&plan_id) || !valid_id(&task_id) {
        return Err(ApiError::NotFound);
    }
    let StatusBody { status } = serde_json::from_slice(&body)
        .map_err(|e| ApiError::BadRequest(format!("invalid body: {e}")))?;
    let live = s.live.clone();
    let (pid, tid) = (plan_id.clone(), task_id.clone());
    let plan = blocking(move || {
        set_task_status_op(&s.store, &s.tax, &s.paths, &pid, &tid, status).map_err(|e| match e {
            OpError::NotFound(_) => ApiError::NotFound,
            OpError::Invalid(m) => ApiError::BadRequest(m),
            OpError::Internal(m) => ApiError::internal(m),
        })
    })
    .await?;
    let _ = live.send(LiveEvent::TaskStatus {
        plan_id,
        task_id,
        status,
    });
    Ok(Json(plan))
}

async fn activity(State(s): State<AppState>) -> ApiResult<Vec<crate::store::LiveTask>> {
    blocking(move || s.store.live_agents().map(Json).map_err(ApiError::internal)).await
}

#[derive(Deserialize)]
struct LogQuery {
    limit: Option<u32>,
}

async fn task_log(
    State(s): State<AppState>,
    Path((plan_id, task_id)): Path<(String, String)>,
    Query(q): Query<LogQuery>,
) -> ApiResult<Vec<crate::store::LogEntry>> {
    if !valid_id(&plan_id) || !valid_id(&task_id) {
        return Err(ApiError::NotFound);
    }
    let limit = q.limit.unwrap_or(30).clamp(1, 200);
    blocking(move || {
        s.store
            .task_log(&plan_id, &task_id, limit)
            .map(Json)
            .map_err(ApiError::internal)
    })
    .await
}

fn live_type(ev: &LiveEvent) -> &'static str {
    match ev {
        LiveEvent::PlanChanged { .. } => "plan_changed",
        LiveEvent::TaskStatus { .. } => "task_status",
        LiveEvent::AgentStep { .. } => "agent_step",
        LiveEvent::IndexReady => "index_ready",
    }
}

async fn stream(State(s): State<AppState>) -> Sse<impl Stream<Item = Result<Event, Infallible>>> {
    let rx = s.live.subscribe();
    let events = futures_util::stream::unfold(rx, |mut rx| async move {
        loop {
            match rx.recv().await {
                Ok(ev) => {
                    let data = serde_json::to_string(&ev).unwrap_or_default();
                    let event = Event::default().event(live_type(&ev)).data(data);
                    return Some((Ok(event), rx));
                }
                Err(RecvError::Lagged(_)) => continue,
                Err(RecvError::Closed) => return None,
            }
        }
    });
    Sse::new(events).keep_alive(KeepAlive::new().interval(Duration::from_secs(15)))
}
