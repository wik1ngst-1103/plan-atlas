use std::path::Path;
use std::sync::Arc;

use axum::Router;
use axum::body::Body;
use axum::http::{Request, StatusCode};
use http_body_util::BodyExt;
use plan_atlas::config::Paths;
use plan_atlas::events::LiveEvent;
use plan_atlas::index::Indexer;
use plan_atlas::plan::parse_plan;
use plan_atlas::server::{AppState, build_router};
use plan_atlas::store::Store;
use plan_atlas::taxonomy::Taxonomy;
use serde_json::Value;
use tempfile::TempDir;
use tokio::sync::broadcast;
use tower::ServiceExt;

const PORT: u16 = 7420;
const HOST: &str = "127.0.0.1:7420";

struct Env {
    _dir: TempDir,
    paths: Paths,
    state: AppState,
    app: Router,
}

fn env() -> Env {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(dir.path().to_path_buf());
    paths.ensure().unwrap();
    let proj = paths.plans.join("auth-gateway");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plans/structured.md"),
        proj.join("2026-11-20-jwt.md"),
    )
    .unwrap();
    let tax = Arc::new(Taxonomy::embedded_default());
    let store = Arc::new(Store::open(&paths.db, tax.clone()).unwrap());
    Indexer::new(paths.clone(), store.clone(), tax.clone())
        .full_scan()
        .unwrap();
    let (live, _) = broadcast::channel(16);
    let state = AppState {
        store,
        paths: Arc::new(paths.clone()),
        tax,
        live,
        port: PORT,
    };
    let app = build_router(state.clone());
    Env {
        _dir: dir,
        paths,
        state,
        app,
    }
}

fn req(method: &str, uri: &str) -> axum::http::request::Builder {
    Request::builder()
        .method(method)
        .uri(uri)
        .header("host", HOST)
}

async fn call(app: &Router, r: Request<Body>) -> (StatusCode, Value) {
    let resp = app.clone().oneshot(r).await.unwrap();
    let status = resp.status();
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    let v = serde_json::from_slice(&bytes).unwrap_or(Value::Null);
    (status, v)
}

const PLAN: &str = "PLAN-auth-gateway-001";

#[tokio::test]
async fn health_ok() {
    let e = env();
    let (st, v) = call(
        &e.app,
        req("GET", "/api/health").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["app"], "plan-atlas");
    assert!(v["version"].is_string());
}

#[tokio::test]
async fn localhost_host_accepted_case_insensitive() {
    let e = env();
    let r = Request::builder()
        .uri("/api/health")
        .header("host", "LocalHost:7420")
        .body(Body::empty())
        .unwrap();
    assert_eq!(call(&e.app, r).await.0, StatusCode::OK);
}

#[tokio::test]
async fn bad_host_forbidden() {
    let e = env();
    for host in ["evil.com:7420", "127.0.0.1:9999", "127.0.0.1"] {
        let r = Request::builder()
            .uri("/api/health")
            .header("host", host)
            .body(Body::empty())
            .unwrap();
        let (st, v) = call(&e.app, r).await;
        assert_eq!(st, StatusCode::FORBIDDEN, "{host}");
        assert!(v["error"].is_string());
    }
    let r = Request::builder()
        .uri("/api/health")
        .body(Body::empty())
        .unwrap();
    assert_eq!(call(&e.app, r).await.0, StatusCode::FORBIDDEN);
}

