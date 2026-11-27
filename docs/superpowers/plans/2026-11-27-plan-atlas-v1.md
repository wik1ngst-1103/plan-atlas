# Plan Atlas v1 Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Một binary `plan-atlas` (Rust) gom plan của Claude Code về một kho, theo dõi task và hoạt động subagent theo thời gian thực, phục vụ web UI cục bộ, cài được bằng một lệnh.

**Architecture:** File Markdown trong `~/.plan-atlas/plans/` là nguồn sự thật; SQLite (WAL) làm chỉ mục + lưu sự kiện. Cùng một binary chạy 4 vai: web server (axum + SSE, SPA nhúng), MCP server stdio, hook ghi sự kiện, trình cài đặt vào Claude Code. Frontend Svelte 5 build sẵn rồi nhúng bằng `rust-embed`.

**Tech Stack:** Rust 1.85+ (edition 2024) · tokio · axum 0.8 · rusqlite (feature `bundled`, FTS5) · notify + notify-debouncer-full · pulldown-cmark · serde + serde_yaml_ng · clap 4 · rust-embed 8 · webbrowser · rmcp (MCP SDK chính thức) · ts-rs · tempfile (test) — Svelte 5 · Vite · TypeScript · d3-force · vitest — cargo-dist.

**Spec:** `docs/superpowers/specs/2026-11-27-plan-atlas-design.md` · Prototype UI: `docs/prototype/index.html`

## Global Constraints

- Một binary tên `plan-atlas`; thư mục dữ liệu `~/.plan-atlas` (ghi đè bằng env `PLAN_ATLAS_HOME`).
- Bố cục dữ liệu: `plans/<project>/<YYYY-MM-DD>-<slug>.md`, `plans/unsorted/`, `atlas.db`, `config.toml`, `taxonomy.toml`.
- ID: plan `PLAN-<project>-<NNN>` (đếm theo dự án, 3 chữ số), task `TASK-<NNN>`.
- Trạng thái task (serde snake_case): `todo | in_progress | review | done`. Quan hệ: `depends_on | extends | relates_to`.
- Mặc định complexity → (model, effort): 1→(haiku,low) 2→(sonnet,low) 3→(sonnet,medium) 4→(opus,medium) 5→(opus,high).
- Server chỉ bind `127.0.0.1`, cổng mặc định `7420` (thử tới `7430`), ghi đè bằng `--port` / env `PLAN_ATLAS_PORT`.
- Mọi request không phải GET phải có header `X-Plan-Atlas: 1`; header `Host` phải là `127.0.0.1:<port>` hoặc `localhost:<port>`, sai → 403. Không CORS.
- Ngân sách: RSS rảnh ≤ 40 MB với 1.000 plan; binary release ≤ 20 MB; JS gzip ≤ 200 KB; khởi động tới khi mở trình duyệt ≤ 1 s.
- Tokio `multi_thread` với `worker_threads(2)`. SQLite: một `Connection` ghi sau `Mutex`, gọi qua `spawn_blocking`; `journal_mode=WAL`, `busy_timeout=2000`.
- `plan-atlas hook` luôn thoát mã 0, không in stdout, tự huỷ sau 200 ms.
- Không bao giờ sửa/xoá file trong `~/.claude/plans/`.
- UI song ngữ `en` + `vi` ngay từ v1: mọi chuỗi giao diện, câu mô tả hoạt động (humanize) và nhãn/mô tả chủ đề đều có cả hai ngôn ngữ. Ngôn ngữ mặc định theo `navigator.language` (bắt đầu bằng `vi` → `vi`, còn lại `en`), đổi bằng nút trong TopBar, lưu ở `localStorage` (bọc try/catch). Thiếu khoá ở một ngôn ngữ là lỗi CI.
- Giấy phép MIT; mọi commit theo Conventional Commits.

## Review Focus

1. **Hook gặp sự cố** (stdin hỏng, DB bị khoá, thư mục chưa tạo) → vẫn thoát 0 trong ≤ 200 ms, không in gì; Claude Code không bị ảnh hưởng. *Test ở Task 9.*
2. **`settings.json` có sẵn hook của người dùng hoặc JSON không hợp lệ** → gộp không mất gì, tạo bản sao lưu, JSON hỏng thì dừng và không ghi đè; chạy `setup` 2 lần không nhân đôi. *Test ở Task 11.*
3. **File plan sửa tay bị lỗi** (thiếu frontmatter, trùng TASK id, CRLF, không phải UTF-8) → vẫn lập chỉ mục kèm cảnh báo, không panic; ghi lại trạng thái giữ nguyên CRLF và mọi byte khác. *Test ở Task 3, 4.*
4. **Tìm không dấu tiếng Việt, kể cả “đ”** → “dang nhap” tìm ra “đăng nhập”. *Test ở Task 5.*
5. **Chạy `plan-atlas` lần hai / cổng bị app khác chiếm** → lần hai chỉ mở trình duyệt tới instance đang chạy; cổng bị chiếm thì chuyển cổng kế tiếp. *Test ở Task 12.*

---

## File Structure

