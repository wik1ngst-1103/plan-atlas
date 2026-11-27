//! `plan-atlas mcp`: an MCP server over stdio that lets Claude Code author
//! plans and track task status.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use anyhow::Context;
use rmcp::handler::server::router::tool::ToolRouter;
use rmcp::handler::server::wrapper::Parameters;
use rmcp::model::{CallToolResult, ContentBlock, Implementation, ServerCapabilities, ServerConfig};
use rmcp::{ErrorData, ServerHandler, ServiceExt, tool, tool_handler, tool_router};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_yaml_ng::{Mapping, Value};
use time::{OffsetDateTime, UtcOffset};

use crate::config::Paths;
use crate::index::index_file;
use crate::ops::{OpError, lock_dir, safe_plan_path, set_task_status_op, valid_id};
use crate::plan::{
    Plan, TaskStatus, Warning, next_plan_id, parse_plan, plan_file_name, write_atomic,
};
use crate::project::{project_root_name, sanitize_project};
use crate::store::{PlanFilter, Store};
use crate::taxonomy::Taxonomy;

const SEARCH_LIMIT: usize = 20;
const UNTITLED: &str = "Untitled plan";

const INSTRUCTIONS: &str = "Plan Atlas stores implementation plans as Markdown files and tracks \
their tasks. Before writing a plan, call get_taxonomy (tags must come from it) and search_plans \
(to find related plans for the `related` field). Create plans with create_plan, revise them with \
update_plan, and call start_task / complete_task when you begin and finish a task.";

const CREATE_PLAN_DESC: &str = "Create a new plan file and index it. Returns {plan_id, path, \
warnings}. Before calling: call get_taxonomy (every tag must be one of its tags) and \
search_plans (list related plans in `related`).\n\n\
Required structure:\n\
1. YAML frontmatter between `---` lines with `title`, `tags` (list of taxonomy tags) and \
optionally `related` (list of `{to: <plan id>, type: depends_on|extends|relates_to}`). \
`id`, `project` and `created` are assigned by the server and overwritten if present.\n\
2. Sections as `## ` headings, in order: `## 01. Overview`, `## 02. Requirements & Scope`, \
`## 03. Technical Analysis`, `## 04. Task Breakdown`, `## 05. Risks & Rollback`.\n\
3. Under section 04, one `### TASK-NNN: <name>` heading per task (TASK-001, TASK-002, ...), \
immediately followed by metadata lines: `- complexity: 1-5`, optionally `- model: \
haiku|sonnet|opus` and `- effort: low|medium|high`, `- depends_on: [TASK-001]`, \
`- status: todo`, `- tags: [<taxonomy tag>]`; then a free Markdown description.\n\n\
The plan is never rejected for structure problems: fix anything reported in `warnings` with \
update_plan. `project` defaults to the current git repository's directory name.";

const UPDATE_PLAN_DESC: &str = "Replace the full Markdown of an existing plan (same structure \
as create_plan) and reindex it. The plan keeps its id, project and created date; those \
frontmatter keys are overwritten. Returns {plan_id, path, warnings}; fix any warnings with \
another update_plan call.";

