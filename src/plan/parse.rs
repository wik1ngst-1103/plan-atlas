use std::path::Path;

use pulldown_cmark::{Event, HeadingLevel, Options, Parser, Tag, TagEnd};
use serde_yaml_ng::Value;

use super::model::{
    Effort, Model, Plan, Relation, RelationKind, Section, Task, TaskStatus, Warning,
};
use super::slug::slugify;

struct Frontmatter {
    id: Option<String>,
    project: Option<String>,
    title: Option<String>,
    tags: Vec<String>,
    related: Vec<Relation>,
    created: Option<String>,
}

fn scalar_string(v: &Value) -> Option<String> {
    match v {
        Value::String(s) => Some(s.clone()),
        Value::Number(n) => Some(n.to_string()),
        _ => None,
    }
}

fn parse_kind(s: &str) -> Option<RelationKind> {
    match s {
        "depends_on" => Some(RelationKind::DependsOn),
        "extends" => Some(RelationKind::Extends),
        "relates_to" => Some(RelationKind::RelatesTo),
        _ => None,
    }
}

/// Lenient extraction: every bad field yields an `invalid_frontmatter` warning
/// naming it; good fields are kept. Err only for unparseable YAML / non-mapping.
fn parse_frontmatter(y: &str, warnings: &mut Vec<Warning>) -> Result<Frontmatter, String> {
    let value: Value = serde_yaml_ng::from_str(y).map_err(|e| e.to_string())?;
    let Value::Mapping(map) = value else {
        return Err("frontmatter is not a mapping".to_string());
    };
    let mut bad = |field: &str, why: &str| {
        warnings.push(Warning::new(
            "invalid_frontmatter",
            format!("frontmatter field '{field}' ignored: {why}"),
        ));
    };
    let mut fm = Frontmatter {
        id: None,
        project: None,
        title: None,
        tags: Vec::new(),
        related: Vec::new(),
        created: None,
    };
    for (k, v) in &map {
        let Some(key) = k.as_str() else { continue };
        match key {
            "id" | "project" | "title" | "created" => match scalar_string(v) {
                Some(s) => match key {
                    "id" => fm.id = Some(s),
                    "project" => fm.project = Some(s),
                    "title" => fm.title = Some(s),
                    _ => fm.created = Some(s),
                },
                None if v.is_null() => {}
                None => bad(key, "expected a string"),
            },
            "tags" => match v {
                Value::Null => {}
                Value::Sequence(items) => {
                    for it in items {
                        match scalar_string(it) {
                            Some(s) => fm.tags.push(s),
                            None => bad("tags", "item is not a string"),
                        }
                    }
                }
                other => match scalar_string(other) {
                    Some(s) => fm.tags.push(s),
                    None => bad("tags", "expected a list of strings"),
                },
            },
            "related" => match v {
                Value::Null => {}
                Value::Sequence(items) => {
                    for it in items {
                        let to = it.get("to").and_then(scalar_string);
                        let kind = it.get("type").and_then(Value::as_str).and_then(parse_kind);
                        match (to, kind) {
                            (Some(to), Some(kind)) => fm.related.push(Relation { to, kind }),
                            _ => bad("related", "item needs 'to' and a valid 'type'"),
                        }
                    }
                }
                _ => bad("related", "expected a list"),
            },
            _ => {}
        }
    }
    Ok(fm)
}

struct Heading {
    level: u8,
    title: String,
    /// Byte range of the heading itself within the body text.
    start: usize,
    end: usize,
}