```
Cargo.toml  rust-toolchain.toml  dist-workspace.toml
src/
  main.rs                 # clap dispatch → các module
  config.rs               # Paths, Config, đọc/ghi config.toml
  taxonomy.rs             # Taxonomy, nạp taxonomy.toml, nhãn en/vi
  plan/model.rs           # Plan, Task, Section, Relation, enums, Warning
  plan/parse.rs           # parse_plan
  plan/write.rs           # set_task_status, render_new_plan
  plan/validate.rs        # validate
  store/mod.rs            # Store (mở DB, migrate)
  store/schema.sql
  store/plans.rs          # upsert/xoá/truy vấn plan, task, topic, graph
  store/search.rs         # FTS5 + bỏ dấu
  store/events.rs         # events, sessions, live agents, log
  index.rs                # Indexer: quét toàn bộ + watcher
  import.rs               # nhập ~/.claude/plans
  events/hook.rs          # lệnh `hook`
  events/humanize.rs      # tool + input → câu thường (en/vi)
  events/live.rs          # poller data_version → broadcast
  server/mod.rs           # build_router, serve
  server/routes.rs        # handler REST + SSE
  server/security.rs      # middleware Host + X-Plan-Atlas
  server/assets.rs        # SPA nhúng
  mcp.rs                  # lệnh `mcp`
  setup.rs                # setup / uninstall-claude
  launch.rs               # lệnh mặc định
assets/skill/SKILL.md.tmpl   assets/taxonomy.default.toml
tests/                    # integration test Rust
web/                      # Svelte app (package.json, vite.config.ts, src/...)
scripts/bench-memory.sh
.github/workflows/ci.yml  .github/ISSUE_TEMPLATE/*  .github/pull_request_template.md
README.md CONTRIBUTING.md LICENSE CODE_OF_CONDUCT.md SECURITY.md CHANGELOG.md
```

---

### Task 1: Khung dự án, CI, tài liệu mã nguồn mở

**Files:**
- Create: `Cargo.toml`, `rust-toolchain.toml` (channel `1.85` trở lên, components `clippy, rustfmt`), `src/main.rs`, `.gitignore`, `.editorconfig`, `CHANGELOG.md`, `.github/workflows/ci.yml`, `.github/ISSUE_TEMPLATE/bug_report.yml`, `.github/ISSUE_TEMPLATE/feature_request.yml`, `.github/pull_request_template.md`
- Đã có: `README.md`, `CONTRIBUTING.md`, `LICENSE`, `CODE_OF_CONDUCT.md`, `SECURITY.md` (chỉ cập nhật nếu lệnh thay đổi)
- Test: `tests/cli.rs`

**Interfaces:**
- Produces: `enum Command { Serve{port:Option<u16>, no_open:bool}, Mcp, Hook, Setup{yes:bool}, UninstallClaude, Import, Task{action:TaskAction} }`; không có lệnh con → `Serve`.

- [ ] **Step 1:** `git init`, nhánh `main`. Test `cli_help_lists_subcommands`: chạy `plan-atlas --help` (dùng `assert_cmd`) → stdout chứa `mcp`, `hook`, `setup`, `import`.
- [ ] **Step 2:** `cargo test --test cli` → FAIL.
- [ ] **Step 3:** Khai báo `Command` bằng clap derive trong `src/main.rs`; mỗi nhánh tạm `todo!()` trừ `--help/--version`. Profile release: `opt-level = 3`, `lto = "thin"`, `codegen-units = 1`, `strip = true`; giữ `panic = "unwind"` mặc định (Task 9 cần `catch_unwind` để hook không bao giờ làm hỏng Claude Code).
- [ ] **Step 4:** CI `ci.yml`: job `rust` (ubuntu, macos, windows): `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings`, `cargo test`. (Job `web` thêm ở Task 13.)
- [ ] **Step 5:** `cargo test` → PASS. Commit `chore: scaffold project, CI and community files`.

### Task 2: Đường dẫn, cấu hình, bộ phân loại

**Files:** Create `src/config.rs`, `src/taxonomy.rs`, `assets/taxonomy.default.toml` · Test `tests/config.rs`

**Interfaces:**
- Produces: `struct Paths { home, plans, unsorted, db, config, taxonomy }`, `Paths::resolve() -> Paths` (env `PLAN_ATLAS_HOME` hoặc `~/.plan-atlas`), `Paths::ensure() -> io::Result<()>`; `struct Config { port: u16, locale: String, setup_done: bool }` + `Config::load(&Paths)`, `Config::save(&Paths)`; `struct Taxonomy { tags: BTreeMap<String, TagInfo> }`, `struct TagInfo { group: String, label_en, label_vi, desc_en, desc_vi }`, `Taxonomy::load(&Paths) -> Taxonomy` (thiếu file → chép từ default), `Taxonomy::contains(&str) -> bool`.

