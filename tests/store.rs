use std::path::{Path, PathBuf};
use std::sync::Arc;

use plan_atlas::plan::{Plan, Relation, RelationKind, Section, Task, TaskStatus};
use plan_atlas::store::{PlanFilter, Store, TaskFilter};
use plan_atlas::taxonomy::Taxonomy;
use rusqlite::Connection;
use tempfile::TempDir;

fn store() -> (TempDir, Store) {
    let dir = tempfile::tempdir().unwrap();
    let s = Store::open(
        &dir.path().join("i.db"),
        Arc::new(Taxonomy::embedded_default()),
    )
    .unwrap();
    (dir, s)
}

fn task(id: &str, name: &str, status: TaskStatus, tags: &[&str]) -> Task {
    Task {
        id: id.into(),
        name: name.into(),
        complexity: Some(2),
        model: None,
        effort: None,
        depends_on: vec![],
        status,
        tags: tags.iter().map(|s| s.to_string()).collect(),
        body_md: format!("body of {name}"),
    }
}

fn plan(id: &str, project: &str, tags: &[&str], related: &[(&str, RelationKind)]) -> Plan {
    Plan {
        id: id.into(),
        project: project.into(),
        title: format!("Title {id}"),
        tags: tags.iter().map(|s| s.to_string()).collect(),
        related: related
            .iter()
            .map(|(to, kind)| Relation {
                to: to.to_string(),
                kind: *kind,
            })
            .collect(),
        created: Some("2026-11-01".into()),
        sections: vec![Section {
            level: 2,
            title: "Overview".into(),
            anchor: "overview".into(),
            body_md: "overview text".into(),
            children: vec![],
        }],
        tasks: vec![
            task("T1", "First", TaskStatus::Done, &["tech/jwt"]),
            task("T2", "Second", TaskStatus::Todo, &[]),
        ],
        structured: true,
        source_path: PathBuf::from(format!("/plans/{id}.md")),
        content_hash: format!("hash-{id}"),
        warnings: vec![],
    }
}

#[test]
fn fts5_available() {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch(
        "CREATE VIRTUAL TABLE t USING fts5(a, tokenize='unicode61 remove_diacritics 2')",
    )
    .unwrap();
}

#[test]
fn store_is_send_sync() {
    fn check<T: Send + Sync>() {}
    check::<Store>();
}

#[test]
fn reopen_keeps_data() {
    let dir = tempfile::tempdir().unwrap();
    let db = dir.path().join("i.db");
    let tax = Arc::new(Taxonomy::embedded_default());
    Store::open(&db, tax.clone())
        .unwrap()
        .upsert_plan(&plan("a-001", "p", &[], &[]))
        .unwrap();
    let s = Store::open(&db, tax).unwrap();
    assert!(s.get_plan("a-001").unwrap().is_some());
}

#[test]
fn upsert_then_get_roundtrip() {
    let (_d, s) = store();
    let p = plan("auth-001", "demo", &["domain/security"], &[]);
    s.upsert_plan(&p).unwrap();
    assert_eq!(s.get_plan("auth-001").unwrap().unwrap(), p);
    assert!(s.get_plan("nope").unwrap().is_none());

    let plans = s.list_plans(&PlanFilter::default()).unwrap();
    assert_eq!(plans.len(), 1);
    assert_eq!((plans[0].task_count, plans[0].done_count), (2, 1));
    assert_eq!(plans[0].tags, vec!["domain/security"]);

    let projects = s.list_projects().unwrap();
    assert_eq!(projects.len(), 1);
    assert_eq!(
        (
            projects[0].plan_count,
            projects[0].task_count,
            projects[0].done_count
        ),
        (1, 2, 1)
    );

    let by_tag = s
        .list_plans(&PlanFilter {
            project: None,
            tag: Some("tech/jwt".into()),
        })
        .unwrap();
    assert_eq!(by_tag.len(), 1); // via task tag
    let tasks = s
        .list_tasks(&TaskFilter {
            project: None,
            tag: Some("tech/jwt".into()),
        })
        .unwrap();
    assert_eq!(tasks.len(), 1);
    assert_eq!(tasks[0].task.id, "T1");
    assert_eq!(
        s.find_task_path("auth-001").unwrap().unwrap(),
        Path::new("/plans/auth-001.md")
    );
    assert_eq!(s.plan_ids_for_project("demo").unwrap(), vec!["auth-001"]);
}

