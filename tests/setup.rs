//! `setup` / `uninstall-claude`: skill, hooks merge into settings.json, MCP
//! registration. Every test uses a temp claude home and a fake `claude` CLI.

use std::fs;
use std::path::{Path, PathBuf};

use plan_atlas::setup::{
    McpStatus, SKILL_TEMPLATE, install, is_ephemeral_exe, merge_hooks, render_skill, uninstall,
};
use plan_atlas::taxonomy::Taxonomy;
use serde_json::{Value, json};

const BIN: &str = "/opt/plan atlas/bin/plan-atlas";
const EVENTS: [&str; 4] = ["PreToolUse", "SubagentStart", "SubagentStop", "SessionEnd"];

fn user_settings() -> Value {
    json!({
        "theme": "dark",
        "permissions": { "allow": ["Bash(ls:*)"] },
        "hooks": {
            "PreToolUse": [
                { "matcher": "Bash", "hooks": [{ "type": "command", "command": "/usr/bin/guard.sh" }] }
            ],
            "Stop": [ { "hooks": [{ "type": "command", "command": "notify-send done" }] } ]
        },
        "model": "opus"
    })
}

fn our_commands(v: &Value, event: &str) -> Vec<String> {
    v["hooks"][event]
        .as_array()
        .into_iter()
        .flatten()
        .flat_map(|g| g["hooks"].as_array().cloned().unwrap_or_default())
        .filter_map(|h| h["command"].as_str().map(str::to_string))
        .filter(|c| c.replace('"', "").contains("plan-atlas hook"))
        .collect()
}

#[test]
fn merge_preserves_user_hooks() {
    let merged = merge_hooks(user_settings(), Path::new(BIN));
    // Other keys survive, in their original order.
    let keys: Vec<&String> = merged.as_object().unwrap().keys().collect();
    assert_eq!(keys, ["theme", "permissions", "hooks", "model"]);
    assert_eq!(merged["permissions"], user_settings()["permissions"]);
    assert_eq!(merged["hooks"]["Stop"], user_settings()["hooks"]["Stop"]);
    // The user's PreToolUse group stays first; ours is appended.
    let pre = merged["hooks"]["PreToolUse"].as_array().unwrap();
    assert_eq!(pre.len(), 2);
    assert_eq!(pre[0], user_settings()["hooks"]["PreToolUse"][0]);
    assert_eq!(
        pre[1],
        json!({ "matcher": "*", "hooks": [{
            "type": "command", "command": format!("\"{BIN}\" hook"), "timeout": 5
        }] })
    );
    for ev in ["SubagentStart", "SubagentStop", "SessionEnd"] {
        let groups = merged["hooks"][ev].as_array().unwrap();
        assert_eq!(groups.len(), 1, "{ev}");
        assert!(groups[0].get("matcher").is_none(), "{ev} has no matcher");
        assert_eq!(our_commands(&merged, ev), [format!("\"{BIN}\" hook")]);
    }
}

#[test]
fn merge_is_idempotent() {
    let once = merge_hooks(user_settings(), Path::new(BIN));
    let twice = merge_hooks(once.clone(), Path::new(BIN));
    assert_eq!(once, twice);
    // Starting from nothing works too.
    let empty_once = merge_hooks(json!({}), Path::new(BIN));
    assert_eq!(empty_once.clone(), merge_hooks(empty_once, Path::new(BIN)));
}

#[test]
fn merge_updates_moved_binary_in_place() {
    let old = merge_hooks(user_settings(), Path::new("/old/plan-atlas"));
    let new = merge_hooks(old, Path::new(BIN));
    for ev in EVENTS {
        assert_eq!(our_commands(&new, ev), [format!("\"{BIN}\" hook")], "{ev}");
    }
    assert_eq!(new["hooks"]["PreToolUse"].as_array().unwrap().len(), 2);
}

#[test]
fn invalid_json_aborts_without_write() {
    let home = tempfile::tempdir().unwrap();
    let settings = home.path().join("settings.json");
    fs::write(&settings, "{ \"theme\": \"dark\", ").unwrap();
    let err = install(
        home.path(),
        Path::new(BIN),
        &Taxonomy::embedded_default(),
        None,
    );
    assert!(err.is_err());
    assert_eq!(
        fs::read_to_string(&settings).unwrap(),
        "{ \"theme\": \"dark\", "
    );
    let names: Vec<String> = fs::read_dir(home.path())
        .unwrap()
        .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
        .collect();
    assert_eq!(names, ["settings.json"], "no backup, no skill");
}