- [ ] **Step 1:** Tests: `resolve_respects_env` (đặt `PLAN_ATLAS_HOME` = tempdir → `paths.db == tmp/atlas.db`); `config_roundtrip` (save rồi load bằng nhau, mặc định `port=7420, locale="en", setup_done=false`); `taxonomy_default_written_and_loaded` (sau load có file, `contains("tech/jwt")`, nhóm `type|domain|tech`).
- [ ] **Step 2:** Chạy → FAIL.
- [ ] **Step 3:** Cài đặt. `taxonomy.default.toml` chứa ít nhất các tag trong prototype (`type/feature, docs, refactor, bugfix, research, infra`; `domain/security, backend, frontend, data, devops, ai`; `tech/jwt, oauth, rbac, redis, postgres, react, threejs, gltf, rag`) với nhãn + mô tả en/vi (lấy từ `TAGS` trong prototype cho bản vi).
- [ ] **Step 4:** PASS. Commit `feat: paths, config and taxonomy`.

### Task 3: Mô hình & bộ phân tích plan

**Files:** Create `src/plan/model.rs`, `src/plan/parse.rs`, `src/plan/validate.rs` · Test `tests/plan_parse.rs`, fixtures `tests/fixtures/plans/{structured.md, unstructured.md, crlf.md, dup_task.md, no_frontmatter.md}`

**Interfaces:**
- Produces (derive `Serialize, Deserialize, ts_rs::TS`, export sang `web/src/lib/types.gen.ts`):
  `enum TaskStatus {Todo, InProgress, Review, Done}`, `enum Model {Haiku, Sonnet, Opus}`, `enum Effort {Low, Medium, High}`, `enum RelationKind {DependsOn, Extends, RelatesTo}`,
  `struct Relation { to: String, kind: RelationKind }`,
  `struct Task { id, name, complexity: Option<u8>, model: Option<Model>, effort: Option<Effort>, depends_on: Vec<String>, status: TaskStatus, tags: Vec<String>, body_md: String }` + `Task::resolved_model() -> (Model, Effort)` (theo bảng mặc định khi thiếu; complexity thiếu → 3),
  `struct Section { level: u8, title: String, anchor: String, body_md: String, children: Vec<Section> }`,
  `struct Warning { code: String, message: String }`,
  `struct Plan { id, project, title, tags, related: Vec<Relation>, created: Option<String>, sections: Vec<Section>, tasks: Vec<Task>, structured: bool, source_path: PathBuf, content_hash: String, warnings: Vec<Warning> }`,
  `fn parse_plan(bytes: &[u8], path: &Path) -> Plan`, `fn validate(plan: &Plan, tax: &Taxonomy) -> Vec<Warning>`.

Quy tắc phân tích: frontmatter YAML giữa hai dòng `---` đầu file. Heading cấp 2 thành section gốc, cấp 3 thành con. Trong section có tiêu đề bắt đầu `04.`, heading `### TASK-\d{3}: <name>` là task; các dòng `- key: value` liền sau heading là metadata (`complexity, model, effort, depends_on, status, tags`, mảng dạng `[a, b]`), phần còn lại là `body_md`. `anchor` = slug của title. `structured = frontmatter hợp lệ && có section 04`. `content_hash` = blake3/sha256 hex của bytes. Bytes không phải UTF-8 → `String::from_utf8_lossy` + warning `non_utf8`.

- [ ] **Step 1:** Tests:
  - `parses_structured_fixture`: id/project/title đúng; 5 section gốc; section 03 có 3 con; 3 task, TASK-002 `status == InProgress`, `depends_on == ["TASK-001"]`; `related[0].kind == DependsOn`.
  - `resolved_model_defaults`: task complexity 4 không có model → `(Opus, Medium)`.
  - `unstructured_has_sections_no_tasks`: `structured == false`, `tasks.is_empty()`, sections theo heading.
  - `crlf_parses_same_as_lf`: kết quả `crlf.md` bằng `structured.md` (trừ hash, path).
  - `duplicate_task_ids_warn`: `validate` trả warning code `duplicate_task_id`.
  - `missing_frontmatter_warns_not_panics`: warning `missing_frontmatter`, title lấy từ H1 đầu tiên hoặc tên file.
  - `unknown_tag_warns`: tag `tech/xyz` → warning `unknown_tag`.
- [ ] **Step 2:** FAIL. **Step 3:** Cài đặt bằng `pulldown-cmark` (duyệt event heading có offset để cắt `body_md` từ văn bản gốc). **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(plan): model, parser and validation`.

### Task 4: Ghi plan (đổi trạng thái, tạo plan mới)

**Files:** Create `src/plan/write.rs` · Test `tests/plan_write.rs`

**Interfaces:**
- Consumes: `TaskStatus`, `parse_plan`.
- Produces: `fn set_task_status(bytes: &[u8], task_id: &str, status: TaskStatus) -> Result<Vec<u8>, WriteError>` (`WriteError::TaskNotFound`); `fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()>` (ghi file tạm cùng thư mục rồi rename); `fn next_plan_id(project: &str, existing: &[String]) -> String`; `fn plan_file_name(created: &str, title: &str) -> String` (`2026-11-20-migrate-sessions-to-jwt-rbac.md`, slug ASCII, bỏ dấu, ≤ 60 ký tự).

- [ ] **Step 1:** Tests: `status_line_replaced` (chỉ đúng một dòng đổi, diff mọi dòng khác bằng nhau); `status_line_inserted_when_missing` (chèn ngay sau heading task); `preserves_crlf` (file CRLF vẫn CRLF toàn bộ); `unknown_task_errors`; `next_id_increments` (`["PLAN-a-001","PLAN-a-007"]` → `PLAN-a-008`); `file_name_slug` (`"Chuyển session sang JWT + RBAC"` → `2026-11-20-chuyen-session-sang-jwt-rbac.md`).
- [ ] **Step 2:** FAIL. **Step 3:** Cài đặt (thao tác trên dòng, không render lại markdown). **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(plan): status write-back and new plan naming`.

