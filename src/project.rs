//! Project naming: the directory name of the git repository a path is in.

use std::fs;
use std::path::{Path, PathBuf};

use crate::plan::slug::slugify;

/// Slug of the repository directory containing `cwd` (or of `cwd` itself
/// outside a repository); `None` when that slug is empty.
///
/// Git worktrees (and anything else with a `.git` *file*) resolve to the main
/// repository: `.git` holds `gitdir: <path>`, whose `commondir` points to the
/// main repository's `.git` directory.
pub fn project_root_name(cwd: &Path) -> Option<String> {
    let root = match cwd.ancestors().find(|d| d.join(".git").exists()) {
        Some(dir) => main_repo_dir(dir).unwrap_or_else(|| dir.to_path_buf()),
        None => cwd.to_path_buf(),
    };
    let base = root.file_name()?.to_string_lossy();
    let base = base.strip_suffix(".git").unwrap_or(&base);
    Some(slugify(base)).filter(|s| !s.is_empty())
}

/// Like [`project_root_name`], but `None` when `path` is not inside a git
/// repository (instead of falling back to the directory's own name).
pub fn repo_project_name(path: &Path) -> Option<String> {
    path.ancestors()
        .any(|d| d.join(".git").exists())
        .then(|| project_root_name(path))
        .flatten()
}

/// A user-given project name as a directory-safe slug (`[a-z0-9-]`), or
/// `unsorted` when nothing is left.
pub fn sanitize_project(name: &str) -> String {
    let s = slugify(name);
    if s.is_empty() {
        "unsorted".to_string()
    } else {
        s
    }
}

/// For a checkout whose `.git` is a file, the main repository's directory.
fn main_repo_dir(dir: &Path) -> Option<PathBuf> {
    let dot_git = dir.join(".git");
    if !dot_git.is_file() {
        return None;
    }
    let text = fs::read_to_string(&dot_git).ok()?;
    let gitdir = text.lines().find_map(|l| l.strip_prefix("gitdir:"))?.trim();
    let gitdir = dir.join(gitdir);
    let common = fs::read_to_string(gitdir.join("commondir")).ok()?;
    let common = fs::canonicalize(gitdir.join(common.trim())).ok()?;
    if common.file_name().is_some_and(|n| n == ".git") {
        common.parent().map(Path::to_path_buf)
    } else {
        // Bare repository, e.g. `repo.git`.
        Some(common)
    }
}