#[derive(Debug, Deserialize, JsonSchema)]
pub struct ListPlansArgs {
    /// Only plans of this project (directory name under plans/). Omit for all projects.
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct SearchPlansArgs {
    /// Full-text query over plan titles, sections, tasks and tag labels.
    pub query: String,
    /// Keep only plans (and tasks) carrying all of these taxonomy tags.
    pub tags: Option<Vec<String>>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct PlanIdArgs {
    /// Plan id, e.g. PLAN-my-project-001.
    pub plan_id: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct CreatePlanArgs {
    /// The full plan Markdown, including YAML frontmatter.
    pub markdown: String,
    /// Project name; defaults to the current git repository's directory name.
    pub project: Option<String>,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct UpdatePlanArgs {
    /// Plan id, e.g. PLAN-my-project-001.
    pub plan_id: String,
    /// The full new plan Markdown, including YAML frontmatter.
    pub markdown: String,
}

#[derive(Debug, Deserialize, JsonSchema)]
pub struct TaskArgs {
    /// Plan id, e.g. PLAN-my-project-001.
    pub plan_id: String,
    /// Task id within the plan, e.g. TASK-001.
    pub task_id: String,
}

#[derive(Debug, Serialize)]
struct WriteResult {
    plan_id: String,
    path: PathBuf,
    warnings: Vec<Warning>,
}

#[derive(Debug, Serialize)]
struct TaxonomyTag<'a> {
    tag: &'a str,
    group: &'a str,
    label_en: &'a str,
    desc_en: &'a str,
    label_vi: &'a str,
    desc_vi: &'a str,
}

#[derive(Debug, Serialize)]
struct TaxonomyOut<'a> {
    tags: Vec<TaxonomyTag<'a>>,
}

struct Ctx {
    store: Store,
    tax: Taxonomy,
    paths: Paths,
    default_project: String,
    offset: UtcOffset,
}

impl Ctx {
    fn today(&self) -> String {
        let d = OffsetDateTime::now_utc().to_offset(self.offset).date();
        format!("{:04}-{:02}-{:02}", d.year(), u8::from(d.month()), d.day())
    }
}

#[derive(Clone)]
pub struct PlanAtlasMcp {
    ctx: Arc<Ctx>,
    tool_router: ToolRouter<Self>,
}

/// Run the MCP server on stdin/stdout until the client disconnects.
pub fn run(paths: Paths) -> anyhow::Result<()> {
    // Read the local offset while still single-threaded (`time` refuses to
    // read it once other threads exist).
    let offset = UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC);
    paths.ensure().context("creating data directories")?;
    let tax = Taxonomy::load(&paths);
    let store = Store::open(&paths.db, Arc::new(tax.clone())).context("opening database")?;
    let cwd = std::env::current_dir().context("reading current directory")?;
    let ctx = Ctx {
        store,
        tax,
        paths,
        default_project: default_project(&cwd),
        offset,
    };
    let rt = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()?;
    rt.block_on(async {
        let service = PlanAtlasMcp::new(ctx)
            .serve(rmcp::transport::stdio())
            .await
            .context("starting MCP server")?;
        service.waiting().await.context("MCP server")?;
        anyhow::Ok(())
    })?;
    // A pending stdin read cannot be cancelled; don't wait for it.
    rt.shutdown_background();
    Ok(())
}

/// Repository directory name of `cwd` (main repository for worktrees).
fn default_project(cwd: &Path) -> String {
    project_root_name(cwd).unwrap_or_else(|| "unsorted".to_string())
}

/// Split `---\n<yaml>---\n<body>`; returns (yaml, body). Body is untouched.
fn split_frontmatter(md: &str) -> (Option<&str>, &str) {
    let rest = md.strip_prefix('\u{feff}').unwrap_or(md);
    let Some(rest) = rest
        .strip_prefix("---\n")
        .or_else(|| rest.strip_prefix("---\r\n"))
    else {
        return (None, md);
    };
    let mut pos = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return (Some(&rest[..pos]), &rest[pos + line.len()..]);
        }
        pos += line.len();
    }
    (None, md)
}

fn first_h1(body: &str) -> Option<String> {
    let mut in_fence = false;
    for line in body.lines() {
        let t = line.trim_start();
        if t.starts_with("```") || t.starts_with("~~~") {
            in_fence = !in_fence;
        } else if !in_fence && let Some(title) = line.strip_prefix("# ") {
            let title = title.trim();
            if !title.is_empty() {
                return Some(title.to_string());
            }
        }
    }
    None
}