### Task 5: Kho SQLite + tìm kiếm

**Files:** Create `src/store/{mod.rs, schema.sql, plans.rs, search.rs}` · Test `tests/store.rs`

**Interfaces:**
- Consumes: `Plan`, `Task`, `Taxonomy`.
- Produces: `Store::open(path: &Path) -> Result<Store>` (WAL, migrate theo `PRAGMA user_version`); `upsert_plan(&self, &Plan) -> Result<()>` (bỏ qua nếu `content_hash` không đổi); `delete_by_path(&self, &Path)`; `list_projects() -> Vec<ProjectSummary{name, plan_count, task_count, done_count}>`; `list_plans(PlanFilter{project:Option<String>, tag:Option<String>}) -> Vec<PlanSummary{id, project, title, tags, created, task_count, done_count, structured, warnings_count}>`; `get_plan(id) -> Option<Plan>`; `list_tasks(TaskFilter{project, tag}) -> Vec<TaskRow{plan_id, plan_title, project, task: Task}>`; `topics() -> Vec<TopicCount{tag, plans, tasks}>`; `graph() -> Graph{nodes: Vec<PlanSummary>, edges: Vec<Edge{from, to, kind: EdgeKind}>}` (`EdgeKind::Declared(RelationKind) | SharedTopics(Vec<String>)`, chủ đề chung chỉ tính `domain/` và `tech/`); `search(q: &str, limit: usize) -> SearchResults{plans, tasks, topics}`; `data_version() -> i64`; `plan_ids_for_project(&str) -> Vec<String>`; `find_task_path(plan_id) -> Option<PathBuf>`.

Bảng: `plans(id PK, project, title, path UNIQUE, json, hash, created)`, `tasks(plan_id, id, name, status, complexity, json, PK(plan_id,id))`, `plan_tags(plan_id, tag)`, `task_tags(plan_id, task_id, tag)`, `relations(from_id, to_id, kind)`, FTS5 `search_fts(kind, ref, title, body, tokenize='unicode61 remove_diacritics 2')`. Trước khi chèn và khi truy vấn, chuẩn hoá `đ/Đ → d` (unicode61 không gập ký tự này).

- [ ] **Step 1:** Tests: `upsert_then_get_roundtrip`; `upsert_same_hash_is_noop` (đếm số lần ghi qua `changes()`); `delete_by_path_removes_tasks_and_tags`; `graph_has_declared_and_shared_edges` (dùng 3 fixture như prototype: auth-001, auth-002 depends_on auth-001, rag-002 cùng `tech/jwt`); `search_without_diacritics` (plan có “đăng nhập” → `search("dang nhap")` trả plan đó; `search("bao mat")` trả plan có chủ đề “Bảo mật” qua nhãn taxonomy được đưa vào `body`).
- [ ] **Step 2:** FAIL. **Step 3:** Cài đặt. **Step 4:** PASS.
- [ ] **Step 5:** Commit `feat(store): sqlite index with diacritic-insensitive search`.

### Task 6: Lập chỉ mục & theo dõi file

**Files:** Create `src/index.rs` · Test `tests/index.rs`

**Interfaces:**
- Consumes: `Paths`, `Store`, `parse_plan`, `validate`, `Taxonomy`.
- Produces: `Indexer::new(paths, store: Arc<Store>, tax: Arc<Taxonomy>)`; `full_scan(&self) -> Result<ScanReport{indexed, removed, warnings}>` (xoá khỏi DB các path không còn tồn tại); `watch(self, tx: broadcast::Sender<LiveEvent>) -> Result<WatchHandle>` (debounce 300 ms, chỉ file `.md`, phát `LiveEvent::PlanChanged{plan_id}`).

- [ ] **Step 1:** Tests: `full_scan_indexes_and_prunes` (2 file → 2 plan; xoá 1 file, quét lại → 1); `watch_picks_up_new_file` (ghi file mới, nhận `PlanChanged` trong 2 s).
- [ ] **Step 2:** FAIL. **Step 3:** Cài đặt với `notify-debouncer-full`. **Step 4:** PASS. **Step 5:** Commit `feat: indexer and file watcher`.

### Task 7: Nhập plan cũ từ Claude Code

**Files:** Create `src/import.rs` · Test `tests/import.rs`

**Interfaces:**
- Produces: `fn import_claude_plans(claude_home: &Path, paths: &Paths) -> Result<ImportReport{imported, skipped, unsorted}>`; `fn infer_project(plan_file_name: &str, claude_home: &Path) -> Option<String>` (quét `projects/*/*.jsonl` tìm tên file, lấy `cwd` gần nhất trong dòng đó, trả basename của git root hoặc của `cwd`).

