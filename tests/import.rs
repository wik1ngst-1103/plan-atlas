use std::fs;
use std::path::{Path, PathBuf};

use plan_atlas::config::Paths;
use plan_atlas::import::{
    ImportReport, import_claude_plans, infer_project, infer_projects, infer_projects_with,
};
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
            unsorted: 1,
            failed: 0
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
            unsorted: 0,
            failed: 0
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

#[cfg(unix)]
#[test]
fn unreadable_source_is_skipped_not_fatal() {
    use std::os::unix::fs::PermissionsExt;
    let e = setup();
    let bad = e.claude.join("plans/mystery.md");
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&bad).is_ok() {
        // Running as root: permissions are not enforced.
        fs::set_permissions(&bad, fs::Permissions::from_mode(0o644)).unwrap();
        return;
    }
    let r = import_claude_plans(&e.claude, &e.paths).unwrap();
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!((r.imported, r.failed), (2, 1));
}

#[test]
fn invalid_source_frontmatter_not_duplicated() {
    let e = setup();
    let orig = "---\n: [unclosed\n---\n# Broken\n\nBody.\n";
    fs::write(e.claude.join("plans/broken.md"), orig).unwrap();
    import_claude_plans(&e.claude, &e.paths).unwrap();
    let dest = e.paths.unsorted.join("broken.md");
    let text = fs::read_to_string(&dest).unwrap();
    assert!(text.starts_with("---\n"));
    // Exactly one leading block: the second `---` line closes it, the only
    // other `---` lines are inside the fenced copy of the original.
    let p = parse_plan(text.as_bytes(), &dest);
    assert!(p.warnings.iter().all(|w| w.code != "invalid_frontmatter"));
    assert!(p.id.starts_with("PLAN-unsorted-"));
    assert!(text.contains("<!-- original frontmatter (invalid) -->"));
    assert!(text.contains("```yaml\n---\n: [unclosed\n---\n```"));
    assert!(text.contains("# Broken"));
    assert!(
        !text[4..]
            .split("\n---\n")
            .next()
            .unwrap()
            .contains("unclosed")
    );
}

/// `n` transcripts in separate project dirs; transcript `i` mentions
/// `plan-{i}.md` from repo `repo-{i}`. Older index → older mtime.
fn many_transcripts(n: usize) -> (tempfile::TempDir, PathBuf) {
    let tmp = tempfile::tempdir().unwrap();
    let claude = tmp.path().join("claude");
    for i in 0..n {
        let repo = tmp.path().join(format!("work/repo-{i}"));
        fs::create_dir_all(repo.join(".git")).unwrap();
        let dir = claude.join(format!("projects/-work-repo-{i}"));
        fs::create_dir_all(&dir).unwrap();
        let mut text = String::new();
        text.push_str(&format!(
            "{{\"cwd\":\"{}\",\"type\":\"user\"}}\n",
            repo.display()
        ));
        for _ in 0..50 {
            text.push_str("{\"type\":\"assistant\",\"message\":\"filler line\"}\n");
        }
        text.push_str(&format!(
            "{{\"type\":\"tool\",\"path\":\"/home/u/.claude/plans/plan-{i}.md\"}}\n"
        ));
        let f = dir.join("s.jsonl");
        fs::write(&f, text).unwrap();
        let t = std::time::SystemTime::UNIX_EPOCH
            + std::time::Duration::from_secs(1_000_000 + i as u64);
        fs::File::options()
            .write(true)
            .open(&f)
            .unwrap()
            .set_modified(t)
            .unwrap();
    }
    (tmp, claude)
}

#[test]
fn batch_inference_reads_each_transcript_at_most_once() {
    let n = 30;
    let (_tmp, claude) = many_transcripts(n);
    let names: Vec<String> = (0..n).map(|i| format!("plan-{i}.md")).collect();
    let mut opens: Vec<PathBuf> = Vec::new();
    let found = infer_projects_with(&names, &claude, |p| {
        opens.push(p.to_path_buf());
        fs::File::open(p).ok().map(std::io::BufReader::new)
    });
    assert_eq!(found.len(), n);
    for i in 0..n {
        assert_eq!(found[&format!("plan-{i}.md")], format!("repo-{i}"));
    }
    let mut unique = opens.clone();
    unique.sort();
    unique.dedup();
    assert_eq!(unique.len(), opens.len(), "a transcript was read twice");
    assert_eq!(opens.len(), n);
}

#[test]
fn batch_inference_stops_when_all_resolved_newest_first() {
    let n = 10;
    let (_tmp, claude) = many_transcripts(n);
    let mut opens = 0;
    let found = infer_projects_with(&["plan-9.md".to_string()], &claude, |p| {
        opens += 1;
        fs::File::open(p).ok().map(std::io::BufReader::new)
    });
    assert_eq!(found["plan-9.md"], "repo-9");
    assert_eq!(
        opens, 1,
        "newest transcript resolves it; nothing else is read"
    );
    assert!(infer_projects(&[], &claude).is_empty());
}