/// Set `id`, `project`, `created` in the frontmatter (creating one with a
/// `title` if absent); everything else, including the body, is kept as is.
fn with_frontmatter(md: &str, id: &str, project: &str, created: &str) -> Result<String, OpError> {
    let (yaml, body) = split_frontmatter(md);
    let mut map = match yaml {
        Some(y) if y.trim().is_empty() => Mapping::new(),
        Some(y) => match serde_yaml_ng::from_str::<Value>(y) {
            Ok(Value::Mapping(m)) => m,
            Ok(Value::Null) => Mapping::new(),
            Ok(_) => {
                return Err(OpError::Invalid(
                    "frontmatter must be a YAML mapping (key: value lines)".to_string(),
                ));
            }
            Err(e) => {
                return Err(OpError::Invalid(format!(
                    "frontmatter is not valid YAML: {e}"
                )));
            }
        },
        None => Mapping::new(),
    };
    let mut set = |k: &str, v: &str| {
        map.insert(Value::String(k.to_string()), Value::String(v.to_string()));
    };
    set("id", id);
    set("project", project);
    if yaml.is_none() {
        set(
            "title",
            &first_h1(body).unwrap_or_else(|| UNTITLED.to_string()),
        );
    }
    set("created", created);
    let yaml = serde_yaml_ng::to_string(&Value::Mapping(map)).map_err(OpError::internal)?;
    Ok(format!("---\n{yaml}---\n{body}"))
}

/// `<dir>/<name>.md`, or `<name>-2.md`, `-3`, ... if taken.
fn free_path(dir: &Path, file_name: &str) -> PathBuf {
    let stem = file_name.strip_suffix(".md").unwrap_or(file_name);
    let mut path = dir.join(file_name);
    let mut n = 2;
    while path.exists() {
        path = dir.join(format!("{stem}-{n}.md"));
        n += 1;
    }
    path
}

/// Write, reindex and report the plan's warnings (parse + validate).
fn write_and_index(ctx: &Ctx, path: &Path, bytes: &[u8]) -> Result<WriteResult, OpError> {
    write_atomic(path, bytes).map_err(OpError::internal)?;
    let id = index_file(&ctx.store, &ctx.tax, path)
        .map_err(OpError::internal)?
        .ok_or_else(|| OpError::internal(format!("{} vanished", path.display())))?;
    let plan = get_plan(ctx, &id)?;
    Ok(WriteResult {
        plan_id: id,
        path: path.to_path_buf(),
        warnings: plan.warnings,
    })
}

fn get_plan(ctx: &Ctx, plan_id: &str) -> Result<Plan, OpError> {
    let not_found = || OpError::NotFound(format!("plan not found: {plan_id}"));
    if !valid_id(plan_id) {
        return Err(not_found());
    }
    ctx.store
        .get_plan(plan_id)
        .map_err(OpError::internal)?
        .ok_or_else(not_found)
}

/// Plan ids of `project`: the index plus the frontmatter of every visible
/// `*.md` in `dir` (files the index may not have picked up yet).
fn known_plan_ids(ctx: &Ctx, project: &str, dir: &Path) -> Result<Vec<String>, OpError> {
    let mut ids = ctx
        .store
        .plan_ids_for_project(project)
        .map_err(OpError::internal)?;
    for entry in std::fs::read_dir(dir).map_err(OpError::internal)?.flatten() {
        let path = entry.path();
        let visible = !entry.file_name().to_string_lossy().starts_with('.');
        if visible
            && path.extension().is_some_and(|e| e == "md")
            && let Ok(bytes) = std::fs::read(&path)
        {
            ids.push(parse_plan(&bytes, &path).id);
        }
    }
    Ok(ids)
}

fn create_plan(ctx: &Ctx, args: CreatePlanArgs) -> Result<WriteResult, OpError> {
    let project = match &args.project {
        Some(p) => sanitize_project(p),
        None => ctx.default_project.clone(),
    };
    let dir = ctx.paths.plans.join(&project);
    let _lock = lock_dir(&dir)?;
    let id = next_plan_id(&project, &known_plan_ids(ctx, &project, &dir)?);
    let created = ctx.today();
    let text = with_frontmatter(&args.markdown, &id, &project, &created)?;
    let title = parse_plan(text.as_bytes(), Path::new(UNTITLED)).title;
    let path = free_path(&dir, &plan_file_name(&created, &title));
    write_and_index(ctx, &path, text.as_bytes())
}