Bản sao ghi vào `plans/<project|unsorted>/<tên gốc>.md`, thêm frontmatter `imported_from: <đường dẫn gốc>`, `id` cấp mới. Nhận diện đã nhập bằng `imported_from` + hash → chạy lại không nhân đôi.

- [ ] **Step 1:** Tests (fixture `tests/fixtures/claude_home/`): `imports_and_infers_project`; `unknown_goes_unsorted`; `idempotent_second_run` (`imported == 0` lần hai); `never_touches_source` (mtime + nội dung file gốc không đổi).
- [ ] **Step 2–4:** FAIL → cài đặt → PASS. Lệnh `plan-atlas import` gọi hàm này và in báo cáo.
- [ ] **Step 5:** Commit `feat: import existing Claude Code plans`.

### Task 8: HTTP API + bảo mật + SSE

**Files:** Create `src/server/{mod.rs, routes.rs, security.rs}` · Test `tests/api.rs` (dùng `tower::ServiceExt::oneshot`)

**Interfaces:**
- Consumes: `Store`, `set_task_status`, `write_atomic`, `LiveEvent`.
- Produces: `fn build_router(state: AppState) -> Router`; `struct AppState { store: Arc<Store>, paths: Arc<Paths>, tax: Arc<Taxonomy>, live: broadcast::Sender<LiveEvent>, port: u16 }`; `enum LiveEvent { PlanChanged{plan_id}, TaskStatus{plan_id, task_id, status}, AgentStep{plan_id, task_id, agent: AgentView} }` (serde tag `type`).

Routes (JSON): `GET /api/health → {app:"plan-atlas", version}` · `GET /api/projects` · `GET /api/plans?project&tag` · `GET /api/plans/:id` · `PATCH /api/plans/:id/tasks/:task {status}` (ghi file → reindex plan → phát `TaskStatus`) · `GET /api/tasks?project&tag` · `GET /api/topics` (kèm nhãn/mô tả theo `?locale`) · `GET /api/graph` · `GET /api/search?q` · `GET /api/activity` · `GET /api/plans/:id/tasks/:task/log?limit=30` · `GET /api/stream` (SSE, keep-alive 15 s).

- [ ] **Step 1:** Tests: `health_ok`; `patch_status_updates_file_and_db` (file trên đĩa có `- status: done`); `patch_without_header_forbidden` (403); `bad_host_forbidden` (`Host: evil.com:7420` → 403); `unknown_plan_404`; `search_endpoint_groups_results`; `path_traversal_rejected` (plan id `../../etc` → 404, không đọc ngoài `plans/`; mọi đường dẫn lấy từ DB phải nằm dưới `paths.plans` sau `canonicalize`).
- [ ] **Step 2–4:** FAIL → cài đặt → PASS. **Step 5:** Commit `feat(server): REST API, SSE and localhost guards`.

### Task 9: Hook, câu chữ thường, luồng thời gian thực

**Files:** Create `src/events/{hook.rs, humanize.rs, live.rs}`, `src/store/events.rs` · Test `tests/hook.rs`, `tests/humanize.rs`, fixtures `tests/fixtures/hooks/*.json`

**Interfaces:**
- Produces: `fn run_hook(stdin: impl Read, paths: &Paths) -> ()` (không bao giờ panic ra ngoài: bọc `catch_unwind`, toàn bộ lỗi nuốt, deadline 200 ms bằng thread + `recv_timeout`); `Store::insert_event(&HookEvent)`, `Store::set_session_task(session, plan_id, task_id)`, `Store::clear_session(session)`, `Store::live_agents() -> Vec<LiveTask{plan_id, task_id, agents: Vec<AgentView{agent_id, role, model:Option<String>, step: Step}>}>` (agent có sự kiện trong 120 s gần nhất và chưa `SubagentStop`), `Store::task_log(plan, task, limit) -> Vec<LogEntry{at, role, step}>`, `Store::prune_events(older_than_days: u32)`; `struct Step { icon: String, text_en: String, text_vi: String, raw: String }`; `fn humanize(tool: &str, input: &serde_json::Value) -> Step`; `fn spawn_live_poller(store, tx) -> JoinHandle` (500 ms, so `data_version`, phát `AgentStep` cho sự kiện mới).

Hook đăng ký cho `PreToolUse`, `SubagentStart`, `SubagentStop`, `SessionEnd`. `tool_name == "mcp__plan-atlas__start_task"` → `set_session_task` từ `tool_input.plan_id/task_id`. Sự kiện gắn agent theo `agent_id` nếu payload có, không có → `"main"`; role lấy từ `agent_type` (ví dụ `Explore`, `general-purpose`).