/// Parse a plan file. Never panics; problems become warnings.
pub fn parse_plan(bytes: &[u8], path: &Path) -> Plan {
    let mut warnings = Vec::new();
    let content_hash = blake3::hash(bytes).to_hex().to_string();

    let text = match std::str::from_utf8(bytes) {
        Ok(s) => s.to_string(),
        Err(_) => {
            warnings.push(Warning::new(
                "non_utf8",
                "file is not valid UTF-8; invalid bytes were replaced",
            ));
            String::from_utf8_lossy(bytes).into_owned()
        }
    };
    let text = text.trim_start_matches('\u{feff}').replace("\r\n", "\n");

    let stem = path
        .file_stem()
        .map(|s| s.to_string_lossy().into_owned())
        .unwrap_or_default();

    let (fm_text, body) = split_frontmatter(&text);
    let fm = match fm_text {
        None => {
            warnings.push(Warning::new(
                "missing_frontmatter",
                "no YAML frontmatter found",
            ));
            None
        }
        Some(y) => match parse_frontmatter(y, &mut warnings) {
            Ok(f) => Some(f),
            Err(e) => {
                warnings.push(Warning::new(
                    "invalid_frontmatter",
                    format!("frontmatter could not be parsed: {e}"),
                ));
                None
            }
        },
    };

    let headings = scan_headings(body);
    let sections = build_sections(body, &headings);
    let has_tasks_section = sections.iter().any(|s| is_task_section(&s.title));
    let tasks = extract_tasks(body, &headings, &mut warnings);

    let h1 = headings
        .iter()
        .find(|h| h.level == 1)
        .map(|h| h.title.clone())
        .filter(|t| !t.is_empty());

    let structured = fm.is_some() && has_tasks_section;
    let (id, project, title, tags, related, created) = match fm {
        Some(f) => (
            f.id.unwrap_or_else(|| stem.clone()),
            f.project.unwrap_or_else(|| "unsorted".to_string()),
            f.title.or(h1).unwrap_or_else(|| stem.clone()),
            f.tags,
            f.related,
            f.created,
        ),
        None => (
            stem.clone(),
            "unsorted".to_string(),
            h1.unwrap_or_else(|| stem.clone()),
            Vec::new(),
            Vec::new(),
            None,
        ),
    };

    Plan {
        id,
        project,
        title,
        tags,
        related,
        created,
        sections,
        tasks,
        structured,
        source_path: path.to_path_buf(),
        content_hash,
        warnings,
    }
}

fn is_task_section(title: &str) -> bool {
    title.trim_start().starts_with("04.")
}

/// Returns (yaml, rest). Frontmatter is delimited by `---` lines at file start.
fn split_frontmatter(text: &str) -> (Option<&str>, &str) {
    let Some(rest) = text.strip_prefix("---\n") else {
        return (None, text);
    };
    let mut pos = 0;
    for line in rest.split_inclusive('\n') {
        if line.trim_end() == "---" {
            return (Some(&rest[..pos]), &rest[pos + line.len()..]);
        }
        pos += line.len();
    }
    (None, text)
}

fn scan_headings(body: &str) -> Vec<Heading> {
    let mut out = Vec::new();
    let mut current: Option<Heading> = None;
    for (ev, range) in Parser::new_ext(body, Options::empty()).into_offset_iter() {
        match ev {
            Event::Start(Tag::Heading { level, .. }) => {
                current = Some(Heading {
                    level: level_num(level),
                    title: String::new(),
                    start: range.start,
                    end: range.end,
                });
            }
            Event::Text(t) | Event::Code(t) => {
                if let Some(h) = current.as_mut() {
                    h.title.push_str(&t);
                }
            }
            Event::End(TagEnd::Heading(_)) => {
                if let Some(mut h) = current.take() {
                    h.title = h.title.trim().to_string();
                    h.end = range.end;
                    out.push(h);
                }
            }
            _ => {}
        }
    }
    out
}

fn level_num(l: HeadingLevel) -> u8 {
    match l {
        HeadingLevel::H1 => 1,
        HeadingLevel::H2 => 2,
        HeadingLevel::H3 => 3,
        HeadingLevel::H4 => 4,
        HeadingLevel::H5 => 5,
        HeadingLevel::H6 => 6,
    }
}

/// Text under heading `i`, up to the next heading of level 1..=3.
fn body_of<'a>(body: &'a str, headings: &[Heading], i: usize) -> &'a str {
    let from = headings[i].end;
    let to = headings[i + 1..]
        .iter()
        .find(|h| h.level <= 3)
        .map_or(body.len(), |h| h.start);
    body[from..to].trim()
}

fn build_sections(body: &str, headings: &[Heading]) -> Vec<Section> {
    let mut roots: Vec<Section> = Vec::new();
    for (i, h) in headings.iter().enumerate() {
        if h.level != 2 && h.level != 3 {
            continue;
        }
        let section = Section {
            level: h.level,
            title: h.title.clone(),
            anchor: slugify(&h.title),
            body_md: body_of(body, headings, i).to_string(),
            children: Vec::new(),
        };
        match roots.last_mut() {
            Some(parent) if h.level == 3 && parent.level == 2 => parent.children.push(section),
            _ => roots.push(section),
        }
    }
    roots
}

