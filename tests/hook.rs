use std::io::Cursor;
use std::path::Path;
use std::sync::Arc;
use std::time::{Duration, Instant};

use assert_cmd::Command;
use plan_atlas::config::Paths;
use plan_atlas::events::live::{PollerConfig, spawn_live_poller, spawn_live_poller_with};
use plan_atlas::events::{LiveEvent, humanize, run_hook};
use plan_atlas::store::{EventKind, HookEvent, Store, now_ms};
use plan_atlas::taxonomy::Taxonomy;
use tempfile::TempDir;
use tokio::sync::broadcast;

const PLAN: &str = "PLAN-auth-gateway-001";

fn setup() -> (TempDir, Paths, Store) {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(dir.path().to_path_buf());
    paths.ensure().unwrap();
    let store = open(&paths);
    (dir, paths, store)
}

fn open(paths: &Paths) -> Store {
    Store::open(&paths.db, Arc::new(Taxonomy::embedded_default())).unwrap()
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/hooks")
            .join(format!("{name}.json")),
    )
    .unwrap()
}

fn feed(paths: &Paths, names: &[&str]) {
    for name in names {
        run_hook(Cursor::new(fixture(name)), paths);
    }
}

fn event(session: &str, agent: &str, kind: EventKind, at: i64) -> HookEvent {
    let step = (kind == EventKind::PreTool)
        .then(|| humanize("Read", &serde_json::json!({"file_path": "/p/a.rs"})));
    HookEvent {
        at,
        session_id: session.into(),
        agent_id: agent.into(),
        role: "main".into(),
        kind,
        tool: step.as_ref().map(|_| "Read".to_string()),
        step,
    }
}

#[test]
fn garbage_stdin_exits_quietly() {
    let (_d, paths, store) = setup();
    let t = Instant::now();
    run_hook(Cursor::new(b"{not json".to_vec()), &paths);
    run_hook(Cursor::new(Vec::new()), &paths);
    run_hook(Cursor::new(b"{\"hook_event_name\":42}".to_vec()), &paths);
    assert!(t.elapsed() < Duration::from_millis(700));
    assert_eq!(store.max_event_id().unwrap(), 0);
}

#[test]
fn locked_db_returns_within_deadline() {
    let (_d, paths, _store) = setup();
    let locker = rusqlite::Connection::open(&paths.db).unwrap();
    locker
        .execute_batch("BEGIN IMMEDIATE; INSERT INTO sessions VALUES ('x','p','t',0);")
        .unwrap();
    let t = Instant::now();
    run_hook(Cursor::new(fixture("pre_tool_read")), &paths);
    let took = t.elapsed();
    assert!(took < Duration::from_millis(300), "took {took:?}");
    locker.execute_batch("ROLLBACK").unwrap();
}

#[test]
fn start_task_links_session() {
    let (_d, paths, store) = setup();
    feed(&paths, &["pre_tool_start_task", "pre_tool_read"]);
    let log = store.task_log(PLAN, "TASK-001", 30).unwrap();
    assert_eq!(log.len(), 1, "{log:?}");
    assert_eq!(log[0].role, "main");
    assert_eq!(log[0].step.text_en, "read token.rs");
    assert_eq!(log[0].step.text_vi, "đọc token.rs");
    assert!(store.task_log(PLAN, "TASK-002", 30).unwrap().is_empty());
}

#[test]
fn unlinked_events_are_stored_but_not_shown() {
    let (_d, paths, store) = setup();
    feed(&paths, &["pre_tool_read"]);
    assert_eq!(store.max_event_id().unwrap(), 1);
    assert!(store.live_agents().unwrap().is_empty());
    assert!(store.task_log(PLAN, "TASK-001", 30).unwrap().is_empty());
}

#[test]
fn subagent_stop_removes_agent() {
    let (_d, paths, store) = setup();
    feed(
        &paths,
        &[
            "pre_tool_start_task",
            "subagent_start",
            "pre_tool_subagent_grep",
        ],
    );
    let live = store.live_agents().unwrap();
    assert_eq!(live.len(), 1);
    assert_eq!(
        (live[0].plan_id.as_str(), live[0].task_id.as_str()),
        (PLAN, "TASK-001")
    );
    let sub = live[0]
        .agents
        .iter()
        .find(|a| a.agent_id == "agent-abc123")
        .expect("subagent is live");
    assert_eq!(sub.role, "Explore");
    assert_eq!(sub.model, None);
    assert_eq!(sub.step.text_en, "search the code");

    feed(&paths, &["subagent_stop"]);
    let live = store.live_agents().unwrap();
    assert!(
        live.iter()
            .flat_map(|t| &t.agents)
            .all(|a| a.agent_id != "agent-abc123"),
        "{live:?}"
    );
}