#[test]
fn install_writes_skill_settings_and_backup() {
    let home = tempfile::tempdir().unwrap();
    let settings = home.path().join("settings.json");
    let original = serde_json::to_string(&user_settings()).unwrap();
    fs::write(&settings, &original).unwrap();

    let r = install(
        home.path(),
        Path::new(BIN),
        &Taxonomy::embedded_default(),
        None,
    )
    .unwrap();
    assert_eq!(r.skill_path, home.path().join("skills/plan-atlas/SKILL.md"));
    assert!(r.skill_path.is_file());
    assert!(r.settings_changed);
    let backup = r.backup.expect("backup made");
    assert!(
        backup
            .file_name()
            .unwrap()
            .to_string_lossy()
            .starts_with("settings.json.bak-")
    );
    assert_eq!(fs::read_to_string(&backup).unwrap(), original);
    let text = fs::read_to_string(&settings).unwrap();
    assert!(text.ends_with("}\n") && text.contains("\n  \"theme\""));
    let v: Value = serde_json::from_str(&text).unwrap();
    assert_eq!(v, merge_hooks(user_settings(), Path::new(BIN)));
    match r.mcp {
        McpStatus::Manual { command } => {
            assert!(command.contains("mcp add --transport stdio --scope user plan-atlas --"));
            assert!(command.contains(BIN));
        }
        other => panic!("expected Manual, got {other:?}"),
    }

    // Second run: nothing to change, no new backup.
    let r2 = install(
        home.path(),
        Path::new(BIN),
        &Taxonomy::embedded_default(),
        None,
    )
    .unwrap();
    assert!(!r2.settings_changed);
    assert!(r2.backup.is_none());
}

#[test]
fn install_creates_missing_settings() {
    let dir = tempfile::tempdir().unwrap();
    let home = dir.path().join("claude");
    let r = install(&home, Path::new(BIN), &Taxonomy::embedded_default(), None).unwrap();
    assert!(r.settings_changed);
    assert!(r.backup.is_none());
    let v: Value =
        serde_json::from_str(&fs::read_to_string(home.join("settings.json")).unwrap()).unwrap();
    assert_eq!(v, merge_hooks(json!({}), Path::new(BIN)));
}

#[test]
fn uninstall_removes_only_ours() {
    let home = tempfile::tempdir().unwrap();
    let settings = home.path().join("settings.json");
    fs::write(&settings, serde_json::to_string(&user_settings()).unwrap()).unwrap();
    install(
        home.path(),
        Path::new(BIN),
        &Taxonomy::embedded_default(),
        None,
    )
    .unwrap();
    let skill_dir = home.path().join("skills/plan-atlas");
    assert!(skill_dir.is_dir());

    let r = uninstall(home.path(), None).unwrap();
    assert!(r.settings_changed);
    assert!(r.backup.is_some());
    assert!(!skill_dir.exists());
    let v: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    assert_eq!(v, user_settings());
    match r.mcp {
        McpStatus::Manual { command } => {
            assert!(command.ends_with("mcp remove --scope user plan-atlas"))
        }
        other => panic!("expected Manual, got {other:?}"),
    }
}

#[test]
fn uninstall_drops_empty_hooks_object() {
    let home = tempfile::tempdir().unwrap();
    let settings = home.path().join("settings.json");
    fs::write(&settings, "{\"theme\":\"dark\"}").unwrap();
    install(
        home.path(),
        Path::new(BIN),
        &Taxonomy::embedded_default(),
        None,
    )
    .unwrap();
    uninstall(home.path(), None).unwrap();
    let v: Value = serde_json::from_str(&fs::read_to_string(&settings).unwrap()).unwrap();
    assert_eq!(v, json!({ "theme": "dark" }));
}

#[test]
fn skill_contains_taxonomy_tags() {
    let tax = Taxonomy::embedded_default();
    let skill = render_skill(SKILL_TEMPLATE, &tax);
    assert!(!skill.contains("{{TAXONOMY}}"));
    assert!(skill.starts_with("---\nname: plan-atlas\n"));
    for (tag, info) in &tax.tags {
        let line = format!("- `{tag}` — {}: {}", info.label_en, info.desc_en);
        assert!(skill.contains(&line), "missing {line}");
    }
    // Grouped in type, domain, tech order.
    let pos = |g: &str| skill.find(&format!("`{g}/")).unwrap();
    assert!(pos("type") < pos("domain") && pos("domain") < pos("tech"));
    for tool in [
        "get_taxonomy",
        "search_plans",
        "create_plan",
        "update_plan",
        "start_task",
        "complete_task",
        "plan-atlas task start",
        "plan-atlas task done",
        "new/",
    ] {
        assert!(skill.contains(tool), "skill should mention {tool}");
    }
}

