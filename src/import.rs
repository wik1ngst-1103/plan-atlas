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
use crate::plan::slug::slugify;
use crate::plan::{next_plan_id, write_atomic};

const MAX_TRANSCRIPT_BYTES: u64 = 50 * 1024 * 1024;
const UNSORTED: &str = "unsorted";

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ImportReport {
    pub imported: usize,
    pub skipped: usize,
    pub unsorted: usize,
}

/// Split `---` frontmatter from the body. Returns the parsed mapping and body;
/// text without valid mapping frontmatter yields `(None, whole text)`.
fn split_frontmatter(text: &str) -> (Option<Mapping>, &str) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return (None, text);
    };
    let mut pos = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return match serde_yaml_ng::from_str::<Value>(&rest[..pos]) {
                Ok(Value::Mapping(m)) => (Some(m), &rest[pos + line.len()..]),
                Ok(Value::Null) => (Some(Mapping::new()), &rest[pos + line.len()..]),
                _ => (None, text),
            };
        }
        pos += line.len();
    }
    (None, text)
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

fn git_root_or_self(cwd: &Path) -> PathBuf {
    cwd.ancestors()
        .find(|a| a.join(".git").exists())
        .unwrap_or(cwd)
        .to_path_buf()
}

/// Find which project produced `plan_file_name` by scanning Claude Code
/// transcripts under `claude_home/projects/*/*.jsonl`.
pub fn infer_project(plan_file_name: &str, claude_home: &Path) -> Option<String> {
    let mut dirs: Vec<_> = fs::read_dir(claude_home.join("projects"))
        .ok()?
        .flatten()
        .map(|e| e.path())
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        let Ok(rd) = fs::read_dir(&dir) else { continue };
        let mut files: Vec<_> = rd
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().is_some_and(|e| e == "jsonl"))
            .collect();
        files.sort();
        for file in files {
            if let Some(cwd) = cwd_for_match(&file, plan_file_name) {
                let root = git_root_or_self(Path::new(&cwd));
                let name = slugify(&root.file_name()?.to_string_lossy());
                if !name.is_empty() {
                    return Some(name);
                }
            }
        }
    }
    None
}

/// First line mentioning `needle` decides: its `cwd`, else the nearest earlier one.
fn cwd_for_match(file: &Path, needle: &str) -> Option<String> {
    if fs::metadata(file).ok()?.len() > MAX_TRANSCRIPT_BYTES {
        return None;
    }
    let mut reader = BufReader::new(fs::File::open(file).ok()?);
    let mut last_cwd: Option<String> = None;
    let mut buf = Vec::new();
    loop {
        buf.clear();
        if reader.read_until(b'\n', &mut buf).ok()? == 0 {
            return None;
        }
        let line = String::from_utf8_lossy(&buf);
        let cwd = if line.contains("\"cwd\"") {
            serde_json::from_str::<serde_json::Value>(&line)
                .ok()
                .and_then(|v| v.get("cwd").and_then(|c| c.as_str()).map(str::to_string))
        } else {
            None
        };
        if line.contains(needle) {
            return cwd.or(last_cwd);
        }
        if cwd.is_some() {
            last_cwd = cwd;
        }
    }
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

    for src in sources {
        let bytes = fs::read(&src).with_context(|| format!("reading {}", src.display()))?;
        let hash = blake3::hash(&bytes).to_hex().to_string();
        let abs = fs::canonicalize(&src).unwrap_or_else(|_| src.clone());
        let from = abs.to_string_lossy().to_string();
        let text = String::from_utf8_lossy(&bytes).into_owned();
        let (fm, body) = split_frontmatter(&text);
        let mut fm = fm;

        let (dest, project, id, is_unsorted) = match known.get(&from) {
            Some(k) if k.hash == hash => {
                report.skipped += 1;
                continue;
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
                let inferred = infer_project(&name, claude_home);
                let (dir, project, uns) = match inferred {
                    Some(p) => (paths.plans.join(&p), p, false),
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
            let title = first_heading(body)
                .unwrap_or_else(|| src.file_stem().unwrap_or_default().to_string_lossy().into());
            let mut m = Mapping::new();
            m.insert("title".into(), title.into());
            m
        });
        map.insert("id".into(), id.into());
        map.insert("project".into(), project.into());
        map.insert("imported_from".into(), from.clone().into());
        map.insert("imported_hash".into(), hash.clone().into());
        let out = format!("{}{}", build_yaml(map), body);
        write_atomic(&dest, out.as_bytes())
            .with_context(|| format!("writing {}", dest.display()))?;
        known.insert(from, Known { hash, dest });
        report.imported += 1;
        if is_unsorted {
            report.unsorted += 1;
        }
    }
    Ok(report)
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