#[test]
fn complete_task_and_session_end_unlink() {
    let (_d, paths, store) = setup();
    feed(
        &paths,
        &[
            "pre_tool_start_task",
            "pre_tool_complete_task",
            "pre_tool_read",
        ],
    );
    let log = store.task_log(PLAN, "TASK-001", 30).unwrap();
    assert_eq!(
        log.len(),
        1,
        "only the complete_task step is linked: {log:?}"
    );
    assert_eq!(log[0].step.text_en, "update the plan");
    assert!(store.live_agents().unwrap().is_empty());

    feed(
        &paths,
        &["pre_tool_start_task", "pre_tool_read", "session_end"],
    );
    assert_eq!(store.task_log(PLAN, "TASK-001", 30).unwrap().len(), 2);
    assert!(store.live_agents().unwrap().is_empty());
}

#[test]
fn bash_task_start_links_session() {
    let (_d, paths, store) = setup();
    feed(&paths, &["pre_tool_bash_start", "pre_tool_bash_test"]);
    let log = store.task_log(PLAN, "TASK-002", 30).unwrap();
    assert_eq!(log.len(), 1);
    assert_eq!(log[0].step.text_vi, "chạy kiểm thử");
    let live = store.live_agents().unwrap();
    assert_eq!(live[0].task_id, "TASK-002");
    assert_eq!(live[0].agents[0].agent_id, "main");
}

