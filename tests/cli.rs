use assert_cmd::Command;
use predicates::prelude::*;

fn bin() -> Command {
    Command::cargo_bin("plan-atlas").unwrap()
}

#[test]
fn cli_help_lists_subcommands() {
    bin().arg("--help").assert().success().stdout(
        predicate::str::contains("mcp")
            .and(predicate::str::contains("hook"))
            .and(predicate::str::contains("setup"))
            .and(predicate::str::contains("import")),
    );
}

/// `plan-atlas` with every home pointed into `tmp`.
fn isolated(tmp: &std::path::Path) -> Command {
    let mut c = bin();
    c.env("HOME", tmp.join("home"))
        .env("PLAN_ATLAS_HOME", tmp.join("atlas"))
        .env("CLAUDE_CONFIG_DIR", tmp.join("claude"));
    c
}

#[test]
fn cli_import_path_with_project() {
    let tmp = tempfile::tempdir().unwrap();
    let src = tmp.path().join("notes");
    std::fs::create_dir_all(src.join("sub")).unwrap();
    std::fs::write(src.join("a.md"), "# A\n").unwrap();
    std::fs::write(src.join("sub/b.md"), "# B\n").unwrap();
    std::fs::write(src.join("skip.txt"), "x").unwrap();
    isolated(tmp.path())
        .args(["import", src.to_str().unwrap(), "--project", "Side"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Imported 2, skipped 0 (0 unsorted)",
        ));
    assert!(tmp.path().join("atlas/plans/side/a.md").is_file());
    assert!(tmp.path().join("atlas/plans/side/b.md").is_file());
    // Re-run: nothing new.
    isolated(tmp.path())
        .args(["import", src.to_str().unwrap(), "--project", "Side"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 0, skipped 2"));
}

#[test]
fn cli_import_reports_failures_on_stderr() {
    let tmp = tempfile::tempdir().unwrap();
    let missing = tmp.path().join("nope.md");
    isolated(tmp.path())
        .args(["import", missing.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("1 failed"))
        .stderr(predicate::str::contains("nope.md"));
}

#[test]
fn cli_import_without_path_uses_claude_config_dir() {
    let tmp = tempfile::tempdir().unwrap();
    let plans = tmp.path().join("claude/plans");
    std::fs::create_dir_all(&plans).unwrap();
    std::fs::write(plans.join("x.md"), "# X\n").unwrap();
    isolated(tmp.path())
        .arg("import")
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Imported 1, skipped 0 (1 unsorted)",
        ));
    assert!(tmp.path().join("atlas/plans/unsorted/x.md").is_file());
}

#[test]
fn cli_version_prints_package_version() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
