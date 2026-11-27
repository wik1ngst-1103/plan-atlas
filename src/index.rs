//! Keeps the SQLite index in sync with the Markdown plan files: a full scan
//! at startup plus a debounced file watcher for live updates.

use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use notify_debouncer_full::notify::{EventKind, RecursiveMode};
use notify_debouncer_full::{DebounceEventResult, Debouncer, RecommendedCache, new_debouncer};
use tokio::sync::broadcast;

use crate::config::Paths;
use crate::events::LiveEvent;
use crate::plan::{Plan, parse_plan, validate};
use crate::store::Store;
use crate::taxonomy::Taxonomy;

const BATCH: usize = 200;
const DEBOUNCE: Duration = Duration::from_millis(300);

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ScanReport {
    pub indexed: usize,
    pub removed: usize,
    /// One entry per warning, formatted `path: code: message`.
    pub warnings: Vec<String>,
}

/// Keeps the debouncer (and thus the watch) alive; dropping it stops watching.
pub struct WatchHandle {
    _debouncer: Debouncer<notify_debouncer_full::notify::RecommendedWatcher, RecommendedCache>,
}

pub struct Indexer {
    paths: Paths,
    store: Arc<Store>,
    tax: Arc<Taxonomy>,
}

/// Only visible `*.md` files under `root` count as plans.
fn is_plan_file(root: &Path, path: &Path) -> bool {
    if path.extension().and_then(|e| e.to_str()) != Some("md") {
        return false;
    }
    let rel = path.strip_prefix(root).unwrap_or(path);
    !rel.components()
        .any(|c| c.as_os_str().to_string_lossy().starts_with('.'))
}

/// The plans root as configured and canonicalized: watchers may report
/// either form (macOS FSEvents reports `/private/var/...`, symlinked homes
/// resolve to their target).
fn watch_roots(plans: &Path) -> Vec<PathBuf> {
    let mut roots = vec![plans.to_path_buf()];
    if let Ok(c) = std::fs::canonicalize(plans)
        && c != plans
    {
        roots.push(c);
    }
    roots
}

/// [`is_plan_file`] relative to whichever of `roots` contains `path`.
fn is_watched_plan_file(roots: &[PathBuf], path: &Path) -> bool {
    match roots.iter().find(|r| path.starts_with(r)) {
        Some(root) => is_plan_file(root, path),
        None => false,
    }
}

fn collect_md(root: &Path, dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(rd) = std::fs::read_dir(dir) else {
        return;
    };
    for entry in rd.flatten() {
        let path = entry.path();
        if entry.file_name().to_string_lossy().starts_with('.') {
            continue;
        }
        let Ok(ft) = entry.file_type() else { continue };
        if ft.is_dir() {
            collect_md(root, &path, out);
        } else if is_plan_file(root, &path) {
            out.push(path);
        }
    }
}

impl Indexer {
    pub fn new(paths: Paths, store: Arc<Store>, tax: Arc<Taxonomy>) -> Self {
        Indexer { paths, store, tax }
    }

    pub fn full_scan(&self) -> Result<ScanReport> {
        let mut files = Vec::new();
        collect_md(&self.paths.plans, &self.paths.plans, &mut files);

        let mut report = ScanReport::default();
        let mut batch = Vec::with_capacity(BATCH);
        for file in &files {
            match load_plan(&self.tax, file) {
                Ok(plan) => {
                    report.warnings.extend(
                        plan.warnings
                            .iter()
                            .map(|w| format!("{}: {}: {}", file.display(), w.code, w.message)),
                    );
                    batch.push(plan);
                    if batch.len() >= BATCH {
                        self.flush(&mut batch, &mut report)?;
                    }
                }
                Err(e) => eprintln!("plan-atlas: skipping {}: {e:#}", file.display()),
            }
        }
        self.flush(&mut batch, &mut report)?;

        let seen: HashSet<&PathBuf> = files.iter().collect();
        for path in self.store.all_paths()? {
            if !seen.contains(&path) {
                self.store.delete_by_path(&path)?;
                report.removed += 1;
            }
        }
        Ok(report)
    }