#[test]
fn live_window_log_order_and_prune() {
    let (_d, _paths, store) = setup();
    store.set_session_task("s", PLAN, "TASK-001").unwrap();
    let now = now_ms();
    let day = 86_400_000;
    store
        .insert_event(&event("s", "old", EventKind::PreTool, now - 10 * day))
        .unwrap();
    store
        .insert_event(&event("s", "stale", EventKind::PreTool, now - 121_000))
        .unwrap();
    store
        .insert_event(&event("s", "fresh", EventKind::PreTool, now - 1_000))
        .unwrap();
    store
        .insert_event(&event("s", "nostep", EventKind::SubagentStart, now))
        .unwrap();
    let live = store.live_agents().unwrap();
    let ids: Vec<_> = live[0].agents.iter().map(|a| a.agent_id.as_str()).collect();
    assert_eq!(ids, ["fresh"]);

    let log = store.task_log(PLAN, "TASK-001", 2).unwrap();
    assert_eq!(log.len(), 2);
    assert!(log[0].at > log[1].at, "newest first");

    assert_eq!(store.prune_events(7).unwrap(), 1);
    assert_eq!(store.task_log(PLAN, "TASK-001", 30).unwrap().len(), 2);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn poller_broadcasts_new_steps_only() {
    let (_d, paths, store) = setup();
    store.set_session_task("s", PLAN, "TASK-001").unwrap();
    // History before the poller starts is not replayed.
    store
        .insert_event(&event("s", "main", EventKind::PreTool, now_ms()))
        .unwrap();
    let store = Arc::new(store);
    let (tx, mut rx) = broadcast::channel(16);
    let handle = spawn_live_poller(store.clone(), tx);
    tokio::time::sleep(Duration::from_millis(700)).await;

    // A different connection (like the hook process) writes a linked step.
    let other = paths.clone();
    tokio::task::spawn_blocking(move || {
        feed(
            &other,
            &[
                "pre_tool_start_task",
                "subagent_start",
                "pre_tool_subagent_grep",
            ],
        )
    })
    .await
    .unwrap();

    let ev = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .expect("an AgentStep arrives")
        .unwrap();
    match ev {
        LiveEvent::AgentStep {
            plan_id,
            task_id,
            agent,
        } => {
            assert_eq!((plan_id.as_str(), task_id.as_str()), (PLAN, "TASK-001"));
            assert_eq!(agent.agent_id, "agent-abc123");
            assert_eq!(agent.step.text_en, "search the code");
        }
        other => panic!("unexpected {other:?}"),
    }
    // The new agent also changes the live set; nothing else is broadcast.
    let ev = tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .unwrap()
        .unwrap();
    assert!(matches!(ev, LiveEvent::ActivityChanged), "{ev:?}");
    assert!(
        rx.try_recv().is_err(),
        "unlinked start_task and SubagentStart are not broadcast"
    );
    handle.abort();
}

async fn next_event(rx: &mut broadcast::Receiver<LiveEvent>) -> LiveEvent {
    tokio::time::timeout(Duration::from_secs(3), rx.recv())
        .await
        .expect("an event arrives")
        .unwrap()
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn poller_broadcasts_activity_changed_on_subagent_stop() {
    let (_d, paths, store) = setup();
    let store = Arc::new(store);
    let (tx, mut rx) = broadcast::channel(16);
    let handle = spawn_live_poller(store.clone(), tx);
    tokio::time::sleep(Duration::from_millis(700)).await;

    let other = paths.clone();
    let feed_bg = |names: &'static [&'static str]| {
        let p = other.clone();
        tokio::task::spawn_blocking(move || feed(&p, names))
    };
    feed_bg(&[
        "pre_tool_start_task",
        "subagent_start",
        "pre_tool_subagent_grep",
    ])
    .await
    .unwrap();
    assert!(matches!(
        next_event(&mut rx).await,
        LiveEvent::AgentStep { .. }
    ));
    assert!(matches!(
        next_event(&mut rx).await,
        LiveEvent::ActivityChanged
    ));
    let agents = |store: &Store| -> Vec<String> {
        store
            .live_agents()
            .unwrap()
            .iter()
            .flat_map(|t| t.agents.iter().map(|a| a.agent_id.clone()))
            .collect()
    };
    assert!(agents(&store).contains(&"agent-abc123".to_string()));

    // SubagentStop writes no step, only a change to the live set.
    feed_bg(&["subagent_stop"]).await.unwrap();
    assert!(matches!(
        next_event(&mut rx).await,
        LiveEvent::ActivityChanged
    ));
    assert!(!agents(&store).contains(&"agent-abc123".to_string()));
    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn poller_broadcasts_activity_changed_on_idle_expiry() {
    let (_d, _paths, store) = setup();
    store.set_session_task("s", PLAN, "TASK-001").unwrap();
    // Live now, expires from the 120 s window in about 0.8 s, with no write.
    store
        .insert_event(&event("s", "main", EventKind::PreTool, now_ms() - 119_200))
        .unwrap();
    assert_eq!(store.live_agents().unwrap().len(), 1);
    let store = Arc::new(store);
    let (tx, mut rx) = broadcast::channel(16);
    let cfg = PollerConfig {
        poll: Duration::from_millis(50),
        activity_every: Duration::from_millis(200),
        ..PollerConfig::default()
    };
    let handle = spawn_live_poller_with(store.clone(), tx, cfg);
    assert!(matches!(
        next_event(&mut rx).await,
        LiveEvent::ActivityChanged
    ));
    assert!(store.live_agents().unwrap().is_empty());
    // Nothing live any more: no further rechecks or broadcasts.
    tokio::time::sleep(Duration::from_millis(500)).await;
    assert!(rx.try_recv().is_err());
    handle.abort();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn poller_prunes_old_events_periodically() {
    let (_d, _paths, store) = setup();
    store
        .insert_event(&event(
            "s",
            "old",
            EventKind::PreTool,
            now_ms() - 8 * 86_400_000,
        ))
        .unwrap();
    let store = Arc::new(store);
    let (tx, _rx) = broadcast::channel(16);
    let cfg = PollerConfig {
        poll: Duration::from_millis(50),
        prune_every: Duration::from_millis(100),
        ..PollerConfig::default()
    };
    let handle = spawn_live_poller_with(store.clone(), tx, cfg);
    let deadline = Instant::now() + Duration::from_secs(3);
    while store.max_event_id().unwrap() != 0 && Instant::now() < deadline {
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    assert_eq!(store.max_event_id().unwrap(), 0, "old event pruned");
    handle.abort();
}

#[test]
fn exit_code_always_success() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("not-yet-created");
    let hook = |input: Vec<u8>| {
        Command::cargo_bin("plan-atlas")
            .unwrap()
            .args(["hook", "--unexpected-flag"])
            .env("PLAN_ATLAS_HOME", &home)
            .write_stdin(input)
            .assert()
            .code(0)
            .stdout("");
    };
    // Garbage, empty, and a valid payload against a home that does not exist
    // yet: always exit 0, never print. Creating a fresh database may take
    // longer than the hook's 200 ms budget on a slow CI runner, in which case
    // the hook rightly drops the event instead of slowing Claude Code down.
    for input in [b"{not json".to_vec(), Vec::new(), fixture("pre_tool_read")] {
        hook(input);
    }
    // With the database already in place, a valid payload is recorded.
    let store = open(&Paths::from_home(home.clone()));
    let before = store.max_event_id().unwrap();
    hook(fixture("pre_tool_read"));
    assert_eq!(
        store.max_event_id().unwrap(),
        before + 1,
        "valid payload was recorded"
    );
}
