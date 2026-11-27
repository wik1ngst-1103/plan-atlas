use std::path::{Path, PathBuf};

use plan_atlas::plan::{Effort, Model, RelationKind, Task, TaskStatus, parse_plan, validate};
use plan_atlas::taxonomy::Taxonomy;

fn fixture(name: &str) -> (Vec<u8>, PathBuf) {
    let p = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/plans")
        .join(name);
    (std::fs::read(&p).unwrap(), p)
}

fn parse(name: &str) -> plan_atlas::plan::Plan {
    let (b, p) = fixture(name);
    parse_plan(&b, &p)
}

fn codes(ws: &[plan_atlas::plan::Warning]) -> Vec<&str> {
    ws.iter().map(|w| w.code.as_str()).collect()
}

#[test]
fn parses_structured_fixture() {
    let p = parse("structured.md");
    assert_eq!(p.id, "PLAN-auth-gateway-001");
    assert_eq!(p.project, "auth-gateway");
    assert_eq!(p.title, "Migrate sessions to JWT + RBAC");
    assert_eq!(p.created.as_deref(), Some("2026-11-20"));
    assert!(p.structured);
    assert_eq!(p.sections.len(), 5);
    assert_eq!(p.sections[2].children.len(), 3);
    assert_eq!(p.sections[2].children[0].anchor, "current-architecture");
    assert_eq!(p.sections[0].anchor, "01-overview");
    assert_eq!(p.tasks.len(), 3);
    assert_eq!(p.tasks[1].status, TaskStatus::InProgress);
    assert_eq!(p.tasks[1].depends_on, vec!["TASK-001"]);
    assert_eq!(p.tasks[0].tags, vec!["tech/jwt"]);
    assert_eq!(p.tasks[0].body_md, "Free markdown description.");
    assert_eq!(p.related[0].kind, RelationKind::DependsOn);
    assert_eq!(p.related[0].to, "PLAN-auth-gateway-000");
    assert!(p.warnings.is_empty(), "{:?}", p.warnings);
    assert_eq!(p.content_hash.len(), 64);
}

#[test]
fn resolved_model_defaults() {
    let p = parse("structured.md");
    let t: &Task = &p.tasks[1];
    assert_eq!(t.model, None);
    assert_eq!(t.resolved_model(), (Model::Opus, Effort::Medium));
    assert_eq!(p.tasks[2].resolved_model(), (Model::Sonnet, Effort::Low));
}

#[test]
fn unstructured_has_sections_no_tasks() {
    let p = parse("unstructured.md");
    assert!(!p.structured);
    assert!(p.tasks.is_empty());
    assert_eq!(p.title, "Some freeform plan");
    assert_eq!(p.sections.len(), 2);
    assert_eq!(p.sections[1].title, "Steps");
    assert_eq!(p.sections[1].children[0].title, "Sub step");
}

#[test]
fn crlf_parses_same_as_lf() {
    let a = parse("structured.md");
    let mut b = parse("crlf.md");
    assert_ne!(a.content_hash, b.content_hash);
    b.content_hash = a.content_hash.clone();
    b.source_path = a.source_path.clone();
    assert_eq!(a, b);
}

#[test]
fn duplicate_task_ids_warn() {
    let p = parse("dup_task.md");
    let w = validate(&p, &Taxonomy::embedded_default());
    assert!(codes(&w).contains(&"duplicate_task_id"), "{w:?}");
}

#[test]
fn missing_frontmatter_warns_not_panics() {
    let p = parse("no_frontmatter.md");
    assert!(codes(&p.warnings).contains(&"missing_frontmatter"));
    assert_eq!(p.title, "Plan Without Frontmatter");
    assert_eq!(p.id, "no_frontmatter");
    assert!(!p.structured);

    let p = parse_plan(b"just text", Path::new("/x/notes.md"));
    assert_eq!(p.title, "notes");
}

#[test]
fn unknown_tag_warns() {
    let src = std::fs::read_to_string(fixture("structured.md").1)
        .unwrap()
        .replace("tech/jwt]\nrelated", "tech/xyz]\nrelated");
    let p = parse_plan(src.as_bytes(), Path::new("x.md"));
    let w = validate(&p, &Taxonomy::embedded_default());
    assert!(
        w.iter()
            .any(|w| w.code == "unknown_tag" && w.message.contains("tech/xyz"))
    );
    let ok = validate(&parse("structured.md"), &Taxonomy::embedded_default());
    assert!(!codes(&ok).contains(&"unknown_tag"), "{ok:?}");
}

#[test]
fn invalid_frontmatter_and_non_utf8_and_metadata() {
    let p = parse_plan(b"---\nid: [oops\n---\n## 04. Tasks\n", Path::new("a.md"));
    assert!(codes(&p.warnings).contains(&"invalid_frontmatter"));
    assert!(!p.structured);

    let p = parse_plan(b"# T\n\xff\xfe bad\n", Path::new("a.md"));
    assert!(codes(&p.warnings).contains(&"non_utf8"));

    let src = "---\nid: A\n---\n## 04. Tasks\n### TASK-001: x\n- complexity: 9\n- status: weird\n\nbody\n";
    let p = parse_plan(src.as_bytes(), Path::new("a.md"));
    let n = p
        .warnings
        .iter()
        .filter(|w| w.code == "invalid_task_metadata")
        .count();
    assert_eq!(n, 2);
    assert_eq!(p.tasks[0].status, TaskStatus::Todo);
    assert_eq!(p.tasks[0].body_md, "body");
}

#[test]
fn missing_section_warns_for_structured() {
    let src = "---\nid: A\n---\n## 01. Overview\n## 04. Tasks\n";
    let p = parse_plan(src.as_bytes(), Path::new("a.md"));
    assert!(p.structured);
    let w = validate(&p, &Taxonomy::embedded_default());
    assert_eq!(
        codes(&w)
            .iter()
            .filter(|c| **c == "missing_section")
            .count(),
        3
    );
}

#[test]
fn serde_shapes() {
    let p = parse("structured.md");
    let v = serde_json::to_value(&p.tasks[1]).unwrap();
    assert_eq!(v["status"], "in_progress");
    let r = serde_json::to_value(&p.related[0]).unwrap();
    assert_eq!(r["kind"], "depends_on");
}

#[test]
fn partially_bad_frontmatter_keeps_good_fields() {
    let src = "---\nid: 42\nproject: p\ntitle: T\ntags: tech/jwt\nrelated:\n  - { to: A, type: nope }\n  - { to: B, type: extends }\ncreated: 2026-11-20\n---\n## 04. Tasks\n";
    let p = parse_plan(src.as_bytes(), Path::new("a.md"));
    assert_eq!(p.id, "42");
    assert_eq!(p.project, "p");
    assert_eq!(p.title, "T");
    assert_eq!(p.created.as_deref(), Some("2026-11-20"));
    assert_eq!(p.tags, vec!["tech/jwt"]);
    assert_eq!(p.related.len(), 1);
    assert_eq!(p.related[0].to, "B");
    assert!(p.structured);
    let w: Vec<_> = p
        .warnings
        .iter()
        .filter(|w| w.code == "invalid_frontmatter")
        .collect();
    assert_eq!(w.len(), 1);
    assert!(w[0].message.contains("related"));
}