Bảng humanize (vi; en tương tự): `Read`→📖 “đọc {file}”, `Edit|Write|MultiEdit`→✏️ “sửa {file}”, `Grep|Glob`→🔍 “tìm kiếm trong code”, `Bash` có `test` → 🧪 “chạy kiểm thử”, `Bash` có `lint|fmt|clippy` → 🧹 “kiểm tra quy chuẩn code”, `Bash` có `install|add` → 📦 “cài thư viện”, `Bash` khác → ⚙️ “chạy lệnh”, `WebFetch|WebSearch` → 🌐 “tra cứu trên web”, `Task|Agent` → 🤝 “giao việc cho trợ lý”, `mcp__plan-atlas__*` → 🗺 “cập nhật plan”, khác → ⚙️ “dùng công cụ {tool}”. `{file}` = tên file (không đường dẫn).

- [ ] **Step 1:** Ghi lại payload thật: thêm cờ ẩn `plan-atlas hook --dump <dir>` lưu stdin; chạy một phiên Claude Code có subagent, lưu 4 loại payload vào `tests/fixtures/hooks/`. Kiểm tra tên trường (`session_id`, `tool_name`, `tool_input`, `agent_id`, `agent_type`) so với tài liệu hooks hiện hành; nếu khác, cập nhật Interfaces ở task này trước khi viết code.
- [ ] **Step 2:** Tests: `garbage_stdin_exits_quietly` (stdin `"{not json"` → hàm trả về, không panic, stdout rỗng); `locked_db_returns_within_deadline` (giữ transaction ghi ở connection khác → `run_hook` xong < 300 ms); `start_task_links_session` (fixture start_task rồi fixture Read cùng session → `task_log` có 1 dòng của task đó); `subagent_stop_removes_agent`; `humanize_cases` (mỗi dòng bảng trên một assert, ví dụ `Bash {"command":"npm test -- token"}` → `text_vi == "chạy kiểm thử"`).
- [ ] **Step 3–4:** FAIL → cài đặt → PASS. **Step 5:** Commit `feat(events): hook ingestion, humanized steps, live poller`.

### Task 10: MCP server

**Files:** Create `src/mcp.rs` · Test `tests/mcp.rs` (gửi JSON-RPC qua stdin/stdout của tiến trình con)

**Interfaces:**
- Consumes: `Store`, `parse_plan`, `validate`, `next_plan_id`, `plan_file_name`, `write_atomic`, `set_task_status`, `Taxonomy`.
- Produces — tools (tên chính xác): `list_plans{project?}`, `search_plans{query, tags?}`, `get_plan{plan_id}`, `get_taxonomy{}`, `create_plan{markdown, project?}` → `{plan_id, path, warnings}` (project mặc định = basename git root của cwd tiến trình MCP; ghi đè `id`, `project`, `created` trong frontmatter), `update_plan{plan_id, markdown}`, `start_task{plan_id, task_id}` → status `in_progress`, `complete_task{plan_id, task_id}` → status `done`. Lỗi cấu trúc → trả `warnings` để Claude tự sửa (không từ chối).

- [ ] **Step 1:** Tests: `tools_list_has_all_eight`; `create_plan_writes_file_with_assigned_id` (file tồn tại ở `plans/<project>/`, frontmatter `id: PLAN-<project>-001`); `create_plan_returns_warnings_for_missing_section`; `start_task_sets_in_progress`; `project_name_sanitized` (`project: "../x"` → thư mục `plans/x`, chỉ giữ `[a-z0-9-_]`).
- [ ] **Step 2–4:** FAIL → cài đặt bằng `rmcp` (transport stdio) → PASS. **Step 5:** Commit `feat: MCP server for plan authoring and task tracking`.

### Task 11: Cài vào Claude Code (`setup`, `uninstall-claude`) + Skill

**Files:** Create `src/setup.rs`, `assets/skill/SKILL.md.tmpl` (viết lại `docs/prototype/SKILL.draft.md` bằng tiếng Anh, hướng dẫn dùng tool MCP thay vì tự ghi file, gọi `search_plans` trước khi tạo plan để điền `related`, gọi `start_task`/`complete_task`, chọn model subagent theo `resolved_model`; phần tag sinh từ taxonomy) · Test `tests/setup.rs`

**Interfaces:**
- Produces: `fn install(claude_home: &Path, bin: &Path, tax: &Taxonomy) -> Result<SetupReport>`; `fn uninstall(claude_home: &Path) -> Result<SetupReport>`; `fn merge_hooks(settings: serde_json::Value, bin: &Path) -> serde_json::Value` (thuần, để test); `fn render_skill(tmpl: &str, tax: &Taxonomy) -> String`.

Hành vi: ghi `~/.claude/skills/plan-atlas/SKILL.md`; sao lưu `settings.json` → `settings.json.bak-<unix_ts>`; thêm hook cho 4 sự kiện ở Task 9 với `command = "<bin tuyệt đối> hook"`, nhận diện hook của mình bằng chuỗi con `plan-atlas hook` (idempotent); MCP: nếu `claude` có trong PATH chạy `claude mcp add --scope user plan-atlas -- <bin> mcp`, không thì in lệnh để người dùng tự chạy. JSON không hợp lệ → lỗi, không ghi gì.

