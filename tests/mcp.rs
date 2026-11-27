//! Drives `plan-atlas mcp` over newline-delimited JSON-RPC on stdin/stdout,
//! and `plan-atlas task start|done` as a subprocess.

use std::io::{BufRead, BufReader, Write};
use std::path::{Path, PathBuf};
use std::process::{Child, ChildStdin, Command, Stdio};
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use plan_atlas::config::Paths;
use plan_atlas::index::Indexer;
use plan_atlas::store::Store;
use plan_atlas::taxonomy::Taxonomy;
use serde_json::{Value, json};
use tempfile::TempDir;

const TIMEOUT: Duration = Duration::from_secs(10);

fn bin() -> PathBuf {
    assert_cmd::cargo::cargo_bin("plan-atlas")
}

struct Env {
    dir: TempDir,
}

impl Env {
    /// Temp dir holding `home/` (PLAN_ATLAS_HOME) and a repo `My Repo/` with
    /// `.git/` and a nested `src/` subdirectory.
    fn new() -> Env {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("home")).unwrap();
        std::fs::create_dir_all(dir.path().join("My Repo/.git")).unwrap();
        std::fs::create_dir_all(dir.path().join("My Repo/src")).unwrap();
        Env { dir }
    }
    fn home(&self) -> PathBuf {
        self.dir.path().join("home")
    }
    fn repo_src(&self) -> PathBuf {
        self.dir.path().join("My Repo/src")
    }
    fn plans(&self) -> PathBuf {
        self.home().join("plans")
    }
}

struct Mcp {
    child: Child,
    stdin: ChildStdin,
    lines: Receiver<String>,
    next_id: u64,
}

impl Mcp {
    fn start(env: &Env) -> Mcp {
        let mut child = Command::new(bin())
            .arg("mcp")
            .env("PLAN_ATLAS_HOME", env.home())
            .current_dir(env.repo_src())
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .unwrap();
        let stdin = child.stdin.take().unwrap();
        let stdout = child.stdout.take().unwrap();
        let (tx, rx) = channel();
        std::thread::spawn(move || {
            for line in BufReader::new(stdout).lines() {
                let Ok(line) = line else { break };
                if tx.send(line).is_err() {
                    break;
                }
            }
        });
        let mut mcp = Mcp {
            child,
            stdin,
            lines: rx,
            next_id: 1,
        };
        let init = mcp.request(
            "initialize",
            json!({
                "protocolVersion": "2025-06-18",
                "capabilities": {},
                "clientInfo": { "name": "test", "version": "0" }
            }),
        );
        assert!(init.get("result").is_some(), "initialize failed: {init}");
        mcp.send(json!({ "jsonrpc": "2.0", "method": "notifications/initialized" }));
        mcp
    }

    fn send(&mut self, msg: Value) {
        writeln!(self.stdin, "{msg}").unwrap();
        self.stdin.flush().unwrap();
    }

    /// Send a request and wait for the response with the same id.
    fn request(&mut self, method: &str, params: Value) -> Value {
        let id = self.next_id;
        self.next_id += 1;
        self.send(json!({ "jsonrpc": "2.0", "id": id, "method": method, "params": params }));
        loop {
            let line = self
                .lines
                .recv_timeout(TIMEOUT)
                .expect("no response from MCP server");
            let v: Value = serde_json::from_str(&line).unwrap();
            if v.get("id") == Some(&json!(id)) {
                return v;
            }
        }
    }

    /// `tools/call`; returns (is_error, parsed JSON of the first text content
    /// or the raw text if it is not JSON).
    fn call(&mut self, name: &str, args: Value) -> (bool, Value) {
        let resp = self.request("tools/call", json!({ "name": name, "arguments": args }));
        let result = resp
            .get("result")
            .unwrap_or_else(|| panic!("tools/call {name} failed: {resp}"));
        let is_error = result["isError"].as_bool().unwrap_or(false);
        let text = result["content"][0]["text"].as_str().unwrap_or_default();
        let body = serde_json::from_str(text).unwrap_or_else(|_| Value::String(text.to_string()));
        (is_error, body)
    }
}

impl Drop for Mcp {
    fn drop(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }
}

const FULL_PLAN: &str = "---
title: Migrate sessions to JWT
tags: [type/feature]
owner: alice
---
## 01. Overview

Move sessions to JWT.

## 02. Requirements & Scope

Scope.

## 03. Technical Analysis

Analysis.

