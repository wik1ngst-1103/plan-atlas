//! Plan write-back: line-based status edits, atomic writes, new plan naming.

use std::fmt;
use std::fs;
use std::io::{self, Write};
use std::path::Path;
use std::sync::atomic::{AtomicU64, Ordering};

use super::model::TaskStatus;
use super::slug::slugify;

const MAX_SLUG_LEN: usize = 60;

#[derive(Debug)]
pub enum WriteError {
    TaskNotFound(String),
}

impl fmt::Display for WriteError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WriteError::TaskNotFound(id) => write!(f, "task not found: {id}"),
        }
    }
}

impl std::error::Error for WriteError {}

fn status_str(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Review => "review",
        TaskStatus::Done => "done",
    }
}

/// Line content without its trailing `\n` / `\r\n`.
fn content(line: &[u8]) -> &[u8] {
    let l = line.strip_suffix(b"\n").unwrap_or(line);
    l.strip_suffix(b"\r").unwrap_or(l)
}

fn is_fence(line: &[u8]) -> bool {
    let t = content(line).trim_ascii_start();
    t.starts_with(b"```") || t.starts_with(b"~~~")
}

fn is_meta_line(line: &[u8]) -> bool {
    let c = content(line);
    let Some(rest) = c.strip_prefix(b"- ") else {
        return false;
    };
    rest.iter().position(|&b| b == b':').is_some_and(|i| {
        i > 0
            && rest[..i]
                .iter()
                .all(|b| b.is_ascii_alphanumeric() || *b == b'_' || *b == b'-')
    })
}

fn is_status_line(line: &[u8]) -> bool {
    is_meta_line(line) && content(line).starts_with(b"- status:")
}

/// Set `- status: <value>` for `task_id`, editing only the necessary line.
pub fn set_task_status(
    bytes: &[u8],
    task_id: &str,
    status: TaskStatus,
) -> Result<Vec<u8>, WriteError> {
    let eol: &[u8] = if bytes.windows(2).any(|w| w == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    };
    let prefix = format!("### {task_id}:");
    let lines: Vec<&[u8]> = bytes.split_inclusive(|&b| b == b'\n').collect();

    let mut in_fence = false;
    let mut heading = None;
    for (i, line) in lines.iter().enumerate() {
        if is_fence(line) {
            in_fence = !in_fence;
        } else if !in_fence && line.starts_with(prefix.as_bytes()) {
            heading = Some(i);
            break;
        }
    }
    let h = heading.ok_or_else(|| WriteError::TaskNotFound(task_id.to_string()))?;

    let mut end = h + 1;
    let mut status_idx = None;
    while end < lines.len() && is_meta_line(lines[end]) {
        if status_idx.is_none() && is_status_line(lines[end]) {
            status_idx = Some(end);
        }
        end += 1;
    }

    let new_text = format!("- status: {}", status_str(status));
    let mut out = Vec::with_capacity(bytes.len() + new_text.len() + 2);
    for (i, line) in lines.iter().enumerate() {
        if Some(i) == status_idx {
            out.extend_from_slice(new_text.as_bytes());
            out.extend_from_slice(&line[content(line).len()..]);
        } else {
            out.extend_from_slice(line);
            if i == h && status_idx.is_none() {
                if !line.ends_with(b"\n") {
                    out.extend_from_slice(eol);
                }
                out.extend_from_slice(new_text.as_bytes());
                // Keep the file's original trailing-newline state when the
                // heading was the last line.
                if i + 1 < lines.len() {
                    out.extend_from_slice(eol);
                }
            }
        }
    }
    Ok(out)
}

static TMP_COUNTER: AtomicU64 = AtomicU64::new(0);

/// Write via a unique temp file in the same directory, fsync, then rename over
/// `path`. Symlinks are followed (the link stays a link) and the existing
/// file's permissions are kept.
pub fn write_atomic(path: &Path, bytes: &[u8]) -> io::Result<()> {
    let real = match fs::symlink_metadata(path) {
        Ok(m) if m.file_type().is_symlink() => fs::canonicalize(path)?,
        _ => path.to_path_buf(),
    };
    let dir = real.parent().filter(|p| !p.as_os_str().is_empty());
    let name = real
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "path has no file name"))?
        .to_string_lossy();
    let tmp_name = format!(
        ".{name}.tmp-{}-{}",
        std::process::id(),
        TMP_COUNTER.fetch_add(1, Ordering::Relaxed)
    );
    let tmp = match dir {
        Some(d) => d.join(&tmp_name),
        None => Path::new(&tmp_name).to_path_buf(),
    };
    let result = (|| {
        let mut f = fs::OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&tmp)?;
        f.write_all(bytes)?;
        if let Ok(meta) = fs::metadata(&real) {
            fs::set_permissions(&tmp, meta.permissions())?;
        }
        f.sync_all()?;
        drop(f);
        fs::rename(&tmp, &real)
    })();
    if result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    result
}

/// Next `PLAN-<project>-NNN`, considering only ids of that project.
pub fn next_plan_id(project: &str, existing: &[String]) -> String {
    let prefix = format!("PLAN-{project}-");
    let max = existing
        .iter()
        .filter_map(|id| id.strip_prefix(&prefix))
        .filter(|d| !d.is_empty() && d.bytes().all(|b| b.is_ascii_digit()))
        .filter_map(|d| d.parse::<u64>().ok())
        .max()
        .unwrap_or(0);
    format!("{prefix}{:03}", max + 1)
}

/// `<created>-<slug>.md` with the slug limited to 60 chars at a `-` boundary.
pub fn plan_file_name(created: &str, title: &str) -> String {
    let mut slug = slugify(title);
    if slug.len() > MAX_SLUG_LEN {
        // Slug is ASCII, so byte slicing is safe.
        slug = match slug[..=MAX_SLUG_LEN].rfind('-') {
            Some(i) if i > 0 => slug[..i].to_string(),
            _ => slug[..MAX_SLUG_LEN].to_string(),
        };
    }
    let slug = slug.trim_end_matches('-');
    let slug = if slug.is_empty() { "plan" } else { slug };
    format!("{created}-{slug}.md")
}