#[test]
fn skill_covers_plan_mode_repo_plans_and_deferred_tools() {
    let skill = render_skill(SKILL_TEMPLATE, &Taxonomy::embedded_default());
    for tool in [
        "list_plans",
        "search_plans",
        "get_plan",
        "get_taxonomy",
        "create_plan",
        "update_plan",
        "start_task",
        "complete_task",
    ] {
        let full = format!("mcp__plan-atlas__{tool}");
        assert!(skill.contains(&full), "skill should name {full}");
    }
    assert!(skill.contains("ToolSearch"));
    assert!(skill.contains("deferred is not unavailable"));
    assert!(skill.contains("ExitPlanMode"));
    assert!(skill.contains("write it there too AND register it"));
    assert!(skill.contains("Mention the effort in the prompt"));
    assert!(
        !skill.contains("TASK-000"),
        "example must not depend on a missing task"
    );
    assert!(!skill.contains("not in the repo"));
}

#[cfg(unix)]
fn fake_claude(dir: &Path, get_exit: i32) -> (PathBuf, PathBuf) {
    use std::os::unix::fs::PermissionsExt;
    let log = dir.join("claude.log");
    let script = dir.join("claude");
    fs::write(
        &script,
        format!(
            "#!/bin/sh\necho \"$@\" >> '{}'\nif [ \"$2\" = get ]; then echo '  Command: {BIN}'; exit {get_exit}; fi\nexit 0\n",
            log.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&script, fs::Permissions::from_mode(0o755)).unwrap();
    (script, log)
}

#[cfg(unix)]
#[test]
fn mcp_registered_when_missing() {
    let dir = tempfile::tempdir().unwrap();
    let (claude, log) = fake_claude(dir.path(), 1);
    let home = dir.path().join("claude-home");
    let r = install(
        &home,
        Path::new(BIN),
        &Taxonomy::embedded_default(),
        Some(&claude),
    )
    .unwrap();
    assert!(matches!(r.mcp, McpStatus::Registered), "{:?}", r.mcp);
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        format!(
            "mcp get plan-atlas\nmcp add --transport stdio --scope user plan-atlas -- {BIN} mcp\n"
        )
    );
}

#[cfg(unix)]
#[test]
fn mcp_already_registered_is_skipped() {
    let dir = tempfile::tempdir().unwrap();
    let (claude, log) = fake_claude(dir.path(), 0);
    let home = dir.path().join("claude-home");
    let r = install(
        &home,
        Path::new(BIN),
        &Taxonomy::embedded_default(),
        Some(&claude),
    )
    .unwrap();
    assert!(matches!(r.mcp, McpStatus::AlreadyRegistered), "{:?}", r.mcp);
    assert_eq!(fs::read_to_string(&log).unwrap(), "mcp get plan-atlas\n");

    let r = uninstall(&home, Some(&claude)).unwrap();
    assert!(matches!(r.mcp, McpStatus::Removed), "{:?}", r.mcp);
    assert!(
        fs::read_to_string(&log)
            .unwrap()
            .ends_with("mcp remove --scope user plan-atlas\n")
    );
}

#[cfg(unix)]
#[test]
fn mcp_add_failure_is_reported_not_fatal() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let claude = dir.path().join("claude");
    fs::write(&claude, "#!/bin/sh\necho boom >&2\nexit 1\n").unwrap();
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    let home = dir.path().join("claude-home");
    let r = install(
        &home,
        Path::new(BIN),
        &Taxonomy::embedded_default(),
        Some(&claude),
    )
    .unwrap();
    match r.mcp {
        McpStatus::Failed { command, error } => {
            assert!(command.contains("mcp add"));
            assert!(error.contains("boom"));
        }
        other => panic!("expected Failed, got {other:?}"),
    }
    assert!(home.join("settings.json").is_file());
}

fn cli(dir: &Path) -> assert_cmd::Command {
    let mut c = assert_cmd::Command::cargo_bin("plan-atlas").unwrap();
    c.env("CLAUDE_CONFIG_DIR", dir.join("claude"))
        .env("PLAN_ATLAS_HOME", dir.join("atlas"))
        .env("PATH", dir.join("empty-path"));
    c
}

#[test]
fn cli_setup_without_tty_requires_yes() {
    let dir = tempfile::tempdir().unwrap();
    cli(dir.path())
        .arg("setup")
        .write_stdin("")
        .assert()
        .failure()
        .stderr(predicates::str::contains("--yes"));
    assert!(!dir.path().join("claude").exists());
}