fn update_plan(ctx: &Ctx, args: UpdatePlanArgs) -> Result<WriteResult, OpError> {
    let path = safe_plan_path(&ctx.store, &ctx.paths, &args.plan_id)?;
    let dir = path
        .parent()
        .ok_or_else(|| OpError::internal("plan path has no parent"))?;
    let _lock = lock_dir(dir)?;
    let old = get_plan(ctx, &args.plan_id)?;
    let created = old.created.unwrap_or_else(|| ctx.today());
    let text = with_frontmatter(&args.markdown, &old.id, &old.project, &created)?;
    write_and_index(ctx, &path, text.as_bytes())
}

fn search_plans(ctx: &Ctx, args: SearchPlansArgs) -> Result<crate::store::SearchResults, OpError> {
    let mut found = ctx
        .store
        .search(&args.query, SEARCH_LIMIT)
        .map_err(OpError::internal)?;
    if let Some(tags) = args.tags.filter(|t| !t.is_empty()) {
        let has_all = |have: &[String]| tags.iter().all(|t| have.contains(t));
        found.plans.retain(|p| has_all(&p.tags));
        // Tasks inherit their plan's topics.
        found.tasks.retain(|t| {
            let all: Vec<String> = t.task.tags.iter().chain(&t.plan_tags).cloned().collect();
            has_all(&all)
        });
        found.topics.retain(|t| tags.contains(&t.tag));
    }
    Ok(found)
}

fn taxonomy(tax: &Taxonomy) -> TaxonomyOut<'_> {
    TaxonomyOut {
        tags: tax
            .tags
            .iter()
            .map(|(tag, i)| TaxonomyTag {
                tag,
                group: &i.group,
                label_en: &i.label_en,
                desc_en: &i.desc_en,
                label_vi: &i.label_vi,
                desc_vi: &i.desc_vi,
            })
            .collect(),
    }
}

fn to_result<T: Serialize>(r: Result<T, OpError>) -> CallToolResult {
    let text = r.and_then(|v| serde_json::to_string(&v).map_err(OpError::internal));
    match text {
        Ok(json) => CallToolResult::success(vec![ContentBlock::text(json)]),
        Err(e) => {
            if let OpError::Internal(m) = &e {
                eprintln!("plan-atlas: mcp: {m}");
            }
            CallToolResult::error(vec![ContentBlock::text(e.to_string())])
        }
    }
}

#[tool_router]
impl PlanAtlasMcp {
    fn new(ctx: Ctx) -> Self {
        PlanAtlasMcp {
            ctx: Arc::new(ctx),
            tool_router: Self::tool_router(),
        }
    }

    /// Run a store operation on the blocking pool and wrap it as a tool result.
    async fn blocking<T, F>(&self, f: F) -> Result<CallToolResult, ErrorData>
    where
        T: Serialize + Send + 'static,
        F: FnOnce(&Ctx) -> Result<T, OpError> + Send + 'static,
    {
        let ctx = self.ctx.clone();
        let r = tokio::task::spawn_blocking(move || f(&ctx))
            .await
            .map_err(|e| ErrorData::internal_error(e.to_string(), None))?;
        Ok(to_result(r))
    }