    fn flush(&self, batch: &mut Vec<Plan>, report: &mut ScanReport) -> Result<()> {
        if batch.is_empty() {
            return Ok(());
        }
        self.store.upsert_plans(batch)?;
        report.indexed += batch.len();
        batch.clear();
        Ok(())
    }

    pub fn watch(self, tx: broadcast::Sender<LiveEvent>) -> Result<WatchHandle> {
        let Indexer { paths, store, tax } = self;
        let roots = watch_roots(&paths.plans);
        // Tick once per debounce period (the default ticks 4x as often).
        let mut debouncer =
            new_debouncer(DEBOUNCE, Some(DEBOUNCE), move |res: DebounceEventResult| {
                let events = match res {
                    Ok(events) => events,
                    Err(errs) => {
                        for e in errs {
                            eprintln!("plan-atlas: watch error: {e}");
                        }
                        return;
                    }
                };
                let mut seen = HashSet::new();
                for ev in events {
                    if matches!(ev.kind, EventKind::Access(_)) {
                        continue;
                    }
                    for path in &ev.paths {
                        if is_watched_plan_file(&roots, path) && seen.insert(path.clone()) {
                            handle_path(&store, &tax, &tx, path);
                        }
                    }
                }
            })
            .context("creating file watcher")?;
        debouncer
            .watch(&paths.plans, RecursiveMode::Recursive)
            .with_context(|| format!("watching {}", paths.plans.display()))?;
        Ok(WatchHandle {
            _debouncer: debouncer,
        })
    }
}

fn load_plan(tax: &Taxonomy, path: &Path) -> Result<Plan> {
    let bytes = std::fs::read(path).with_context(|| format!("reading {}", path.display()))?;
    let mut plan = parse_plan(&bytes, path);
    let extra = validate(&plan, tax);
    plan.warnings.extend(extra);
    Ok(plan)
}

/// Read, parse, validate and upsert one plan file. Returns the plan id, or
/// `None` when `path` is not an existing file (nothing to index).
pub fn index_file(store: &Store, tax: &Taxonomy, path: &Path) -> Result<Option<String>> {
    if !path.is_file() {
        return Ok(None);
    }
    let plan = load_plan(tax, path)?;
    store.upsert_plan(&plan)?;
    Ok(Some(plan.id))
}

fn handle_path(store: &Store, tax: &Taxonomy, tx: &broadcast::Sender<LiveEvent>, path: &Path) {
    let emit = |plan_id: String| {
        let _ = tx.send(LiveEvent::PlanChanged { plan_id });
    };
    if path.is_file() {
        match index_file(store, tax, path) {
            Ok(Some(id)) => emit(id),
            Ok(None) => {}
            Err(e) => eprintln!("plan-atlas: indexing {}: {e:#}", path.display()),
        }
    } else {
        match store.plan_id_at_path(path) {
            Ok(Some(id)) => match store.delete_by_path(path) {
                Ok(()) => emit(id),
                Err(e) => eprintln!("plan-atlas: removing {}: {e:#}", path.display()),
            },
            Ok(None) => {}
            Err(e) => eprintln!("plan-atlas: lookup {}: {e:#}", path.display()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[cfg(unix)]
    #[test]
    fn watched_paths_match_canonical_root() {
        let t = tempfile::tempdir().unwrap();
        let real = t.path().join("real/.plan-atlas/plans");
        std::fs::create_dir_all(&real).unwrap();
        let link = t.path().join("home");
        std::os::unix::fs::symlink(t.path().join("real"), &link).unwrap();
        let roots = watch_roots(&link.join(".plan-atlas/plans"));
        let canon = std::fs::canonicalize(&real).unwrap();
        // Reported through the canonical path (like macOS /private/var).
        assert!(is_watched_plan_file(&roots, &canon.join("proj/a.md")));
        assert!(is_watched_plan_file(
            &roots,
            &link.join(".plan-atlas/plans/proj/a.md")
        ));
        assert!(!is_watched_plan_file(
            &roots,
            &canon.join("proj/.hidden.md")
        ));
        assert!(!is_watched_plan_file(&roots, &canon.join("proj/a.txt")));
        assert!(!is_watched_plan_file(&roots, Path::new("/elsewhere/a.md")));
    }
}