#[test]
fn cli_setup_and_uninstall_with_yes() {
    let dir = tempfile::tempdir().unwrap();
    cli(dir.path())
        .args(["setup", "--yes"])
        .assert()
        .success()
        .stdout(predicates::str::contains(
            "mcp add --transport stdio --scope user plan-atlas",
        ));
    assert!(
        dir.path()
            .join("claude/skills/plan-atlas/SKILL.md")
            .is_file()
    );
    let config = fs::read_to_string(dir.path().join("atlas/config.toml")).unwrap();
    assert!(config.contains("setup_done = true"), "{config}");

    cli(dir.path())
        .args(["uninstall-claude", "--yes"])
        .assert()
        .success();
    assert!(!dir.path().join("claude/skills/plan-atlas").exists());
    let v: Value =
        serde_json::from_str(&fs::read_to_string(dir.path().join("claude/settings.json")).unwrap())
            .unwrap();
    assert_eq!(v, json!({}));
}

#[cfg(unix)]
#[test]
fn stale_mcp_registration_is_replaced() {
    use std::os::unix::fs::PermissionsExt;
    let dir = tempfile::tempdir().unwrap();
    let log = dir.path().join("claude.log");
    let claude = dir.path().join("claude");
    fs::write(
        &claude,
        format!(
            "#!/bin/sh\necho \"$@\" >> '{}'\nif [ \"$2\" = get ]; then echo '  Command: /old/Cellar/plan-atlas/0.1.0/bin/plan-atlas'; echo '  Args: mcp'; fi\nexit 0\n",
            log.display()
        ),
    )
    .unwrap();
    fs::set_permissions(&claude, fs::Permissions::from_mode(0o755)).unwrap();
    let home = dir.path().join("claude-home");
    let r = install(
        &home,
        Path::new(BIN),
        &Taxonomy::embedded_default(),
        Some(&claude),
    )
    .unwrap();
    assert!(matches!(r.mcp, McpStatus::Updated), "{:?}", r.mcp);
    assert_eq!(
        fs::read_to_string(&log).unwrap(),
        format!(
            "mcp get plan-atlas\nmcp remove --scope user plan-atlas\nmcp add --transport stdio --scope user plan-atlas -- {BIN} mcp\n"
        )
    );
}

#[test]
fn setup_does_not_overwrite_invalid_config() {
    let dir = tempfile::tempdir().unwrap();
    let atlas = dir.path().join("atlas");
    fs::create_dir_all(&atlas).unwrap();
    let bad = "port = 7420\nlocale = \"vi\"\nthis is not toml\n";
    fs::write(atlas.join("config.toml"), bad).unwrap();
    cli(dir.path())
        .args(["setup", "--yes"])
        .assert()
        .success()
        .stderr(predicates::str::contains("config.toml"));
    assert_eq!(fs::read_to_string(atlas.join("config.toml")).unwrap(), bad);
    cli(dir.path())
        .args(["uninstall-claude", "--yes"])
        .assert()
        .success();
    assert_eq!(fs::read_to_string(atlas.join("config.toml")).unwrap(), bad);
}

#[test]
fn ephemeral_exe_detection() {
    let temp = Path::new("/tmp");
    for p in [
        "/home/u/.npm/_npx/3f2a/node_modules/plan-atlas/bin/plan-atlas",
        r"C:\Users\u\AppData\Local\npm-cache\_npx\3f2a\node_modules\plan-atlas.exe",
        "/tmp/plan-atlas-xyz/plan-atlas",
    ] {
        assert!(is_ephemeral_exe(Path::new(p), temp), "{p}");
    }
    for p in [
        "/usr/local/bin/plan-atlas",
        "/home/u/.cargo/bin/plan-atlas",
        "/usr/lib/node_modules/plan-atlas/bin/plan-atlas",
        "/opt/homebrew/bin/plan-atlas",
        "/tmpfoo/plan-atlas",
    ] {
        assert!(!is_ephemeral_exe(Path::new(p), temp), "{p}");
    }
}

#[cfg(unix)]
#[test]
fn cli_setup_refuses_binary_in_temp_dir() {
    let dir = tempfile::tempdir().unwrap();
    let copy = dir.path().join("plan-atlas");
    fs::copy(assert_cmd::cargo::cargo_bin("plan-atlas"), &copy).unwrap();
    assert_cmd::Command::new(&copy)
        .args(["setup", "--yes"])
        .env("CLAUDE_CONFIG_DIR", dir.path().join("claude"))
        .env("PLAN_ATLAS_HOME", dir.path().join("atlas"))
        .env("PATH", dir.path().join("empty-path"))
        .env("TMPDIR", dir.path())
        .assert()
        .failure()
        .stderr(predicates::str::contains("npm i -g plan-atlas"));
    assert!(!dir.path().join("claude").exists());
}