- [ ] **Step 1:** Tests: `merge_preserves_user_hooks` (settings có sẵn hook khác cho `PreToolUse` → vẫn còn, thêm của mình); `merge_is_idempotent` (gộp 2 lần bằng gộp 1 lần); `invalid_json_aborts_without_write` (nội dung file không đổi, không có `.bak`); `uninstall_removes_only_ours`; `skill_contains_taxonomy_tags`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS. **Step 5:** Commit `feat: install skill, hooks and MCP into Claude Code`.

### Task 12: Trình khởi chạy (lệnh mặc định)

**Files:** Create `src/launch.rs`, `src/server/assets.rs` (tạm phục vụ trang “UI chưa build” nếu `web/dist` trống) · Test `tests/launch.rs`

**Interfaces:**
- Consumes: mọi thứ ở trên.
- Produces: `async fn run(opts: ServeOpts) -> Result<()>`; `fn pick_port(preferred: u16) -> PortChoice { Free(u16), OursRunning(u16) }` (thử `preferred..=7430`; cổng bận thì gọi `/api/health`, đúng `app == "plan-atlas"` → `OursRunning`).

Trình tự: `Paths::ensure` → lần đầu (`!config.setup_done`) và stdin là TTY → hỏi “Connect Plan Atlas to Claude Code? [Y/n]” → `setup::install` → `import_claude_plans` → `full_scan` → `pick_port` (`OursRunning` → mở trình duyệt, thoát 0) → `prune_events(7)` → spawn watcher + poller → serve → `webbrowser::open` (trừ `--no-open`) → in `Plan Atlas is running at http://127.0.0.1:<port> — press Ctrl+C to stop` → Ctrl+C tắt êm.

- [ ] **Step 1:** Tests: `second_instance_detected` (khởi server test trên cổng X, `pick_port(X) == OursRunning(X)`); `foreign_port_skipped` (chiếm cổng X bằng `TcpListener` thường → `Free(X+1)`).
- [ ] **Step 2–4:** FAIL → cài đặt → PASS. Kiểm thử tay: `cargo run` mở trình duyệt; chạy lần hai chỉ mở tab mới.
- [ ] **Step 5:** Commit `feat: launcher with single-instance and auto-open browser`.

### Task 13: Frontend — khung, router, i18n, API, live

**Files:** Create `web/{package.json, vite.config.ts, tsconfig.json, index.html}`, `web/src/{main.ts, App.svelte}`, `web/src/lib/{api.ts, router.ts, i18n.ts, live.ts, types.gen.ts}`, `web/src/locales/{en.json, vi.json}`, `web/src/components/{TopBar, SideNav, Crumbs, StatusChip, TopicChip}.svelte` · Test `web/src/lib/*.test.ts`

**Interfaces:**
- Consumes: API Task 8, kiểu sinh từ ts-rs (Task 3, 5, 9).
- Produces: `route` store với dạng `{page:'home'} | {page:'project', project} | {page:'plan', plan, tab:'docs'|'tasks'} | {page:'board'} | {page:'map'}` parse từ hash `#/home`, `#/project/:p`, `#/plan/:id/:tab`, `#/board`, `#/map`; `t(key, params?)`; `api.get<T>(path)`, `api.patch(path, body)` (luôn gửi `X-Plan-Atlas: 1`); `locale` store (`'en'|'vi'`) + `setLocale()`; mọi request kèm `?locale=` để server trả nhãn chủ đề và câu hoạt động đúng ngôn ngữ; TopBar có nút chuyển `EN | VI`; `live` store mở `EventSource('/api/stream')`, tự kết nối lại, cung cấp `activity` và `onEvent(type, cb)`; `drawer` store dạng ngăn xếp `{type:'task'|'topic'|'plan'|'activity', ...}[]` với `open(x, {push})`, `back()`, `close()`.

- [ ] **Step 1:** Tests vitest: `router_parses_all_routes`; `i18n_falls_back_to_en`; `locales_have_identical_keys` (tập khoá `en.json` == `vi.json`); `initial_locale_from_navigator` (`vi-VN` → `vi`, `fr` → `en`, giá trị trong localStorage được ưu tiên); `drawer_push_back_close`.
- [ ] **Step 2–4:** FAIL → cài đặt (Vite build vào `web/dist`, `base: './'`; dev server proxy `/api` → `127.0.0.1:7420`) → PASS. Thêm job `web` vào `ci.yml` (ubuntu, Node 22): `npm ci && npm run check && npm test && npm run build` trong `web/`. Bố cục và token màu sáng/tối lấy từ prototype (`:root` + `prefers-color-scheme`).
- [ ] **Step 5:** Commit `feat(web): app shell, router, i18n, api and live store`.

### Task 14: Frontend — Tổng quan, Dự án, trang Plan

**Files:** Create `web/src/pages/{Home, Project, Plan}.svelte`, `web/src/components/{PlanCard, LiveNest, DocView, Toc, TaskRow}.svelte`

**Interfaces:** Consumes Task 13 + `GET /api/projects|plans|plans/:id|activity`.