## 04. Task Breakdown

### TASK-001: Authentication Service
- complexity: 3
- status: todo

Build it.

## 05. Risks & Rollback

Rollback.
";

const MISSING_SECTION_PLAN: &str = "---
title: Half a plan
---
## 01. Overview

Text.

## 04. Task Breakdown

### TASK-001: Only task
- status: todo
";

fn md_files(dir: &Path) -> Vec<PathBuf> {
    let mut out: Vec<PathBuf> = std::fs::read_dir(dir)
        .map(|rd| rd.flatten().map(|e| e.path()).collect())
        .unwrap_or_default();
    out.retain(|p| p.extension().is_some_and(|e| e == "md"));
    out.sort();
    out
}

#[test]
fn tools_list_has_all_eight() {
    let env = Env::new();
    let mut mcp = Mcp::start(&env);
    let resp = mcp.request("tools/list", json!({}));
    let mut names: Vec<String> = resp["result"]["tools"]
        .as_array()
        .unwrap()
        .iter()
        .map(|t| t["name"].as_str().unwrap().to_string())
        .collect();
    names.sort();
    assert_eq!(
        names,
        [
            "complete_task",
            "create_plan",
            "get_plan",
            "get_taxonomy",
            "list_plans",
            "search_plans",
            "start_task",
            "update_plan",
        ]
    );
    for t in resp["result"]["tools"].as_array().unwrap() {
        assert!(
            t["description"].as_str().is_some_and(|d| d.len() > 20),
            "tool {} needs a description",
            t["name"]
        );
    }
}

#[test]
fn create_plan_writes_file_with_assigned_id() {
    let env = Env::new();
    let mut mcp = Mcp::start(&env);
    let (err, out) = mcp.call("create_plan", json!({ "markdown": FULL_PLAN }));
    assert!(!err, "{out}");
    assert_eq!(out["plan_id"], "PLAN-my-repo-001");
    assert_eq!(out["warnings"], json!([]));

    let files = md_files(&env.plans().join("my-repo"));
    assert_eq!(files.len(), 1, "{files:?}");
    let name = files[0].file_name().unwrap().to_string_lossy().into_owned();
    assert!(name.ends_with("-migrate-sessions-to-jwt.md"), "{name}");
    assert_eq!(Path::new(out["path"].as_str().unwrap()), files[0]);

    let text = std::fs::read_to_string(&files[0]).unwrap();
    assert!(text.contains("id: PLAN-my-repo-001\n"), "{text}");
    assert!(text.contains("project: my-repo\n"), "{text}");
    assert!(text.contains("owner: alice\n"), "{text}");
    let created = name[..10].to_string();
    assert!(text.contains(&format!("created: {created}\n")), "{text}");
    // Body is kept byte-for-byte.
    let body = FULL_PLAN.split_once("---\n## 01.").unwrap().1;
    assert!(text.ends_with(body), "{text}");

    // The plan is indexed and readable through get_plan.
    let (err, plan) = mcp.call("get_plan", json!({ "plan_id": "PLAN-my-repo-001" }));
    assert!(!err, "{plan}");
    assert_eq!(plan["title"], "Migrate sessions to JWT");
    assert_eq!(plan["tasks"][0]["id"], "TASK-001");

    // A second plan with the same title gets the next id and a distinct file.
    let (_, out2) = mcp.call("create_plan", json!({ "markdown": FULL_PLAN }));
    assert_eq!(out2["plan_id"], "PLAN-my-repo-002");
    let files = md_files(&env.plans().join("my-repo"));
    assert_eq!(files.len(), 2, "{files:?}");
    assert!(out2["path"].as_str().unwrap().ends_with("-jwt-2.md"));
}

#[test]
fn create_plan_without_frontmatter_gets_one() {
    let env = Env::new();
    let mut mcp = Mcp::start(&env);
    let md = "# Quick idea\n\nJust notes.\n";
    let (err, out) = mcp.call("create_plan", json!({ "markdown": md }));
    assert!(!err, "{out}");
    let text = std::fs::read_to_string(out["path"].as_str().unwrap()).unwrap();
    assert!(text.starts_with("---\n"), "{text}");
    assert!(text.contains("id: PLAN-my-repo-001\n"), "{text}");
    assert!(text.contains("title: Quick idea\n"), "{text}");
    assert!(
        text.ends_with("---\n# Quick idea\n\nJust notes.\n"),
        "{text}"
    );
}

