use std::fs;
use std::path::{Path, PathBuf};

use plan_atlas::config::Paths;
use plan_atlas::import::{ImportReport, import_claude_plans, infer_project};
use plan_atlas::plan::parse_plan;

fn copy_dir(from: &Path, to: &Path) {
    fs::create_dir_all(to).unwrap();
    for e in fs::read_dir(from).unwrap().flatten() {
        let t = to.join(e.file_name());
        if e.path().is_dir() {
            copy_dir(&e.path(), &t);
        } else {
            fs::copy(e.path(), t).unwrap();
        }
    }
}

struct Env {
    _tmp: tempfile::TempDir,
    claude: PathBuf,
    paths: Paths,
}

fn setup() -> Env {
    let tmp = tempfile::tempdir().unwrap();
    let claude = tmp.path().join("claude");
    copy_dir(
        &Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/claude_home"),
        &claude,
    );
    // Fake git root + nested cwd.
    let root = tmp.path().join("work/My Repo");
    fs::create_dir_all(root.join(".git")).unwrap();
    fs::create_dir_all(root.join("sub")).unwrap();
    let jsonl = claude.join("projects/-tmp-demo/session.jsonl");
    let text = fs::read_to_string(&jsonl)
        .unwrap()
        .replace("{{CWD}}", root.join("sub").to_str().unwrap());
    fs::write(&jsonl, text).unwrap();
    let paths = Paths::from_home(tmp.path().join("atlas"));
    paths.ensure().unwrap();
    Env {
        _tmp: tmp,
        claude,
        paths,
    }
}

#[test]
fn imports_and_infers_project() {
    let e = setup();
    assert_eq!(
        infer_project("demo-plan.md", &e.claude).as_deref(),
        Some("my-repo")
    );
    let r = import_claude_plans(&e.claude, &e.paths).unwrap();
    assert_eq!(
        r,
        ImportReport {
            imported: 3,
            skipped: 0,
            unsorted: 1
        }
    );
    let dest = e.paths.plans.join("my-repo/demo-plan.md");
    let bytes = fs::read(&dest).unwrap();
    let p = parse_plan(&bytes, &dest);
    assert_eq!(p.id, "PLAN-my-repo-001");
    assert_eq!(p.project, "my-repo");
    let text = String::from_utf8(bytes).unwrap();
    assert!(text.contains("imported_from:") && text.contains("imported_hash:"));
    assert!(text.contains("# Demo plan"));
    // Existing frontmatter fields are kept, new ones added.
    let fm = fs::read_to_string(e.paths.plans.join("my-repo/with-fm.md")).unwrap();
    assert!(fm.contains("status: draft") && fm.contains("PLAN-my-repo-002"));
}

#[test]
fn unknown_goes_unsorted() {
    let e = setup();
    import_claude_plans(&e.claude, &e.paths).unwrap();
    assert!(infer_project("mystery.md", &e.claude).is_none());
    let dest = e.paths.unsorted.join("mystery.md");
    let text = fs::read_to_string(dest).unwrap();
    assert!(text.contains("title: Mystery plan"));
    assert!(text.contains("PLAN-unsorted-001"));
}

#[test]
fn idempotent_second_run() {
    let e = setup();
    import_claude_plans(&e.claude, &e.paths).unwrap();
    let r = import_claude_plans(&e.claude, &e.paths).unwrap();
    assert_eq!(
        r,
        ImportReport {
            imported: 0,
            skipped: 3,
            unsorted: 0
        }
    );
    // Changed source overwrites the same destination, keeping its id.
    let src = e.claude.join("plans/demo-plan.md");
    fs::write(&src, "# Demo plan\n\nEdited.\n").unwrap();
    let r = import_claude_plans(&e.claude, &e.paths).unwrap();
    assert_eq!((r.imported, r.skipped), (1, 2));
    let text = fs::read_to_string(e.paths.plans.join("my-repo/demo-plan.md")).unwrap();
    assert!(text.contains("Edited.") && text.contains("PLAN-my-repo-001"));
    assert!(!e.paths.plans.join("my-repo/demo-plan-2.md").exists());
}

#[test]
fn never_touches_source() {
    let e = setup();
    let files: Vec<_> = fs::read_dir(e.claude.join("plans"))
        .unwrap()
        .flatten()
        .map(|f| f.path())
        .collect();
    let before: Vec<_> = files
        .iter()
        .map(|p| {
            (
                fs::read(p).unwrap(),
                fs::metadata(p).unwrap().modified().unwrap(),
            )
        })
        .collect();
    import_claude_plans(&e.claude, &e.paths).unwrap();
    import_claude_plans(&e.claude, &e.paths).unwrap();
    let after: Vec<_> = files
        .iter()
        .map(|p| {
            (
                fs::read(p).unwrap(),
                fs::metadata(p).unwrap().modified().unwrap(),
            )
        })
        .collect();
    assert_eq!(before, after);
}

#[test]
fn missing_plans_dir_is_zero_report() {
    let e = setup();
    let empty = e.paths.home.join("nothing");
    assert_eq!(
        import_claude_plans(&empty, &e.paths).unwrap(),
        ImportReport::default()
    );
}