fn extract_tasks(body: &str, headings: &[Heading], warnings: &mut Vec<Warning>) -> Vec<Task> {
    let mut tasks = Vec::new();
    let mut in_tasks = false;
    for (i, h) in headings.iter().enumerate() {
        match h.level {
            1 | 2 => in_tasks = h.level == 2 && is_task_section(&h.title),
            3 if in_tasks => {
                if let Some((id, name)) = parse_task_heading(&h.title) {
                    tasks.push(build_task(id, name, body_of(body, headings, i), warnings));
                }
            }
            _ => {}
        }
    }
    tasks
}

/// `TASK-001: Name` -> ("TASK-001", "Name")
fn parse_task_heading(title: &str) -> Option<(&str, &str)> {
    let digits = title.strip_prefix("TASK-")?.get(..3)?;
    if !digits.bytes().all(|b| b.is_ascii_digit()) {
        return None;
    }
    let name = title[8..].strip_prefix(':')?;
    Some((&title[..8], name.trim()))
}

fn build_task(id: &str, name: &str, text: &str, warnings: &mut Vec<Warning>) -> Task {
    let mut task = Task {
        id: id.to_string(),
        name: name.to_string(),
        complexity: None,
        model: None,
        effort: None,
        depends_on: Vec::new(),
        status: TaskStatus::Todo,
        tags: Vec::new(),
        body_md: String::new(),
    };
    let mut bad = |key: &str, value: &str| {
        warnings.push(Warning::new(
            "invalid_task_metadata",
            format!("{id}: invalid {key} value '{value}'"),
        ));
    };

    let mut lines = text.lines().peekable();
    while lines.next_if(|l| l.trim().is_empty()).is_some() {}
    while let Some(line) = lines.peek() {
        let Some((key, value)) = parse_meta_line(line) else {
            break;
        };
        lines.next();
        match key {
            "complexity" => match value.parse::<u8>() {
                Ok(n) if (1..=5).contains(&n) => task.complexity = Some(n),
                _ => bad(key, value),
            },
            "model" => match value.to_ascii_lowercase().as_str() {
                "haiku" => task.model = Some(Model::Haiku),
                "sonnet" => task.model = Some(Model::Sonnet),
                "opus" => task.model = Some(Model::Opus),
                _ => bad(key, value),
            },
            "effort" => match value.to_ascii_lowercase().as_str() {
                "low" => task.effort = Some(Effort::Low),
                "medium" => task.effort = Some(Effort::Medium),
                "high" => task.effort = Some(Effort::High),
                _ => bad(key, value),
            },
            "status" => match value.to_ascii_lowercase().as_str() {
                "todo" => task.status = TaskStatus::Todo,
                "in_progress" => task.status = TaskStatus::InProgress,
                "review" => task.status = TaskStatus::Review,
                "done" => task.status = TaskStatus::Done,
                _ => bad(key, value),
            },
            "depends_on" => task.depends_on = parse_list(value),
            "tags" => task.tags = parse_list(value),
            _ => {}
        }
    }
    task.body_md = lines.collect::<Vec<_>>().join("\n").trim().to_string();
    task
}

/// `- key: value` with a lowercase snake_case key.
fn parse_meta_line(line: &str) -> Option<(&str, &str)> {
    let rest = line.trim().strip_prefix("- ")?;
    let (key, value) = rest.split_once(':')?;
    let key = key.trim();
    if key.is_empty() || !key.bytes().all(|b| b.is_ascii_lowercase() || b == b'_') {
        return None;
    }
    Some((key, value.trim()))
}

/// `[a, b]` (or bare `a, b`) -> ["a", "b"]
fn parse_list(value: &str) -> Vec<String> {
    let inner = value
        .strip_prefix('[')
        .and_then(|v| v.strip_suffix(']'))
        .unwrap_or(value);
    inner
        .split(',')
        .map(|s| s.trim().trim_matches(|c| c == '"' || c == '\'').trim())
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect()
}
