use std::fs;
use std::path::PathBuf;

use plan_atlas::plan::{
    TaskStatus, WriteError, next_plan_id, parse_plan, plan_file_name, set_task_status, write_atomic,
};

fn fixture(name: &str) -> Vec<u8> {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/plans")
        .join(name);
    fs::read(p).unwrap()
}

fn lines(b: &[u8]) -> Vec<&[u8]> {
    b.split_inclusive(|&c| c == b'\n').collect()
}

#[test]
fn status_line_replaced() {
    let src = fixture("structured.md");
    let out = set_task_status(&src, "TASK-002", TaskStatus::Done).unwrap();
    let (a, b) = (lines(&src), lines(&out));
    assert_eq!(a.len(), b.len());
    let diff: Vec<_> = a.iter().zip(&b).filter(|(x, y)| x != y).collect();
    assert_eq!(diff.len(), 1);
    assert_eq!(diff[0].1, b"- status: done\n");
    let plan = parse_plan(&out, std::path::Path::new("x.md"));
    assert_eq!(
        plan.tasks
            .iter()
            .find(|t| t.id == "TASK-002")
            .unwrap()
            .status,
        TaskStatus::Done
    );
}

#[test]
fn status_line_inserted_when_missing() {
    let src = b"# P\n### TASK-001: A\n- complexity: 2\n\nbody\n### TASK-002: B\n";
    let out = set_task_status(src, "TASK-001", TaskStatus::InProgress).unwrap();
    assert_eq!(
        out,
        b"# P\n### TASK-001: A\n- status: in_progress\n- complexity: 2\n\nbody\n### TASK-002: B\n"
            .to_vec()
    );
}

#[test]
fn preserves_crlf() {
    let src = fixture("crlf.md");
    for id in ["TASK-001", "TASK-002"] {
        let out = set_task_status(&src, id, TaskStatus::Review).unwrap();
        assert!(
            !out.windows(2)
                .enumerate()
                .any(|(i, w)| w[1] == b'\n' && w[0] != b'\r' && {
                    let _ = i;
                    true
                })
        );
        assert!(String::from_utf8_lossy(&out).contains("- status: review\r\n"));
    }
}

#[test]
fn preserves_non_utf8_bytes() {
    let src = b"### TASK-001: A\n- complexity: 1\n\nbad \xff\xfe bytes\n";
    let out = set_task_status(src, "TASK-001", TaskStatus::Done).unwrap();
    assert_eq!(
        out,
        b"### TASK-001: A\n- status: done\n- complexity: 1\n\nbad \xff\xfe bytes\n".to_vec()
    );
}

#[test]
fn ignores_fenced_headings_and_prefix_collisions() {
    let src = b"```\n### TASK-001: fake\n```\n### TASK-0011: other\n### TASK-001: real\n";
    let out = set_task_status(src, "TASK-001", TaskStatus::Done).unwrap();
    assert_eq!(
        out,
        b"```\n### TASK-001: fake\n```\n### TASK-0011: other\n### TASK-001: real\n- status: done"
            .to_vec()
    );
}

#[test]
fn unknown_task_errors() {
    let src = fixture("structured.md");
    let err = set_task_status(&src, "TASK-999", TaskStatus::Done).unwrap_err();
    assert!(matches!(err, WriteError::TaskNotFound(ref id) if id == "TASK-999"));
    assert!(err.to_string().contains("TASK-999"));
}

#[test]
fn next_id_increments() {
    let ex = |v: &[&str]| v.iter().map(|s| s.to_string()).collect::<Vec<_>>();
    assert_eq!(
        next_plan_id("a", &ex(&["PLAN-a-001", "PLAN-a-007"])),
        "PLAN-a-008"
    );
    assert_eq!(next_plan_id("a", &ex(&[])), "PLAN-a-001");
    assert_eq!(
        next_plan_id("a", &ex(&["PLAN-b-050", "PLAN-a-x1", "PLAN-a-002"])),
        "PLAN-a-003"
    );
    assert_eq!(next_plan_id("a", &ex(&["PLAN-a-999"])), "PLAN-a-1000");
}

#[test]
fn file_name_slug() {
    assert_eq!(
        plan_file_name("2026-11-20", "Chuyển session sang JWT + RBAC"),
        "2026-11-20-chuyen-session-sang-jwt-rbac.md"
    );
    let long = plan_file_name("2026-11-20", &"word ".repeat(30));
    let slug = long
        .strip_prefix("2026-11-20-")
        .unwrap()
        .strip_suffix(".md")
        .unwrap();
    assert!(slug.len() <= 60 && !slug.ends_with('-'));
    assert_eq!(plan_file_name("2026-11-20", "!!!"), "2026-11-20-plan.md");
}

#[test]
fn write_atomic_replaces_and_leaves_no_temp() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.md");
    fs::write(&p, b"old").unwrap();
    write_atomic(&p, b"new").unwrap();
    assert_eq!(fs::read(&p).unwrap(), b"new");
    assert_eq!(fs::read_dir(dir.path()).unwrap().count(), 1);
}

#[test]
fn concurrent_writes_never_corrupt() {
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.md");
    fs::write(&p, b"init").unwrap();
    let contents: Vec<Vec<u8>> = (0..8).map(|t| vec![b'a' + t as u8; 50_000]).collect();
    std::thread::scope(|s| {
        for c in &contents {
            let p = &p;
            s.spawn(move || {
                for _ in 0..20 {
                    write_atomic(p, c).unwrap();
                }
            });
        }
    });
    let got = fs::read(&p).unwrap();
    assert!(contents.contains(&got));
    let names: Vec<_> = fs::read_dir(dir.path())
        .unwrap()
        .map(|e| e.unwrap().file_name())
        .collect();
    assert_eq!(names.len(), 1, "leftover files: {names:?}");
}

#[cfg(unix)]
#[test]
fn preserves_permissions() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let p = dir.path().join("a.md");
    fs::write(&p, b"old").unwrap();
    fs::set_permissions(&p, fs::Permissions::from_mode(0o640)).unwrap();
    write_atomic(&p, b"new").unwrap();
    assert_eq!(
        fs::metadata(&p).unwrap().permissions().mode() & 0o777,
        0o640
    );
    assert_eq!(fs::read(&p).unwrap(), b"new");
}

#[cfg(unix)]
#[test]
fn writes_through_symlink() {
    let dir = tempfile::tempdir().unwrap();
    let real = dir.path().join("real.md");
    let link = dir.path().join("link.md");
    fs::write(&real, b"old").unwrap();
    std::os::unix::fs::symlink(&real, &link).unwrap();
    write_atomic(&link, b"new").unwrap();
    assert!(
        fs::symlink_metadata(&link)
            .unwrap()
            .file_type()
            .is_symlink()
    );
    assert_eq!(fs::read(&real).unwrap(), b"new");
}
