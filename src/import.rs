//! Import Markdown plans into the store: Claude Code's own plans
//! (`~/.claude/plans/*.md`), any files or folders given on the command line,
//! and files uploaded through the web UI. Sources are only ever read; copies
//! carry `imported_from` / `imported_hash` frontmatter so re-runs are
//! idempotent.

use std::collections::{HashMap, HashSet};
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde::Serialize;
use serde_yaml_ng::{Mapping, Value};
use ts_rs::TS;

use crate::config::Paths;
use crate::ops::{lock_dir, try_lock_dir};
use crate::plan::slug::slugify;
use crate::plan::{next_plan_id, write_atomic};
use crate::project::{project_root_name, repo_project_name, sanitize_project};

const MAX_TRANSCRIPT_BYTES: u64 = 50 * 1024 * 1024;
const UNSORTED: &str = "unsorted";

#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct ImportReport {
    /// Copies written (new, or updated because the source changed).
    pub imported: usize,
    /// Sources already imported with the same content, or inside the store.
    pub skipped: usize,
    /// Of `imported`, those that went to `unsorted`.
    pub unsorted: usize,
    pub failed: usize,
    /// The plans written by this run, in import order.
    pub plans: Vec<ImportedPlan>,
    /// One entry per failed source.
    pub failures: Vec<ImportFailure>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct ImportedPlan {
    pub id: String,
    pub project: String,
    pub title: String,
    /// The copy in the store.
    #[serde(skip)]
    #[ts(skip)]
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, TS)]
#[ts(export, export_to = "../web/src/lib/generated/")]
pub struct ImportFailure {
    /// The path (or uploaded name) that could not be imported.
    pub source: String,
    pub error: String,
}

impl ImportReport {
    /// `Imported N, skipped M (K unsorted[, F failed])`.
    pub fn summary(&self) -> String {
        let failed = if self.failed > 0 {
            format!(", {} failed", self.failed)
        } else {
            String::new()
        };
        format!(
            "Imported {}, skipped {} ({} unsorted{failed})",
            self.imported, self.skipped, self.unsorted
        )
    }
}

/// Split `---` frontmatter from the body. Returns the parsed mapping, the
/// body, and (when a leading block exists but is not a valid mapping) the raw
/// block text so the caller can preserve it.
fn split_frontmatter_full(text: &str) -> (Option<Mapping>, &str, Option<&str>) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return (None, text, None);
    };
    let mut pos = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            let after = &rest[pos + line.len()..];
            return match serde_yaml_ng::from_str::<Value>(&rest[..pos]) {
                Ok(Value::Mapping(m)) => (Some(m), after, None),
                Ok(Value::Null) => (Some(Mapping::new()), after, None),
                _ => (None, after, Some(&text[..text.len() - after.len()])),
            };
        }
        pos += line.len();
    }
    (None, text, None)
}

fn split_frontmatter(text: &str) -> (Option<Mapping>, &str) {
    let (m, body, _) = split_frontmatter_full(text);
    (m, body)
}

fn str_field(m: &Mapping, key: &str) -> Option<String> {
    m.get(key).and_then(Value::as_str).map(str::to_string)
}

fn walk_md(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = fs::read_dir(dir) else { return };
    let mut entries: Vec<_> = rd.flatten().map(|e| e.path()).collect();
    entries.sort();
    for p in entries {
        if p.is_dir() {
            walk_md(&p, out);
        } else if p.extension().is_some_and(|e| e == "md") {
            out.push(p);
        }
    }
}

fn frontmatter_of(path: &Path) -> Option<Mapping> {
    let bytes = fs::read(path).ok()?;
    split_frontmatter(&String::from_utf8_lossy(&bytes)).0
}

/// Find which project produced `plan_file_name` by scanning Claude Code
/// transcripts under `claude_home/projects/*/*.jsonl`.
pub fn infer_project(plan_file_name: &str, claude_home: &Path) -> Option<String> {
    infer_projects(&[plan_file_name.to_string()], claude_home).remove(plan_file_name)
}