#[test]
fn upsert_same_hash_is_noop() {
    let (_d, s) = store();
    let mut p = plan("auth-001", "demo", &[], &[]);
    s.upsert_plan(&p).unwrap();
    let (changes, ver) = (s.total_changes(), s.data_version().unwrap());
    s.upsert_plan(&p).unwrap();
    assert_eq!(s.total_changes(), changes);
    assert_eq!(s.data_version().unwrap(), ver);

    p.content_hash = "other".into();
    p.title = "Changed".into();
    s.upsert_plan(&p).unwrap();
    assert!(s.total_changes() > changes);
    assert!(s.data_version().unwrap() > ver);
    assert_eq!(s.get_plan("auth-001").unwrap().unwrap().title, "Changed");
    assert_eq!(s.list_tasks(&TaskFilter::default()).unwrap().len(), 2);
}

#[test]
fn delete_by_path_removes_tasks_and_tags() {
    let (_d, s) = store();
    let p = plan(
        "auth-001",
        "demo",
        &["domain/security"],
        &[("x", RelationKind::RelatesTo)],
    );
    s.upsert_plan(&p).unwrap();
    s.delete_by_path(&p.source_path).unwrap();
    assert!(s.get_plan("auth-001").unwrap().is_none());
    assert!(s.list_tasks(&TaskFilter::default()).unwrap().is_empty());
    assert!(s.topics().unwrap().is_empty());
    assert!(s.list_projects().unwrap().is_empty());
    let r = s.search("title", 10).unwrap();
    assert!(r.plans.is_empty() && r.tasks.is_empty());
    s.delete_by_path(&p.source_path).unwrap(); // idempotent
}

#[test]
fn graph_has_declared_and_shared_edges() {
    let (_d, s) = store();
    s.upsert_plan(&plan(
        "auth-001",
        "demo",
        &["domain/security", "tech/jwt", "type/feature"],
        &[],
    ))
    .unwrap();
    s.upsert_plan(&plan(
        "auth-002",
        "demo",
        &["domain/security", "tech/jwt"],
        &[("auth-001", RelationKind::DependsOn)],
    ))
    .unwrap();
    s.upsert_plan(&plan(
        "rag-002",
        "demo",
        &["tech/jwt", "type/feature"],
        &[("ghost-9", RelationKind::RelatesTo)],
    ))
    .unwrap();
    let g = s.graph().unwrap();
    assert_eq!(g.nodes.len(), 3);
    let v = serde_json::to_value(&g.edges).unwrap();
    assert_eq!(
        v,
        serde_json::json!([
            {"from": "auth-002", "to": "auth-001", "kind": {"type": "declared", "value": "depends_on"}},
            {"from": "auth-001", "to": "rag-002", "kind": {"type": "shared_topics", "value": ["tech/jwt"]}},
            {"from": "auth-002", "to": "rag-002", "kind": {"type": "shared_topics", "value": ["tech/jwt"]}},
        ])
    );
}

#[test]
fn search_without_diacritics() {
    let (_d, s) = store();
    let mut p = plan("auth-001", "demo", &["domain/security"], &[]);
    p.sections[0].body_md = "Tính năng đăng nhập bằng email".into();
    p.tasks[1].body_md = "Viết màn hình Đăng Nhập".into();
    p.tasks[0].tags.clear(); // tech/jwt's label also contains "đăng nhập"
    s.upsert_plan(&p).unwrap();
    let mut other = plan("other-001", "demo", &["domain/data"], &[]);
    other.tasks[0].tags.clear();
    s.upsert_plan(&other).unwrap();

    let r = s.search("dang nhap", 10).unwrap();
    assert_eq!(r.plans.len(), 1);
    assert_eq!(r.plans[0].id, "auth-001");
    assert_eq!(r.tasks.len(), 1);
    assert_eq!(r.tasks[0].task.id, "T2");
    assert_eq!(s.search("ĐĂNG nh", 10).unwrap().plans.len(), 1); // prefix + case

    // taxonomy label "Bảo mật" is indexed, and topics match it too
    let r = s.search("bao mat", 10).unwrap();
    assert_eq!(r.plans.len(), 1);
    assert_eq!(r.plans[0].id, "auth-001");
    assert!(r.topics.iter().any(|t| t.tag == "domain/security"));
    assert!(s.search("security", 10).unwrap().plans.len() == 1);

    assert!(s.search("   ", 10).unwrap().plans.is_empty());
    assert!(s.search("\"", 10).unwrap().plans.is_empty());
    assert_eq!(s.search("title", 1).unwrap().plans.len(), 1); // limit
}
