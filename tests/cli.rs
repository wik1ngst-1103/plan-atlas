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

#[test]
fn cli_version_prints_package_version() {
    bin()
        .arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}