/// [`infer_projects_with`] reading transcripts from disk.
pub fn infer_projects(names: &[String], claude_home: &Path) -> HashMap<String, String> {
    infer_projects_with(names, claude_home, |p| {
        fs::File::open(p).ok().map(BufReader::new)
    })
}

/// Map plan file names to projects in a single pass over the transcripts
/// under `claude_home/projects/*/*.jsonl`, newest first: each transcript is
/// opened at most once and the pass stops as soon as every name is resolved.
/// Transcripts over 50 MB are skipped. `open` reads one transcript.
pub fn infer_projects_with<R: BufRead>(
    names: &[String],
    claude_home: &Path,
    mut open: impl FnMut(&Path) -> Option<R>,
) -> HashMap<String, String> {
    let mut found = HashMap::new();
    let mut pending: Vec<&str> = names.iter().map(String::as_str).collect();
    pending.sort_unstable();
    pending.dedup();
    if pending.is_empty() {
        return found;
    }
    for file in transcripts_newest_first(claude_home) {
        let Some(reader) = open(&file) else { continue };
        for (name, cwd) in cwds_for_matches(reader, &pending) {
            if let Some(project) = project_root_name(Path::new(&cwd)) {
                found.insert(name.to_string(), project);
            }
        }
        pending.retain(|n| !found.contains_key(*n));
        if pending.is_empty() {
            break;
        }
    }
    found
}

/// `*.jsonl` under `claude_home/projects/*/` of at most 50 MB, newest first.
fn transcripts_newest_first(claude_home: &Path) -> Vec<PathBuf> {
    let Ok(rd) = fs::read_dir(claude_home.join("projects")) else {
        return Vec::new();
    };
    let mut files: Vec<(std::time::SystemTime, PathBuf)> = Vec::new();
    for dir in rd.flatten().map(|e| e.path()).filter(|p| p.is_dir()) {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        for e in rd.flatten() {
            let path = e.path();
            if path.extension().is_none_or(|x| x != "jsonl") {
                continue;
            }
            let Ok(meta) = e.metadata() else { continue };
            if meta.len() > MAX_TRANSCRIPT_BYTES {
                continue;
            }
            let mtime = meta.modified().unwrap_or(std::time::UNIX_EPOCH);
            files.push((mtime, path));
        }
    }
    files.sort_by(|a, b| b.0.cmp(&a.0).then_with(|| a.1.cmp(&b.1)));
    files.into_iter().map(|(_, p)| p).collect()
}

/// For each needle (a plan file name): the first transcript line where Claude
/// wrote or edited that plan file — a `tool_use` of Write, Edit or MultiEdit
/// whose `file_path` is `…/plans/<needle>` — decides, using that line's `cwd`,
/// else the nearest earlier one. Lines that merely mention the name (`ls`
/// output, Read calls, chat text) do not count. Needles without a cwd are left
/// out.
fn cwds_for_matches<'a>(mut reader: impl BufRead, needles: &[&'a str]) -> Vec<(&'a str, String)> {
    let mut out = Vec::new();
    let mut open: Vec<&str> = needles.to_vec();
    let mut last_cwd: Option<String> = None;
    let mut buf = Vec::new();
    while !open.is_empty() {
        buf.clear();
        match reader.read_until(b'\n', &mut buf) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
        let line = String::from_utf8_lossy(&buf);
        let has_cwd = line.contains("\"cwd\"");
        let candidate = line.contains("tool_use") && open.iter().any(|n| line.contains(n));
        if !has_cwd && !candidate {
            continue;
        }
        let Ok(v) = serde_json::from_str::<serde_json::Value>(&line) else {
            continue;
        };
        let cwd = v.get("cwd").and_then(|c| c.as_str()).map(str::to_string);
        if candidate {
            let written = written_plan_files(&v);
            open.retain(|n| {
                if !written.iter().any(|w| w == n) {
                    return true;
                }
                if let Some(c) = cwd.as_ref().or(last_cwd.as_ref()) {
                    out.push((*n, c.clone()));
                }
                false
            });
        }
        if cwd.is_some() {
            last_cwd = cwd;
        }
    }
    out
}