    #[tool(
        description = "List indexed plans (id, project, title, tags, created, task and done \
counts, warning count), optionally for one project.",
        annotations(read_only_hint = true)
    )]
    async fn list_plans(
        &self,
        Parameters(args): Parameters<ListPlansArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        self.blocking(move |ctx| {
            let f = PlanFilter {
                project: args.project,
                tag: None,
            };
            ctx.store.list_plans(&f).map_err(OpError::internal)
        })
        .await
    }

    #[tool(
        description = "Full-text search over all plans and tasks (accent-insensitive). Returns \
{plans, tasks, topics}. Call this before create_plan to find plans to list under `related`. \
`tags` keeps only results carrying all the given taxonomy tags.",
        annotations(read_only_hint = true)
    )]
    async fn search_plans(
        &self,
        Parameters(args): Parameters<SearchPlansArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        self.blocking(move |ctx| search_plans(ctx, args)).await
    }

    #[tool(
        description = "Get one plan in full: frontmatter fields, sections, tasks (id, name, \
complexity, model, effort, depends_on, status, tags, body) and structure warnings.",
        annotations(read_only_hint = true)
    )]
    async fn get_plan(
        &self,
        Parameters(args): Parameters<PlanIdArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        self.blocking(move |ctx| get_plan(ctx, &args.plan_id)).await
    }

    #[tool(
        description = "List the allowed tags, each with its group and English/Vietnamese label \
and description. Plan and task tags must come from this list; call it before create_plan.",
        annotations(read_only_hint = true)
    )]
    async fn get_taxonomy(&self) -> Result<CallToolResult, ErrorData> {
        Ok(to_result(Ok(taxonomy(&self.ctx.tax))))
    }

    #[tool(description = CREATE_PLAN_DESC)]
    async fn create_plan(
        &self,
        Parameters(args): Parameters<CreatePlanArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        self.blocking(move |ctx| create_plan(ctx, args)).await
    }

    #[tool(description = UPDATE_PLAN_DESC)]
    async fn update_plan(
        &self,
        Parameters(args): Parameters<UpdatePlanArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        self.blocking(move |ctx| update_plan(ctx, args)).await
    }

    #[tool(
        description = "Mark a task as in_progress. Call this right before you start working on \
the task; returns the updated plan."
    )]
    async fn start_task(
        &self,
        Parameters(args): Parameters<TaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        self.blocking(move |ctx| {
            set_task_status_op(
                &ctx.store,
                &ctx.tax,
                &ctx.paths,
                &args.plan_id,
                &args.task_id,
                TaskStatus::InProgress,
            )
        })
        .await
    }

    #[tool(
        description = "Mark a task as done. Call this when the task is finished and verified; \
returns the updated plan."
    )]
    async fn complete_task(
        &self,
        Parameters(args): Parameters<TaskArgs>,
    ) -> Result<CallToolResult, ErrorData> {
        self.blocking(move |ctx| {
            set_task_status_op(
                &ctx.store,
                &ctx.tax,
                &ctx.paths,
                &args.plan_id,
                &args.task_id,
                TaskStatus::Done,
            )
        })
        .await
    }
}

#[tool_handler(router = self.tool_router)]
impl ServerHandler for PlanAtlasMcp {
    fn get_info(&self) -> ServerConfig {
        ServerConfig::new(ServerCapabilities::builder().enable_tools().build())
            .with_server_info(Implementation::new("plan-atlas", env!("CARGO_PKG_VERSION")))
            .with_instructions(INSTRUCTIONS)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn project_sanitizing() {
        assert_eq!(sanitize_project("../x"), "x");
        assert_eq!(sanitize_project("My Repo_2"), "my-repo-2");
        assert_eq!(sanitize_project("../.."), "unsorted");
    }

    #[test]
    fn frontmatter_overwrites_only_assigned_keys() {
        let md = "---\nid: OLD\ntitle: T\nowner: a\n---\nbody\n---\n";
        let out = with_frontmatter(md, "PLAN-p-001", "p", "2026-11-27").unwrap();
        assert_eq!(
            out,
            "---\nid: PLAN-p-001\ntitle: T\nowner: a\nproject: p\ncreated: 2026-11-27\n---\nbody\n---\n"
        );
    }

    #[test]
    fn invalid_frontmatter_is_an_error() {
        assert!(with_frontmatter("---\n: [\n---\nx\n", "i", "p", "d").is_err());
    }
}