#[test]
fn create_plan_returns_warnings_for_missing_section() {
    let env = Env::new();
    let mut mcp = Mcp::start(&env);
    let (err, out) = mcp.call("create_plan", json!({ "markdown": MISSING_SECTION_PLAN }));
    assert!(!err, "{out}");
    let codes: Vec<&str> = out["warnings"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["code"].as_str().unwrap())
        .collect();
    assert_eq!(codes.iter().filter(|c| **c == "missing_section").count(), 3);
    // The plan is still written.
    assert!(Path::new(out["path"].as_str().unwrap()).is_file());
}

#[test]
fn project_name_sanitized() {
    let env = Env::new();
    let mut mcp = Mcp::start(&env);
    let (err, out) = mcp.call(
        "create_plan",
        json!({ "markdown": FULL_PLAN, "project": "../x" }),
    );
    assert!(!err, "{out}");
    assert_eq!(out["plan_id"], "PLAN-x-001");
    assert_eq!(md_files(&env.plans().join("x")).len(), 1);
    assert!(!env.home().join("x").exists());
    assert!(!env.dir.path().join("x").exists());
}

#[test]
fn start_task_sets_in_progress() {
    let env = Env::new();
    let mut mcp = Mcp::start(&env);
    let (_, created) = mcp.call("create_plan", json!({ "markdown": FULL_PLAN }));
    let path = PathBuf::from(created["path"].as_str().unwrap());

    let (err, plan) = mcp.call(
        "start_task",
        json!({ "plan_id": "PLAN-my-repo-001", "task_id": "TASK-001" }),
    );
    assert!(!err, "{plan}");
    assert_eq!(plan["tasks"][0]["status"], "in_progress");
    let text = std::fs::read_to_string(&path).unwrap();
    assert!(text.contains("- status: in_progress\n"), "{text}");

    let (err, plan) = mcp.call(
        "complete_task",
        json!({ "plan_id": "PLAN-my-repo-001", "task_id": "TASK-001" }),
    );
    assert!(!err, "{plan}");
    assert_eq!(plan["tasks"][0]["status"], "done");
}

#[test]
fn unknown_plan_or_task_is_a_tool_error() {
    let env = Env::new();
    let mut mcp = Mcp::start(&env);
    let (err, msg) = mcp.call(
        "start_task",
        json!({ "plan_id": "PLAN-nope-001", "task_id": "TASK-001" }),
    );
    assert!(err);
    assert!(msg.as_str().unwrap().contains("PLAN-nope-001"), "{msg}");

    mcp.call("create_plan", json!({ "markdown": FULL_PLAN }));
    let (err, msg) = mcp.call(
        "complete_task",
        json!({ "plan_id": "PLAN-my-repo-001", "task_id": "TASK-999" }),
    );
    assert!(err);
    assert!(msg.as_str().unwrap().contains("TASK-999"), "{msg}");

    let (err, _) = mcp.call("get_plan", json!({ "plan_id": "PLAN-nope-001" }));
    assert!(err);
    let (err, _) = mcp.call(
        "update_plan",
        json!({ "plan_id": "PLAN-nope-001", "markdown": FULL_PLAN }),
    );
    assert!(err);
}

#[test]
fn update_plan_keeps_id_project_created_and_path() {
    let env = Env::new();
    let mut mcp = Mcp::start(&env);
    let (_, created) = mcp.call("create_plan", json!({ "markdown": FULL_PLAN }));
    let path = created["path"].as_str().unwrap().to_string();
    let before = std::fs::read_to_string(&path).unwrap();

    let edited = FULL_PLAN
        .replace(
            "title: Migrate sessions to JWT",
            "title: JWT v2\nid: PLAN-evil-009\nproject: evil",
        )
        .replace("## 05. Risks & Rollback\n\nRollback.\n", "");
    let (err, out) = mcp.call(
        "update_plan",
        json!({ "plan_id": "PLAN-my-repo-001", "markdown": edited }),
    );
    assert!(!err, "{out}");
    assert_eq!(out["plan_id"], "PLAN-my-repo-001");
    assert_eq!(out["path"].as_str().unwrap(), path);
    assert!(
        out["warnings"]
            .as_array()
            .unwrap()
            .iter()
            .any(|w| w["code"] == "missing_section")
    );

    let after = std::fs::read_to_string(&path).unwrap();
    assert_ne!(before, after);
    assert!(after.contains("id: PLAN-my-repo-001\n"), "{after}");
    assert!(after.contains("project: my-repo\n"), "{after}");
    assert!(after.contains("title: JWT v2\n"), "{after}");
    assert!(!after.contains("evil"), "{after}");
    let created_line = before.lines().find(|l| l.starts_with("created: ")).unwrap();
    assert!(after.contains(created_line), "{after}");

    let (_, plan) = mcp.call("get_plan", json!({ "plan_id": "PLAN-my-repo-001" }));
    assert_eq!(plan["title"], "JWT v2");
}

