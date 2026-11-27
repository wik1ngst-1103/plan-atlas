//! Import existing Claude Code plans (`~/.claude/plans/*.md`) into the store.
//! Sources are only ever read; copies carry `imported_from` / `imported_hash`
//! frontmatter so re-runs are idempotent.

use std::collections::HashMap;
use std::fs;
use std::io::{BufRead, BufReader};
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use serde_yaml_ng::{Mapping, Value};

use crate::config::Paths;
use crate::plan::{next_plan_id, write_atomic};
use crate::project::project_root_name;

const MAX_TRANSCRIPT_BYTES: u64 = 50 * 1024 * 1024;
const UNSORTED: &str = "unsorted";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub unsorted: usize,
    pub failed: usize,
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

/// Copy every `claude_home/plans/*.md` into the store. Never modifies sources.
pub fn import_claude_plans(claude_home: &Path, paths: &Paths) -> Result<ImportReport> {
    let mut report = ImportReport::default();
    let src_dir = claude_home.join("plans");
    let Ok(rd) = fs::read_dir(&src_dir) else {
        return Ok(report);
    };
    let mut sources: Vec<PathBuf> = rd
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_file() && p.extension().is_some_and(|e| e == "md"))
        .collect();
    sources.sort();

    let mut known: HashMap<String, Known> = HashMap::new();
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
    let mut ids_by_dir: HashMap<PathBuf, Vec<String>> = HashMap::new();
    let new_names: Vec<String> = sources
        .iter()
        .filter(|src| !known.contains_key(&source_key(src)))
        .filter_map(|src| Some(src.file_name()?.to_string_lossy().into_owned()))
        .collect();
    let inferred = infer_projects(&new_names, claude_home);

    for src in sources {
        let res = (|| -> Result<()> {
            let bytes = fs::read(&src).with_context(|| format!("reading {}", src.display()))?;
            let hash = blake3::hash(&bytes).to_hex().to_string();
            let from = source_key(&src);
            let text = String::from_utf8_lossy(&bytes).into_owned();
            let (fm, body, invalid) = split_frontmatter_full(&text);
            let mut fm = fm;

            let (dest, project, id, is_unsorted) = match known.get(&from) {
                Some(k) if k.hash == hash => {
                    report.skipped += 1;
                    return Ok(());
                }
                Some(k) => {
                    let old = frontmatter_of(&k.dest).unwrap_or_default();
                    let project = str_field(&old, "project").unwrap_or_else(|| UNSORTED.into());
                    let id = str_field(&old, "id");
                    let uns = k.dest.parent() == Some(paths.unsorted.as_path());
                    (k.dest.clone(), project, id, uns)
                }
                None => {
                    let name = src.file_name().unwrap().to_string_lossy().to_string();
                    let (dir, project, uns) = match inferred.get(&name) {
                        Some(p) => (paths.plans.join(p), p.clone(), false),
                        None => (paths.unsorted.clone(), UNSORTED.to_string(), true),
                    };
                    fs::create_dir_all(&dir)?;
                    let dest = free_name(&dir, &name);
                    (dest, project, None, uns)
                }
            };

            let id = match id {
                Some(id) => id,
                None => {
                    let dir = dest.parent().unwrap().to_path_buf();
                    let ids = ids_by_dir
                        .entry(dir.clone())
                        .or_insert_with(|| project_ids(&dir));
                    let id = next_plan_id(&project, ids);
                    ids.push(id.clone());
                    id
                }
            };

            let mut map = fm.take().unwrap_or_else(|| {
                let title = first_heading(body).unwrap_or_else(|| {
                    src.file_stem().unwrap_or_default().to_string_lossy().into()
                });
                let mut m = Mapping::new();
                m.insert("title".into(), title.into());
                m
            });
            map.insert("id".into(), id.into());
            map.insert("project".into(), project.into());
            map.insert("imported_from".into(), from.clone().into());
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
            known.insert(from, Known { hash, dest });
            report.imported += 1;
            if is_unsorted {
                report.unsorted += 1;
            }
            Ok(())
        })();
        if let Err(e) = res {
            eprintln!("plan-atlas: import failed for {}: {e:#}", src.display());
            report.failed += 1;
        }
    }
    Ok(report)
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