#[tokio::test]
async fn patch_status_updates_file_and_db() {
    let e = env();
    let mut rx = e.state.live.subscribe();
    let r = req("PATCH", &format!("/api/plans/{PLAN}/tasks/TASK-001"))
        .header("x-plan-atlas", "1")
        .header("content-type", "application/json")
        .body(Body::from(r#"{"status":"done"}"#))
        .unwrap();
    let (st, v) = call(&e.app, r).await;
    assert_eq!(st, StatusCode::OK, "{v}");
    let t = v["tasks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|t| t["id"] == "TASK-001")
        .unwrap();
    assert_eq!(t["status"], "done");

    let file = e.paths.plans.join("auth-gateway/2026-11-20-jwt.md");
    assert!(
        std::fs::read_to_string(file)
            .unwrap()
            .contains("- status: done")
    );
    let db = e.state.store.get_plan(PLAN).unwrap().unwrap();
    assert_eq!(
        db.tasks.iter().find(|t| t.id == "TASK-001").unwrap().status,
        plan_atlas::plan::TaskStatus::Done
    );
    match rx.try_recv().unwrap() {
        LiveEvent::TaskStatus {
            plan_id, task_id, ..
        } => assert_eq!((plan_id.as_str(), task_id.as_str()), (PLAN, "TASK-001")),
        other => panic!("unexpected {other:?}"),
    }
}

#[tokio::test]
async fn patch_bad_inputs() {
    let e = env();
    let patch = |plan: &str, task: &str, body: &'static str| {
        req("PATCH", &format!("/api/plans/{plan}/tasks/{task}"))
            .header("x-plan-atlas", "1")
            .body(Body::from(body))
            .unwrap()
    };
    let (st, _) = call(&e.app, patch(PLAN, "TASK-001", r#"{"status":"bogus"}"#)).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = call(&e.app, patch(PLAN, "TASK-001", "nope")).await;
    assert_eq!(st, StatusCode::BAD_REQUEST);
    let (st, _) = call(&e.app, patch(PLAN, "TASK-999", r#"{"status":"done"}"#)).await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let (st, _) = call(
        &e.app,
        patch("PLAN-nope-001", "TASK-001", r#"{"status":"done"}"#),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn patch_without_header_forbidden() {
    let e = env();
    let r = req("PATCH", &format!("/api/plans/{PLAN}/tasks/TASK-001"))
        .body(Body::from(r#"{"status":"done"}"#))
        .unwrap();
    let (st, _) = call(&e.app, r).await;
    assert_eq!(st, StatusCode::FORBIDDEN);
    let file = e.paths.plans.join("auth-gateway/2026-11-20-jwt.md");
    assert!(
        !std::fs::read_to_string(file)
            .unwrap()
            .contains("- status: done")
    );
}

#[tokio::test]
async fn unknown_plan_404() {
    let e = env();
    let (st, v) = call(
        &e.app,
        req("GET", "/api/plans/PLAN-x-999")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    assert!(v["error"].is_string());
}

#[tokio::test]
async fn read_endpoints() {
    let e = env();
    for uri in [
        "/api/projects",
        "/api/plans?project=auth-gateway",
        "/api/tasks?tag=domain/security",
        "/api/graph",
        &format!("/api/plans/{PLAN}"),
    ] {
        let (st, _) = call(&e.app, req("GET", uri).body(Body::empty()).unwrap()).await;
        assert_eq!(st, StatusCode::OK, "{uri}");
    }
}

#[tokio::test]
async fn plan_detail_includes_relations_and_summaries_include_tasks() {
    let e = env();
    let other = e
        .paths
        .plans
        .join("auth-gateway")
        .join("2026-11-24-audit.md");
    std::fs::write(
        &other,
        "---\nid: PLAN-auth-gateway-002\nproject: auth-gateway\ntitle: Audit log\nrelated:\n  - { to: PLAN-auth-gateway-001, type: depends_on }\n---\n## 04. Task Breakdown\n",
    )
    .unwrap();
    e.state
        .store
        .upsert_plan(&parse_plan(&std::fs::read(&other).unwrap(), &other))
        .unwrap();

    let (st, v) = call(
        &e.app,
        req("GET", &format!("/api/plans/{PLAN}"))
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v["id"], PLAN); // plan fields stay at the top level
    assert!(v["sections"].is_array());
    assert_eq!(
        v["incoming"],
        serde_json::json!([{ "from_id": "PLAN-auth-gateway-002", "from_title": "Audit log", "kind": "depends_on" }])
    );
    assert_eq!(v["outgoing_titles"], serde_json::json!({})); // PLAN-auth-gateway-000 is not indexed

    let (_, v) = call(
        &e.app,
        req("GET", "/api/plans?project=auth-gateway")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let p1 = v
        .as_array()
        .unwrap()
        .iter()
        .find(|p| p["id"] == PLAN)
        .unwrap();
    assert_eq!(
        p1["tasks"][1],
        serde_json::json!({ "id": "TASK-002", "name": "Token refresh", "status": "in_progress" })
    );
    assert!(p1["tasks"][0].get("body_md").is_none());
}

#[tokio::test]
async fn topics_use_locale_labels() {
    let e = env();
    let (_, en) = call(
        &e.app,
        req("GET", "/api/topics").body(Body::empty()).unwrap(),
    )
    .await;
    let (_, vi) = call(
        &e.app,
        req("GET", "/api/topics?locale=vi")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    let find = |v: &Value| {
        v.as_array()
            .unwrap()
            .iter()
            .find(|t| t["tag"] == "domain/security")
            .cloned()
            .expect("security topic")
    };
    let (a, b) = (find(&en), find(&vi));
    let info = &e.state.tax.tags["domain/security"];
    assert_eq!(a["label"], info.label_en.as_str());
    assert_eq!(b["label"], info.label_vi.as_str());
    assert_eq!(a["group"], info.group.as_str());
    assert!(a["plans"].as_u64().unwrap() >= 1);
}

#[tokio::test]
async fn search_endpoint_groups_results() {
    let e = env();
    let (st, v) = call(
        &e.app,
        req("GET", "/api/search?q=jwt&limit=5")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    for k in ["plans", "tasks", "topics"] {
        assert!(v[k].is_array(), "{k}");
    }
    assert!(!v["plans"].as_array().unwrap().is_empty());
}

#[tokio::test]
async fn path_traversal_rejected() {
    let e = env();
    let (st, _) = call(
        &e.app,
        req("GET", "/api/plans/..%2F..%2Fetc")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
    let r = req("PATCH", "/api/plans/..%2F..%2Fetc/tasks/TASK-001")
        .header("x-plan-atlas", "1")
        .body(Body::from(r#"{"status":"done"}"#))
        .unwrap();
    assert_eq!(call(&e.app, r).await.0, StatusCode::NOT_FOUND);

    // A plan whose DB path points outside paths.plans is never written.
    let outside = tempfile::tempdir().unwrap();
    let file = outside.path().join("evil.md");
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plans/structured.md"),
        &file,
    )
    .unwrap();
    let mut plan = parse_plan(&std::fs::read(&file).unwrap(), &file);
    plan.id = "PLAN-evil-001".into();
    e.state.store.upsert_plan(&plan).unwrap();
    let r = req("PATCH", "/api/plans/PLAN-evil-001/tasks/TASK-001")
        .header("x-plan-atlas", "1")
        .body(Body::from(r#"{"status":"done"}"#))
        .unwrap();
    assert_eq!(call(&e.app, r).await.0, StatusCode::NOT_FOUND);
    assert!(
        !std::fs::read_to_string(&file)
            .unwrap()
            .contains("- status: done")
    );
}

#[tokio::test]
async fn stream_emits_named_events() {
    let e = env();
    let resp = e
        .app
        .clone()
        .oneshot(req("GET", "/api/stream").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    assert_eq!(resp.headers()["content-type"], "text/event-stream");
    e.state
        .live
        .send(LiveEvent::PlanChanged {
            plan_id: PLAN.into(),
        })
        .unwrap();
    let mut body = resp.into_body();
    let frame = body.frame().await.unwrap().unwrap().into_data().unwrap();
    let text = String::from_utf8(frame.to_vec()).unwrap();
    assert!(text.contains("event: plan_changed"), "{text}");
    assert!(text.contains(PLAN), "{text}");
}

#[tokio::test]
async fn activity_and_task_log_endpoints() {
    use plan_atlas::events::humanize;
    use plan_atlas::store::{EventKind, HookEvent, now_ms};
    let e = env();
    let store = &e.state.store;
    store.set_session_task("s1", PLAN, "TASK-001").unwrap();
    for (agent, role, file) in [
        ("main", "main", "/p/a.rs"),
        ("agent-1", "Explore", "/p/b.rs"),
    ] {
        store
            .insert_event(&HookEvent {
                at: now_ms(),
                session_id: "s1".into(),
                agent_id: agent.into(),
                role: role.into(),
                kind: EventKind::PreTool,
                tool: Some("Read".into()),
                step: Some(humanize("Read", &serde_json::json!({ "file_path": file }))),
            })
            .unwrap();
    }

    let (st, v) = call(
        &e.app,
        req("GET", "/api/activity").body(Body::empty()).unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 1, "{v}");
    assert_eq!(v[0]["plan_id"], PLAN);
    assert_eq!(v[0]["task_id"], "TASK-001");
    let agents = v[0]["agents"].as_array().unwrap();
    assert_eq!(agents.len(), 2);
    assert_eq!(agents[1]["role"], "Explore");
    assert_eq!(agents[1]["model"], Value::Null);
    assert_eq!(agents[1]["step"]["text_vi"], "đọc b.rs");
    assert_eq!(agents[1]["step"]["icon"], "📖");
    assert!(agents[1]["at"].is_i64());

    let uri = format!("/api/plans/{PLAN}/tasks/TASK-001/log?limit=1");
    let (st, v) = call(&e.app, req("GET", &uri).body(Body::empty()).unwrap()).await;
    assert_eq!(st, StatusCode::OK);
    let log = v.as_array().unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0]["role"], "Explore", "newest first");
    assert_eq!(log[0]["step"]["text_en"], "read b.rs");

    let (st, v) = call(
        &e.app,
        req(
            "GET",
            &format!("/api/plans/{PLAN}/tasks/TASK-001/log?limit=100000"),
        )
        .body(Body::empty())
        .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::OK);
    assert_eq!(v.as_array().unwrap().len(), 2);

    let (st, _) = call(
        &e.app,
        req("GET", "/api/plans/bad%20id/tasks/TASK-001/log")
            .body(Body::empty())
            .unwrap(),
    )
    .await;
    assert_eq!(st, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn non_api_get_serves_spa_and_guards_hold() {
    let e = env();
    let resp = e
        .app
        .clone()
        .oneshot(req("GET", "/plans/x").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(resp.status(), StatusCode::OK);
    let bytes = resp.into_body().collect().await.unwrap().to_bytes();
    assert!(String::from_utf8_lossy(&bytes).contains("<title>Plan Atlas</title>"));
    let (status, _) = call(&e.app, req("GET", "/api/nope").body(Body::empty()).unwrap()).await;
    assert_eq!(status, StatusCode::NOT_FOUND);
    let bad_host = Request::builder()
        .uri("/")
        .header("host", "evil.example:7420")
        .body(Body::empty())
        .unwrap();
    let (status, _) = call(&e.app, bad_host).await;
    assert_eq!(status, StatusCode::FORBIDDEN);
}

#[test]
fn index_ready_event_serializes_with_type_tag() {
    let v = serde_json::to_value(LiveEvent::IndexReady).unwrap();
    assert_eq!(v, serde_json::json!({ "type": "index_ready" }));
}

#[test]
fn activity_changed_event_serializes_with_type_tag() {
    let v = serde_json::to_value(LiveEvent::ActivityChanged).unwrap();
    assert_eq!(v, serde_json::json!({ "type": "activity_changed" }));
}