#[test]
fn list_search_and_taxonomy() {
    let env = Env::new();
    let mut mcp = Mcp::start(&env);
    mcp.call("create_plan", json!({ "markdown": FULL_PLAN }));
    mcp.call(
        "create_plan",
        json!({ "markdown": MISSING_SECTION_PLAN, "project": "other" }),
    );

    let (err, all) = mcp.call("list_plans", json!({}));
    assert!(!err, "{all}");
    assert_eq!(all.as_array().unwrap().len(), 2);
    let (_, mine) = mcp.call("list_plans", json!({ "project": "my-repo" }));
    assert_eq!(mine.as_array().unwrap().len(), 1);
    assert_eq!(mine[0]["id"], "PLAN-my-repo-001");

    let (err, found) = mcp.call("search_plans", json!({ "query": "JWT" }));
    assert!(!err, "{found}");
    assert_eq!(found["plans"][0]["id"], "PLAN-my-repo-001");
    let (_, none) = mcp.call(
        "search_plans",
        json!({ "query": "JWT", "tags": ["domain/security"] }),
    );
    assert_eq!(none["plans"], json!([]));
    let (_, tagged) = mcp.call(
        "search_plans",
        json!({ "query": "JWT", "tags": ["type/feature"] }),
    );
    assert_eq!(tagged["plans"][0]["id"], "PLAN-my-repo-001");

    let (err, tax) = mcp.call("get_taxonomy", json!({}));
    assert!(!err, "{tax}");
    let tags = tax["tags"].as_array().unwrap();
    let feature = tags.iter().find(|t| t["tag"] == "type/feature").unwrap();
    assert!(feature["group"].as_str().is_some_and(|g| !g.is_empty()));
    assert!(feature["label_en"].as_str().is_some_and(|l| !l.is_empty()));
    assert!(feature["desc_en"].is_string());
}

/// A home with the structured fixture indexed in-process.
fn indexed_home() -> (TempDir, Paths) {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(dir.path().to_path_buf());
    paths.ensure().unwrap();
    let proj = paths.plans.join("auth-gateway");
    std::fs::create_dir_all(&proj).unwrap();
    std::fs::copy(
        Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plans/structured.md"),
        proj.join("2026-11-20-jwt.md"),
    )
    .unwrap();
    let tax = Arc::new(Taxonomy::embedded_default());
    let store = Arc::new(Store::open(&paths.db, tax.clone()).unwrap());
    Indexer::new(paths.clone(), store, tax).full_scan().unwrap();
    (dir, paths)
}

#[test]
fn task_cli_start_and_done() {
    let (_dir, paths) = indexed_home();
    let file = paths.plans.join("auth-gateway/2026-11-20-jwt.md");

    assert_cmd::Command::new(bin())
        .args(["task", "start", "PLAN-auth-gateway-001", "TASK-003"])
        .env("PLAN_ATLAS_HOME", &paths.home)
        .assert()
        .success()
        .stdout("TASK-003 → in_progress\n");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(
        text.contains("### TASK-003: RBAC middleware\n- status: in_progress\n"),
        "{text}"
    );

    assert_cmd::Command::new(bin())
        .args(["task", "done", "PLAN-auth-gateway-001", "TASK-003"])
        .env("PLAN_ATLAS_HOME", &paths.home)
        .assert()
        .success()
        .stdout("TASK-003 → done\n");
    let text = std::fs::read_to_string(&file).unwrap();
    assert!(text.contains("- status: done\n"), "{text}");
}

#[test]
fn task_cli_unknown_task_fails() {
    let (_dir, paths) = indexed_home();
    assert_cmd::Command::new(bin())
        .args(["task", "start", "PLAN-auth-gateway-001", "TASK-404"])
        .env("PLAN_ATLAS_HOME", &paths.home)
        .assert()
        .failure()
        .code(1)
        .stdout("")
        .stderr(predicates::str::contains("TASK-404"));
}