/// File names of plan files (`…/plans/<name>`) written or edited by the
/// Write / Edit / MultiEdit tool calls in one transcript entry.
fn written_plan_files(entry: &serde_json::Value) -> Vec<String> {
    let Some(items) = entry.pointer("/message/content").and_then(|c| c.as_array()) else {
        return Vec::new();
    };
    items
        .iter()
        .filter(|it| it.get("type").and_then(|t| t.as_str()) == Some("tool_use"))
        .filter(|it| {
            matches!(
                it.get("name").and_then(|t| t.as_str()),
                Some("Write" | "Edit" | "MultiEdit")
            )
        })
        .filter_map(|it| it.pointer("/input/file_path").and_then(|p| p.as_str()))
        .filter_map(|fp| {
            // Accept both separators: transcripts may come from Windows.
            let mut parts = fp.rsplit(['/', '\\']);
            let name = parts.next()?;
            (parts.next()? == "plans").then(|| name.to_string())
        })
        .collect()
}

struct Known {
    hash: String,
    dest: PathBuf,
}

fn build_yaml(mut fm: Mapping) -> String {
    // serde_yaml_ng ends with a newline already.
    let y = serde_yaml_ng::to_string(&Value::Mapping(std::mem::take(&mut fm))).unwrap_or_default();
    format!("---\n{y}---\n")
}

fn first_heading(body: &str) -> Option<String> {
    body.lines()
        .find_map(|l| l.strip_prefix("# "))
        .map(|t| t.trim().to_string())
        .filter(|t| !t.is_empty())
}

fn project_ids(dir: &Path) -> Vec<String> {
    let mut files = Vec::new();
    walk_md(dir, &mut files);
    files
        .iter()
        .filter_map(|p| frontmatter_of(p))
        .filter_map(|m| str_field(&m, "id"))
        .collect()
}

/// Most Markdown files one `import_paths` call collects from directories.
pub const MAX_IMPORT_FILES: usize = 2000;

/// Directory names never descended into (besides hidden ones).
const SKIP_DIRS: [&str; 2] = ["node_modules", "target"];

/// One uploaded file: its (relative) name as sent by the browser, and content.
#[derive(Debug, Clone)]
pub struct Upload {
    pub path: String,
    pub bytes: Vec<u8>,
}

/// What one source becomes when it is new to the store.
struct Source<'s> {
    /// `imported_from` value: canonical path, or `upload:<relative path>`.
    from: String,
    /// File name for a new copy (made unique in its directory).
    name: String,
    /// Title when the source has neither frontmatter nor a `# ` heading.
    fallback_title: String,
    bytes: &'s [u8],
    /// Project directory for a new copy (already sanitized).
    project: String,
}

/// Shared state of one import run: what the store already holds, ids per
/// directory, and the per-project locks held while writing.
struct Importer<'a> {
    paths: &'a Paths,
    known: HashMap<String, Known>,
    ids_by_dir: HashMap<PathBuf, Vec<String>>,
    /// `ops::lock_dir` locks, kept until the run ends so cached ids stay valid.
    locks: HashMap<PathBuf, fs::File>,
    report: ImportReport,
}

impl<'a> Importer<'a> {
    fn new(paths: &'a Paths) -> Self {
        let mut known = HashMap::new();
        let mut existing = Vec::new();
        walk_md(&paths.plans, &mut existing);
        for p in existing {
            if let Some(m) = frontmatter_of(&p)
                && let (Some(from), Some(hash)) = (
                    str_field(&m, "imported_from"),
                    str_field(&m, "imported_hash"),
                )
            {
                known.insert(from, Known { hash, dest: p });
            }
        }
        Importer {
            paths,
            known,
            ids_by_dir: HashMap::new(),
            locks: HashMap::new(),
            report: ImportReport::default(),
        }
    }

