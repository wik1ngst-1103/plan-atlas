use std::fs;
use std::path::{Path, PathBuf};

use plan_atlas::config::Paths;
use plan_atlas::import::{
    ImportReport, Upload, import_claude_plans, import_paths, import_uploads, infer_project,
    infer_projects, infer_projects_with,
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

/// (imported, skipped, unsorted, failed)
fn counts(r: &ImportReport) -> (usize, usize, usize, usize) {
    (r.imported, r.skipped, r.unsorted, r.failed)
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
    assert_eq!(counts(&r), (3, 0, 1, 0));
    assert_eq!(r.plans.len(), 3);
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
    assert_eq!(counts(&r), (0, 3, 0, 0));
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
        counts(&import_claude_plans(&empty, &e.paths).unwrap()),
        (0, 0, 0, 0)
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

/// A transcript line where Claude writes `path` with the Write tool, as Claude
/// Code records it.
fn write_line(path: &str) -> String {
    format!(
        "{{\"type\":\"assistant\",\"message\":{{\"role\":\"assistant\",\"content\":[{{\"type\":\"tool_use\",\"name\":\"Write\",\"input\":{{\"file_path\":\"{path}\",\"content\":\"# Plan\"}}}}]}}}}\n"
    )
}

fn set_mtime(f: &Path, secs: u64) {
    let t = std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(secs);
    fs::File::options()
        .write(true)
        .open(f)
        .unwrap()
        .set_modified(t)
        .unwrap();
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
        text.push_str(&write_line(&format!("/home/u/.claude/plans/plan-{i}.md")));
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

/// Regression: a newer session that merely lists plan names (e.g. `ls
/// ~/.claude/plans` output, or a chat message quoting them) must not claim
/// them; only the session where Claude wrote or edited the plan file counts.
#[test]
fn only_plan_file_writes_decide_the_project() {
    let tmp = tempfile::tempdir().unwrap();
    let claude = tmp.path().join("claude");
    let home_plans = "/home/u/.claude/plans";
    let mk_repo = |name: &str| {
        let r = tmp.path().join("work").join(name);
        fs::create_dir_all(r.join(".git")).unwrap();
        r
    };
    let (origin, noisy, editor) = (mk_repo("origin"), mk_repo("noisy"), mk_repo("editor"));

    // Oldest: the real author session writes a.md.
    let d = claude.join("projects/-work-origin");
    fs::create_dir_all(&d).unwrap();
    let f = d.join("s.jsonl");
    fs::write(
        &f,
        format!(
            "{{\"type\":\"user\",\"cwd\":\"{}\"}}\n{}",
            origin.display(),
            write_line(&format!("{home_plans}/a.md"))
        ),
    )
    .unwrap();
    set_mtime(&f, 1_000_000);

    // Middle: another session edits b.md with the Edit tool (counts too).
    let d = claude.join("projects/-work-editor");
    fs::create_dir_all(&d).unwrap();
    let f = d.join("s.jsonl");
    fs::write(
        &f,
        format!(
            "{{\"type\":\"assistant\",\"cwd\":\"{}\",\"message\":{{\"content\":[{{\"type\":\"tool_use\",\"name\":\"Edit\",\"input\":{{\"file_path\":\"{home_plans}/b.md\",\"old_string\":\"x\",\"new_string\":\"y\"}}}}]}}}}\n",
            editor.display()
        ),
    )
    .unwrap();
    set_mtime(&f, 1_000_100);

    // Newest: a session that only lists every plan name, in a tool result,
    // a Read of the plan, and plain chat text.
    let d = claude.join("projects/-work-noisy");
    fs::create_dir_all(&d).unwrap();
    let f = d.join("s.jsonl");
    fs::write(
        &f,
        format!(
            concat!(
                "{{\"type\":\"user\",\"cwd\":\"{cwd}\",\"message\":{{\"content\":[{{\"type\":\"tool_result\",\"content\":\"a.md\\nb.md\\nc.md\"}}]}}}}\n",
                "{{\"type\":\"assistant\",\"cwd\":\"{cwd}\",\"message\":{{\"content\":[{{\"type\":\"tool_use\",\"name\":\"Read\",\"input\":{{\"file_path\":\"{p}/a.md\"}}}}]}}}}\n",
                "{{\"type\":\"assistant\",\"cwd\":\"{cwd}\",\"message\":{{\"content\":[{{\"type\":\"text\",\"text\":\"see {p}/c.md\"}}]}}}}\n"
            ),
            cwd = noisy.display(),
            p = home_plans
        ),
    )
    .unwrap();
    set_mtime(&f, 1_000_200);

    let names = ["a.md", "b.md", "c.md"].map(String::from);
    let found = infer_projects(&names, &claude);
    assert_eq!(found.get("a.md").map(String::as_str), Some("origin"));
    assert_eq!(found.get("b.md").map(String::as_str), Some("editor"));
    assert_eq!(found.get("c.md"), None, "never written anywhere → unsorted");
    // A file of the same name outside a `plans` directory does not count.
    assert_eq!(infer_project("x.md", &claude), None);
}

// ---- import_paths -------------------------------------------------------

fn write(path: &Path, text: &str) {
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    fs::write(path, text).unwrap();
}

/// A plain directory (no git) for sources, an empty Claude home, a store.
fn loose_env() -> (tempfile::TempDir, PathBuf, PathBuf, Paths) {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("notes");
    let claude = tmp.path().join("claude");
    fs::create_dir_all(&claude).unwrap();
    let paths = Paths::from_home(tmp.path().join("atlas"));
    paths.ensure().unwrap();
    (tmp, src, claude, paths)
}

#[test]
fn import_single_file_goes_unsorted() {
    let (_t, src, claude, paths) = loose_env();
    let f = src.join("alpha.md");
    write(&f, "# Alpha plan\n\nBody.\n");
    let r = import_paths(std::slice::from_ref(&f), None, &claude, &paths).unwrap();
    assert_eq!(counts(&r), (1, 0, 1, 0));
    let text = fs::read_to_string(paths.unsorted.join("alpha.md")).unwrap();
    assert!(text.contains("title: Alpha plan"));
    assert!(text.contains("id: PLAN-unsorted-001"));
    let canon = fs::canonicalize(&f).unwrap();
    assert!(text.contains(&format!("imported_from: {}", canon.display())));
    assert_eq!(r.plans[0].id, "PLAN-unsorted-001");
    assert_eq!(r.plans[0].project, "unsorted");
    assert_eq!(r.plans[0].title, "Alpha plan");
    assert_eq!(fs::read_to_string(&f).unwrap(), "# Alpha plan\n\nBody.\n");
}

#[test]
fn import_directory_tree_skips_hidden_and_non_md() {
    let (_t, src, claude, paths) = loose_env();
    write(&src.join("a.md"), "# A\n");
    write(&src.join("deep/er/b.md"), "# B\n");
    write(&src.join("deep/C.MD"), "# C\n");
    write(&src.join("notes.txt"), "not a plan");
    write(&src.join(".hidden.md"), "# Hidden\n");
    write(&src.join(".secret/d.md"), "# D\n");
    write(&src.join("node_modules/pkg/e.md"), "# E\n");
    write(&src.join("target/f.md"), "# F\n");
    write(&src.join(".git/g.md"), "# G\n");
    let r = import_paths(std::slice::from_ref(&src), Some("Side"), &claude, &paths).unwrap();
    assert_eq!(counts(&r), (3, 0, 0, 0));
    let mut titles: Vec<&str> = r.plans.iter().map(|p| p.title.as_str()).collect();
    titles.sort();
    assert_eq!(titles, ["A", "B", "C"]);
    let mut ids: Vec<&str> = r.plans.iter().map(|p| p.id.as_str()).collect();
    ids.sort();
    assert_eq!(ids, ["PLAN-side-001", "PLAN-side-002", "PLAN-side-003"]);
    assert!(paths.plans.join("side/b.md").is_file());
}

#[test]
fn explicit_project_is_sanitized_and_beats_git() {
    let (t, _src, claude, paths) = loose_env();
    let repo = t.path().join("work/repo-x");
    fs::create_dir_all(repo.join(".git")).unwrap();
    let f = repo.join("docs/plan.md");
    write(&f, "# P\n");
    let r = import_paths(&[f], Some("../Weird Name!"), &claude, &paths).unwrap();
    assert_eq!(counts(&r), (1, 0, 0, 0));
    assert!(paths.plans.join("weird-name/plan.md").is_file());
    assert_eq!(r.plans[0].project, "weird-name");
}

#[test]
fn git_repo_infers_project() {
    let (t, _src, claude, paths) = loose_env();
    let repo = t.path().join("work/Nsvideo");
    fs::create_dir_all(repo.join(".git")).unwrap();
    write(&repo.join("docs/plans/roadmap.md"), "# Roadmap\n");
    let r = import_paths(&[repo.join("docs")], None, &claude, &paths).unwrap();
    assert_eq!(counts(&r), (1, 0, 0, 0));
    let text = fs::read_to_string(paths.plans.join("nsvideo/roadmap.md")).unwrap();
    assert!(text.contains("id: PLAN-nsvideo-001") && text.contains("project: nsvideo"));
}

#[test]
fn transcript_inference_used_outside_git() {
    let e = setup();
    // A copy of demo-plan.md outside any repo: the session that wrote
    // `…/plans/demo-plan.md` decides.
    let f = e.paths.home.parent().unwrap().join("loose/demo-plan.md");
    write(&f, "# Demo copy\n");
    let r = import_paths(&[f], None, &e.claude, &e.paths).unwrap();
    assert_eq!(counts(&r), (1, 0, 0, 0));
    assert!(e.paths.plans.join("my-repo/demo-plan.md").is_file());
}

#[test]
fn import_paths_is_idempotent() {
    let (_t, src, claude, paths) = loose_env();
    write(&src.join("a.md"), "# A\n");
    write(&src.join("b/b.md"), "# B\n");
    let dirs = std::slice::from_ref(&src);
    let first = import_paths(dirs, None, &claude, &paths).unwrap();
    assert_eq!(counts(&first), (2, 0, 2, 0));
    let again = import_paths(dirs, None, &claude, &paths).unwrap();
    assert_eq!(counts(&again), (0, 2, 0, 0));
    assert!(again.plans.is_empty());
    // Changed content updates the same copy and keeps its id.
    write(&src.join("a.md"), "# A\n\nEdited.\n");
    let third = import_paths(dirs, None, &claude, &paths).unwrap();
    assert_eq!(counts(&third), (1, 1, 1, 0));
    let text = fs::read_to_string(paths.unsorted.join("a.md")).unwrap();
    assert!(text.contains("Edited.") && text.contains("PLAN-unsorted-001"));
    assert!(!paths.unsorted.join("a-2.md").exists());
}

#[test]
fn store_files_are_skipped_not_copied() {
    let (_t, src, claude, paths) = loose_env();
    write(&src.join("a.md"), "# A\n");
    write(&src.join("b.md"), "# B\n");
    import_paths(std::slice::from_ref(&src), None, &claude, &paths).unwrap();
    let before = fs::read_dir(&paths.unsorted).unwrap().count();
    // The whole store, and one file inside it again (counted once).
    let one = paths.unsorted.join("a.md");
    let r = import_paths(&[paths.plans.clone(), one], None, &claude, &paths).unwrap();
    assert_eq!(counts(&r), (0, 2, 0, 0));
    assert_eq!(fs::read_dir(&paths.unsorted).unwrap().count(), before);
}

#[test]
fn bad_paths_fail_without_stopping_the_rest() {
    let (_t, src, claude, paths) = loose_env();
    write(&src.join("good.md"), "# Good\n");
    write(&src.join("note.txt"), "x");
    let args = [
        src.join("missing.md"),
        src.join("note.txt"),
        src.join("good.md"),
    ];
    let r = import_paths(&args, None, &claude, &paths).unwrap();
    assert_eq!(counts(&r), (1, 0, 1, 2));
    assert_eq!(r.failures.len(), 2);
    assert!(r.failures[0].source.ends_with("missing.md"));
    assert!(!r.failures[0].error.is_empty());
}

#[cfg(unix)]
#[test]
fn unreadable_file_in_tree_is_non_fatal() {
    use std::os::unix::fs::PermissionsExt;
    let (_t, src, claude, paths) = loose_env();
    write(&src.join("a.md"), "# A\n");
    let bad = src.join("b.md");
    write(&bad, "# B\n");
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o000)).unwrap();
    if fs::read(&bad).is_ok() {
        return; // running as root
    }
    let r = import_paths(std::slice::from_ref(&src), None, &claude, &paths).unwrap();
    fs::set_permissions(&bad, fs::Permissions::from_mode(0o644)).unwrap();
    assert_eq!((r.imported, r.failed), (1, 1));
}

#[test]
fn same_file_via_claude_import_and_path_is_one_copy() {
    let e = setup();
    import_claude_plans(&e.claude, &e.paths).unwrap();
    let r = import_paths(&[e.claude.join("plans")], None, &e.claude, &e.paths).unwrap();
    assert_eq!(counts(&r), (0, 3, 0, 0));
}

// ---- import_uploads -----------------------------------------------------

fn up(path: &str, text: &str) -> Upload {
    Upload {
        path: path.to_string(),
        bytes: text.as_bytes().to_vec(),
    }
}

#[test]
fn uploads_import_with_project_and_are_idempotent() {
    let (_t, _src, _claude, paths) = loose_env();
    let files = [
        up("plans/one.md", "# One\n"),
        up("plans/sub/two.md", "# Two\n"),
    ];
    let r = import_uploads(&files, Some("NS Video"), &paths).unwrap();
    assert_eq!(counts(&r), (2, 0, 0, 0));
    let text = fs::read_to_string(paths.plans.join("ns-video/one.md")).unwrap();
    assert!(text.contains("imported_from: upload:plans/one.md"));
    assert!(text.contains("id: PLAN-ns-video-001"));
    assert!(paths.plans.join("ns-video/two.md").is_file());
    let again = import_uploads(&files, Some("NS Video"), &paths).unwrap();
    assert_eq!(counts(&again), (0, 2, 0, 0));
}

#[test]
fn uploads_default_to_unsorted_and_names_are_safe() {
    let (_t, _src, _claude, paths) = loose_env();
    let files = [up("../../evil/..\\Nasty Name!.md", "# N\n")];
    let r = import_uploads(&files, None, &paths).unwrap();
    assert_eq!(counts(&r), (1, 0, 1, 0));
    let names: Vec<String> = fs::read_dir(&paths.unsorted)
        .unwrap()
        .flatten()
        .map(|e| e.file_name().to_string_lossy().into_owned())
        .filter(|n| n.ends_with(".md"))
        .collect();
    assert_eq!(names, ["nasty-name.md"]);
    assert!(!paths.home.join("evil").exists());
}
