use std::sync::Arc;
use std::time::Duration;

use plan_atlas::config::Paths;
use plan_atlas::events::LiveEvent;
use plan_atlas::index::Indexer;
use plan_atlas::plan::write_atomic;
use plan_atlas::store::{PlanFilter, Store};
use plan_atlas::taxonomy::Taxonomy;
use tempfile::TempDir;
use tokio::sync::broadcast;

fn plan_md(id: &str) -> String {
    format!(
        "---\nid: {id}\nproject: demo\ntitle: Plan {id}\ntags: [backend]\ncreated: 2026-11-27\n---\n\n# Plan {id}\n\nbody\n"
    )
}

fn setup() -> (TempDir, Paths, Arc<Store>, Indexer) {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(dir.path().to_path_buf());
    paths.ensure().unwrap();
    let tax = Arc::new(Taxonomy::embedded_default());
    let store = Arc::new(Store::open(&paths.db, tax.clone()).unwrap());
    let idx = Indexer::new(paths.clone(), store.clone(), tax);
    (dir, paths, store, idx)
}

fn count(store: &Store) -> usize {
    store.list_plans(&PlanFilter::default()).unwrap().len()
}

#[test]
fn full_scan_indexes_and_prunes() {
    let (_d, paths, store, idx) = setup();
    let a = paths.plans.join("a.md");
    let b = paths.plans.join("sub").join("b.md");
    std::fs::create_dir_all(b.parent().unwrap()).unwrap();
    std::fs::write(&a, plan_md("demo-1")).unwrap();
    std::fs::write(&b, plan_md("demo-2")).unwrap();
    std::fs::write(paths.plans.join(".hidden.md"), plan_md("demo-3")).unwrap();
    std::fs::write(paths.plans.join("note.txt"), "x").unwrap();

    let r = idx.full_scan().unwrap();
    assert_eq!((r.indexed, r.removed), (2, 0));
    assert_eq!(count(&store), 2);

    std::fs::remove_file(&b).unwrap();
    let r = idx.full_scan().unwrap();
    assert_eq!(r.removed, 1);
    assert_eq!(count(&store), 1);
}

async fn next_event(rx: &mut broadcast::Receiver<LiveEvent>, want: &str) {
    let found = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            if let Ok(LiveEvent::PlanChanged { plan_id }) = rx.recv().await
                && plan_id == want
            {
                return;
            }
        }
    })
    .await;
    assert!(found.is_ok(), "no PlanChanged for {want}");
}

#[tokio::test]
async fn watch_picks_up_new_file() {
    let (_d, paths, store, idx) = setup();
    let (tx, mut rx) = broadcast::channel(16);
    let _h = idx.watch(tx).unwrap();

    let f = paths.plans.join("new.md");
    write_atomic(&f, plan_md("demo-9").as_bytes()).unwrap();
    next_event(&mut rx, "demo-9").await;
    assert_eq!(count(&store), 1);

    std::fs::remove_file(&f).unwrap();
    next_event(&mut rx, "demo-9").await;
    assert_eq!(count(&store), 0);
}