    fn fail(&mut self, source: impl Into<String>, error: impl std::fmt::Display) {
        self.report.failed += 1;
        self.report.failures.push(ImportFailure {
            source: source.into(),
            error: error.to_string(),
        });
    }

    /// Take `dir`'s plan lock (as the MCP server does) unless already held.
    /// Never blocks while holding other locks: on contention every lock (and
    /// the ids cached under them) is released first, so two concurrent
    /// imports cannot deadlock.
    fn lock(&mut self, dir: &Path) -> Result<()> {
        if self.locks.contains_key(dir) {
            return Ok(());
        }
        let file = match try_lock_dir(dir)? {
            Some(f) => f,
            None => {
                self.locks.clear();
                self.ids_by_dir.clear();
                lock_dir(dir)?
            }
        };
        self.locks.insert(dir.to_path_buf(), file);
        Ok(())
    }

    /// Copy one source into the store: skipped when the same content was
    /// imported before, an in-place update (same id) when it changed.
    fn import_one(&mut self, src: Source<'_>) -> Result<()> {
        let hash = blake3::hash(src.bytes).to_hex().to_string();
        let previous = match self.known.get(&src.from) {
            Some(k) if k.hash == hash => {
                self.report.skipped += 1;
                return Ok(());
            }
            Some(k) => Some(k.dest.clone()),
            None => None,
        };
        let text = String::from_utf8_lossy(src.bytes).into_owned();
        let (fm, body, invalid) = split_frontmatter_full(&text);

        let (dest, project, id) = match previous {
            Some(dest) => {
                self.lock(dest.parent().context("plan path has no parent")?)?;
                let old = frontmatter_of(&dest).unwrap_or_default();
                let project = str_field(&old, "project").unwrap_or_else(|| UNSORTED.into());
                (dest, project, str_field(&old, "id"))
            }
            None => {
                let dir = self.paths.plans.join(&src.project);
                self.lock(&dir)?;
                fs::create_dir_all(&dir)?;
                (free_name(&dir, &src.name), src.project.clone(), None)
            }
        };
        let dir = dest
            .parent()
            .context("plan path has no parent")?
            .to_path_buf();
        let is_unsorted = dir == self.paths.unsorted;

        let id = match id {
            Some(id) => id,
            None => {
                let ids = self
                    .ids_by_dir
                    .entry(dir.clone())
                    .or_insert_with(|| project_ids(&dir));
                let id = next_plan_id(&project, ids);
                ids.push(id.clone());
                id
            }
        };

        let mut map = fm.unwrap_or_else(|| {
            let title = first_heading(body).unwrap_or_else(|| src.fallback_title.clone());
            let mut m = Mapping::new();
            m.insert("title".into(), title.into());
            m
        });
        let title = str_field(&map, "title").unwrap_or_else(|| src.fallback_title.clone());
        map.insert("id".into(), id.clone().into());
        map.insert("project".into(), project.clone().into());
        map.insert("imported_from".into(), src.from.clone().into());
        map.insert("imported_hash".into(), hash.clone().into());
        let mut out = format!("{}{}", build_yaml(map), body);
        if let Some(raw) = invalid {
            if !out.ends_with('\n') {
                out.push('\n');
            }
            out.push_str(&format!(
                "\n<!-- original frontmatter (invalid) -->\n```yaml\n{raw}```\n"
            ));
        }
        write_atomic(&dest, out.as_bytes())
            .with_context(|| format!("writing {}", dest.display()))?;
        self.known.insert(
            src.from,
            Known {
                hash,
                dest: dest.clone(),
            },
        );
        self.report.imported += 1;
        if is_unsorted {
            self.report.unsorted += 1;
        }
        self.report.plans.push(ImportedPlan {
            id,
            project,
            title,
            path: dest,
        });
        Ok(())
    }

