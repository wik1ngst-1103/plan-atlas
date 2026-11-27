//! REST routes and the SSE stream.

use std::convert::Infallible;
use std::time::Duration;

use axum::Json;
use axum::Router;
use axum::body::Bytes;
use axum::extract::multipart::{MultipartError, MultipartRejection};
use axum::extract::{DefaultBodyLimit, Multipart, Path, Query, State};
use axum::http::StatusCode;
use axum::response::sse::{Event, KeepAlive, Sse};
use axum::response::{IntoResponse, Response};
use axum::routing::{get, patch, post};
use futures_util::Stream;
use serde::{Deserialize, Serialize};
use tokio::sync::broadcast::error::RecvError;

use super::AppState;
use crate::events::LiveEvent;
use crate::import::{ImportReport, Upload, import_uploads};
use crate::index::index_file;
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
        .route(
            "/api/import",
            post(import_upload).layer(DefaultBodyLimit::max(MAX_UPLOAD_BODY)),
        )
}

const MAX_UPLOAD_FILES: usize = 200;
const MAX_UPLOAD_FILE_BYTES: usize = 2 * 1024 * 1024;
const MAX_UPLOAD_TOTAL_BYTES: usize = 20 * 1024 * 1024;
/// Total files plus room for the multipart framing and the project field.
const MAX_UPLOAD_BODY: usize = MAX_UPLOAD_TOTAL_BYTES + 1024 * 1024;
const MAX_PROJECT_LEN: usize = 100;

fn bad_multipart(e: MultipartError) -> ApiError {
    if e.status() == StatusCode::PAYLOAD_TOO_LARGE {
        ApiError::BadRequest("upload too large: at most 20 MB in total".into())
    } else {
        ApiError::BadRequest(format!("invalid upload: {}", e.body_text()))
    }
}

/// `multipart/form-data` upload of `.md` files (`files`, the file name may be
/// a relative path from a folder pick) and an optional `project`. All or
/// nothing: any rejected file fails the whole request before anything is
/// written. The copies are indexed right away so the returned ids open; the
/// watcher's `plan_changed` events refresh open pages.
async fn import_upload(
    State(s): State<AppState>,
    mp: Result<Multipart, MultipartRejection>,
) -> ApiResult<ImportReport> {
    let mut mp =
        mp.map_err(|e| ApiError::BadRequest(format!("invalid upload: {}", e.body_text())))?;
    let mut files: Vec<Upload> = Vec::new();
    let mut project: Option<String> = None;
    let mut total = 0usize;
    while let Some(mut field) = mp.next_field().await.map_err(bad_multipart)? {
        match field.name() {
            Some("files") => {
                let name = field.file_name().unwrap_or_default().to_string();
                if !name.to_ascii_lowercase().ends_with(".md") {
                    return Err(ApiError::BadRequest(format!(
                        "only Markdown (.md) files can be imported: {name}"
                    )));
                }
                if files.len() >= MAX_UPLOAD_FILES {
                    return Err(ApiError::BadRequest(format!(
                        "too many files: at most {MAX_UPLOAD_FILES} per import"
                    )));
                }
                let mut bytes = Vec::new();
                while let Some(chunk) = field.chunk().await.map_err(bad_multipart)? {
                    bytes.extend_from_slice(&chunk);
                    if bytes.len() > MAX_UPLOAD_FILE_BYTES {
                        return Err(ApiError::BadRequest(format!(
                            "{name} is too large: at most 2 MB per file"
                        )));
                    }
                }
                total += bytes.len();
                if total > MAX_UPLOAD_TOTAL_BYTES {
                    return Err(ApiError::BadRequest(
                        "upload too large: at most 20 MB in total".into(),
                    ));
                }
                files.push(Upload { path: name, bytes });
            }
            Some("project") => {
                let text = field.text().await.map_err(bad_multipart)?;
                let text = text.trim();
                if text.chars().count() > MAX_PROJECT_LEN {
                    return Err(ApiError::BadRequest("project name too long".into()));
                }
                project = Some(text.to_string()).filter(|t| !t.is_empty());
            }
            _ => {}
        }
    }
    if files.is_empty() {
        return Err(ApiError::BadRequest("no files to import".into()));
    }
    blocking(move || {
        let report =
            import_uploads(&files, project.as_deref(), &s.paths).map_err(ApiError::internal)?;
        for p in &report.plans {
            if let Err(e) = index_file(&s.store, &s.tax, &p.path) {
                eprintln!(
                    "plan-atlas: warning: indexing {} failed: {e:#}",
                    p.path.display()
                );
            }
        }
        Ok(Json(report))
    })
    .await
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
        LiveEvent::ActivityChanged => "activity_changed",
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