Hành vi khớp prototype: thẻ plan có nhãn “📄 PLAN · project”, thanh tiến độ, chủ đề (bỏ `type/`), **khối LiveNest lồng trong thẻ** (plan → việc → trợ lý, câu theo locale), thẻ có hoạt động lên đầu nhóm; trang Plan có breadcrumb, quan hệ dạng câu, tab Tài liệu/Các việc, mục lục “On this page” sticky bên phải, hàng task bấm mở drawer; plan có `warnings` hiển thị hộp cảnh báo đầu trang; plan `structured=false` hiện nhãn “Imported”.

- [ ] **Step 1:** Test component (vitest + @testing-library/svelte): `plan_card_shows_live_nest_only_when_active`; `task_row_click_opens_drawer_not_navigation` (hash không đổi, drawer có 1 phần tử).
- [ ] **Step 2–4:** FAIL → cài đặt → PASS. **Step 5:** Commit `feat(web): home, project and plan pages`.

### Task 15: Frontend — Bảng công việc & panel chi tiết

**Files:** Create `web/src/pages/Board.svelte`, `web/src/components/{BoardColumns, TaskCard, Drawer, TaskPanel, TopicPanel, PlanPanel, ActivityPanel, Toast}.svelte`

Hành vi khớp prototype: bộ lọc dự án/chủ đề/chia cột (trạng thái | model | lĩnh vực) có “Đang hiện N/M” + “Xoá lọc”; kéo thả chỉ khi chia theo trạng thái → `PATCH`; TaskPanel có điều khiển phân đoạn trạng thái, “Claude đang làm gì” + nhật ký câu thường + ô “chi tiết kỹ thuật” hiện `raw`, độ khó → model giải thích bằng câu, phụ thuộc, chủ đề; TopicPanel có thống kê + danh sách + 2 nút lọc board / xem bản đồ; Esc đóng; “‹ Back” trong ngăn xếp; cập nhật live chỉ thay chữ, giữ vị trí cuộn.

- [ ] **Step 1:** Tests: `drag_to_done_calls_patch` (mock api); `filter_count_text`; `esc_closes_drawer`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS. **Step 5:** Commit `feat(web): task board and detail drawers`.

### Task 16: Frontend — Bản đồ kiến thức & tìm kiếm

**Files:** Create `web/src/pages/Map.svelte`, `web/src/components/{Graph, TopicList, Search}.svelte`

Graph dùng `d3-force` (chỉ module này), SVG, có zoom/pan + kéo nút; cạnh khai báo nét liền có mũi tên, `relates_to` nét chấm, chủ đề chung nét đứt (bật/tắt); chọn chủ đề làm mờ plan không liên quan; bấm nút mở PlanPanel. Search ở TopBar: `/` hoặc Ctrl/Cmd+K, debounce 120 ms gọi `/api/search`, nhóm Plan/Việc/Chủ đề, tô sáng không dấu (thuật toán chuẩn hoá từng ký tự như prototype), ↑↓/Enter/Esc, plan → điều hướng, việc/chủ đề → drawer.

- [ ] **Step 1:** Tests: `highlight_ignores_diacritics` (`hl("Bảo mật", ["bao"])` bọc đúng “Bảo”); `search_keyboard_enter_opens_selected`.
- [ ] **Step 2–4:** FAIL → cài đặt → PASS. Kiểm tra `npm run build` → gzip tổng JS ≤ 200 KB (script `npm run size`).
- [ ] **Step 5:** Commit `feat(web): knowledge map and global search`.

### Task 17: Nhúng UI, đo tài nguyên, phát hành

**Files:** Modify `src/server/assets.rs`, `build.rs` (báo lỗi rõ nếu thiếu `web/dist` khi build release), Create `scripts/bench-memory.sh`, `dist-workspace.toml`, `.github/workflows/release.yml`, cập nhật `README.md` (lệnh cài thật), `CHANGELOG.md` (`0.1.0`)

- [ ] **Step 1:** Test `tests/assets.rs::serves_index_and_spa_fallback` (`GET /` và `GET /plan/x` trả `index.html`, `GET /assets/<file>` có `Cache-Control: max-age=31536000, immutable`).
- [ ] **Step 2–3:** FAIL → nhúng `web/dist` bằng `rust-embed` (nén sẵn gzip, phục vụ theo `Accept-Encoding`) → PASS.
- [ ] **Step 4:** `scripts/bench-memory.sh`: sinh 1.000 plan giả vào `PLAN_ATLAS_HOME` tạm, chạy `plan-atlas --no-open`, đợi 10 s, đọc `VmRSS` từ `/proc/<pid>/status`, thoát mã 1 nếu > 40 MB; kiểm tra kích thước binary ≤ 20 MB. Thêm vào CI (Linux).
- [ ] **Step 5:** `dist init`: targets `x86_64/aarch64-unknown-linux-musl`, `x86_64/aarch64-apple-darwin`, `x86_64-pc-windows-msvc`; installers `shell, powershell, homebrew, npm`; job build web trước khi build Rust.
- [ ] **Step 6:** Thử tag `v0.1.0-rc.1` trên fork → cài bằng script shell trên máy sạch → gõ `plan-atlas` → trình duyệt mở, hỏi setup, thấy plan đã nhập.
- [ ] **Step 7:** Commit `build: embed UI, memory budget check and release pipeline`.