    /// Import files from disk. A new file's project: `explicit` > the git
    /// repository it sits in (when `use_git`) > the Claude session that
    /// wrote it > `unsorted`. Files inside the store are skipped.
    fn import_files(
        &mut self,
        files: Vec<PathBuf>,
        explicit: Option<String>,
        use_git: bool,
        claude_home: &Path,
    ) {
        let store_root = fs::canonicalize(&self.paths.plans).ok();
        let mut seen = HashSet::new();
        let mut todo = Vec::new();
        for file in files {
            let key = source_key(&file);
            if !seen.insert(key.clone()) {
                continue;
            }
            if store_root
                .as_ref()
                .is_some_and(|root| Path::new(&key).starts_with(root))
            {
                self.report.skipped += 1;
                continue;
            }
            let repo = match (&explicit, use_git) {
                (None, true) => file.parent().and_then(repo_project_name),
                _ => None,
            };
            todo.push((file, key, repo));
        }
        let to_infer: Vec<String> = todo
            .iter()
            .filter(|(_, key, repo)| {
                explicit.is_none() && repo.is_none() && !self.known.contains_key(key)
            })
            .filter_map(|(f, _, _)| Some(f.file_name()?.to_string_lossy().into_owned()))
            .collect();
        let inferred = infer_projects(&to_infer, claude_home);

        for (file, key, repo) in todo {
            let name = file
                .file_name()
                .map(|n| n.to_string_lossy().into_owned())
                .unwrap_or_default();
            let project = explicit
                .clone()
                .or(repo)
                .or_else(|| inferred.get(&name).cloned())
                .unwrap_or_else(|| UNSORTED.to_string());
            let res = fs::read(&file)
                .with_context(|| format!("reading {}", file.display()))
                .and_then(|bytes| {
                    self.import_one(Source {
                        from: key,
                        fallback_title: file
                            .file_stem()
                            .unwrap_or_default()
                            .to_string_lossy()
                            .into_owned(),
                        name,
                        bytes: &bytes,
                        project,
                    })
                });
            if let Err(e) = res {
                self.fail(file.display().to_string(), format!("{e:#}"));
            }
        }
    }
}

/// Copy every `claude_home/plans/*.md` into the store. Never modifies sources.
pub fn import_claude_plans(claude_home: &Path, paths: &Paths) -> Result<ImportReport> {
    let mut imp = Importer::new(paths);
    let Ok(rd) = fs::read_dir(claude_home.join("plans")) else {
        return Ok(imp.report);
    };
    let mut sources: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && is_md(p))
        .collect();
    sources.sort();
    imp.import_files(sources, None, false, claude_home);
    Ok(imp.report)
}

/// Import Markdown files and directory trees (recursively: `*.md` only,
/// hidden entries, `node_modules` and `target` skipped, at most
/// [`MAX_IMPORT_FILES`] files). `project` overrides the project of new
/// copies; otherwise each file's git repository, then the Claude session
/// that wrote it, decides, else `unsorted`. Never modifies sources; files
/// already in the store are skipped. A bad path or unreadable file is
/// recorded in `failures` and the rest still imports.
pub fn import_paths(
    paths: &[PathBuf],
    project: Option<&str>,
    claude_home: &Path,
    store_paths: &Paths,
) -> Result<ImportReport> {
    let mut imp = Importer::new(store_paths);
    let mut files = Vec::new();
    let mut truncated = false;
    for p in paths {
        match fs::metadata(p) {
            Err(e) => imp.fail(p.display().to_string(), e),
            Ok(m) if m.is_dir() => truncated |= !collect_md_tree(p, &mut files),
            Ok(_) if is_md(p) => files.push(p.clone()),
            Ok(_) => imp.fail(p.display().to_string(), "not a Markdown (.md) file"),
        }
    }
    if truncated {
        eprintln!(
            "plan-atlas: warning: more than {MAX_IMPORT_FILES} Markdown files found; \
             importing only the first {MAX_IMPORT_FILES}"
        );
    }
    let explicit = project.map(sanitize_project);
    imp.import_files(files, explicit, true, claude_home);
    Ok(imp.report)
}

/// Import files uploaded through the web UI into `project` (sanitized), or
/// `unsorted`. Each is recorded as `imported_from: upload:<relative path>`,
/// so uploading the same file again is skipped.
pub fn import_uploads(
    files: &[Upload],
    project: Option<&str>,
    paths: &Paths,
) -> Result<ImportReport> {
    let mut imp = Importer::new(paths);
    let project = project
        .map(sanitize_project)
        .unwrap_or_else(|| UNSORTED.to_string());
    for f in files {
        let rel = clean_relative(&f.path);
        let base = rel.rsplit('/').next().unwrap_or_default();
        let stem = Path::new(base)
            .file_stem()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();
        let slug = slugify(&stem);
        let name = format!("{}.md", if slug.is_empty() { "plan" } else { &slug });
        let res = imp.import_one(Source {
            from: format!("upload:{rel}"),
            name,
            fallback_title: stem,
            bytes: &f.bytes,
            project: project.clone(),
        });
        if let Err(e) = res {
            imp.fail(rel, format!("{e:#}"));
        }
    }
    Ok(imp.report)
}

/// `a\b/../c.md` → `a/b/c.md`: forward slashes, no empty, `.` or `..` parts.
fn clean_relative(path: &str) -> String {
    path.split(['/', '\\'])
        .filter(|s| !s.is_empty() && *s != "." && *s != "..")
        .collect::<Vec<_>>()
        .join("/")
}

fn is_md(p: &Path) -> bool {
    p.extension().is_some_and(|e| e.eq_ignore_ascii_case("md"))
}

/// Push the `*.md` files under `dir` (sorted, depth-first) onto `out`.
/// Hidden entries, [`SKIP_DIRS`] and symlinked directories are skipped.
/// Returns `false` once `out` reached [`MAX_IMPORT_FILES`].
fn collect_md_tree(dir: &Path, out: &mut Vec<PathBuf>) -> bool {
    let Ok(rd) = fs::read_dir(dir) else {
        return true;
    };
    let mut entries: Vec<_> = rd.flatten().collect();
    entries.sort_by_key(|e| e.file_name());
    for e in entries {
        let name = e.file_name();
        let name = name.to_string_lossy();
        if name.starts_with('.') {
            continue;
        }
        let Ok(ft) = e.file_type() else { continue };
        let path = e.path();
        if ft.is_dir() {
            if SKIP_DIRS.contains(&name.as_ref()) {
                continue;
            }
            if !collect_md_tree(&path, out) {
                return false;
            }
        } else if is_md(&path) && path.is_file() {
            if out.len() >= MAX_IMPORT_FILES {
                return false;
            }
            out.push(path);
        }
    }
    true
}

/// Canonical source path, as recorded in `imported_from`.
fn source_key(src: &Path) -> String {
    let abs = fs::canonicalize(src).unwrap_or_else(|_| src.to_path_buf());
    abs.to_string_lossy().to_string()
}

/// `name`, or `stem-2.ext`, `stem-3.ext`... when taken by another file.
fn free_name(dir: &Path, name: &str) -> PathBuf {
    let first = dir.join(name);
    if !first.exists() {
        return first;
    }
    let p = Path::new(name);
    let stem = p.file_stem().unwrap_or_default().to_string_lossy();
    let ext = p.extension().map(|e| format!(".{}", e.to_string_lossy()));
    (2..)
        .map(|n| dir.join(format!("{stem}-{n}{}", ext.as_deref().unwrap_or(""))))
        .find(|c| !c.exists())
        .unwrap()
}
