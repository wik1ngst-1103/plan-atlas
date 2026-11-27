//! Shared operations: concurrent task status writes must not lose updates.

use std::path::Path;
use std::sync::{Arc, Barrier};

use plan_atlas::config::Paths;
use plan_atlas::index::Indexer;
use plan_atlas::ops::set_task_status_op;
use plan_atlas::plan::{TaskStatus, parse_plan};
use plan_atlas::store::Store;
use plan_atlas::taxonomy::Taxonomy;

#[test]
fn concurrent_status_changes_to_two_tasks_are_both_persisted() {
    let dir = tempfile::tempdir().unwrap();
    let paths = Paths::from_home(dir.path().to_path_buf());
    paths.ensure().unwrap();
    let proj = paths.plans.join("auth-gateway");
    std::fs::create_dir_all(&proj).unwrap();
    let file = proj.join("2026-11-20-jwt.md");
    let fixture = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/plans/structured.md");
    let tax = Arc::new(Taxonomy::embedded_default());
    let store = Arc::new(Store::open(&paths.db, tax.clone()).unwrap());

    for round in 0..5 {
        std::fs::copy(&fixture, &file).unwrap();
        Indexer::new(paths.clone(), store.clone(), tax.clone())
            .full_scan()
            .unwrap();
        let threads = 16;
        let barrier = Arc::new(Barrier::new(threads));
        let handles: Vec<_> = (0..threads)
            .map(|i| {
                let (store, tax, paths, barrier) =
                    (store.clone(), tax.clone(), paths.clone(), barrier.clone());
                std::thread::spawn(move || {
                    let (task, status) = if i % 2 == 0 {
                        ("TASK-001", TaskStatus::Done)
                    } else {
                        ("TASK-003", TaskStatus::Review)
                    };
                    barrier.wait();
                    set_task_status_op(&store, &tax, &paths, "PLAN-auth-gateway-001", task, status)
                        .unwrap();
                })
            })
            .collect();
        for h in handles {
            h.join().unwrap();
        }
        let plan = parse_plan(&std::fs::read(&file).unwrap(), &file);
        let status = |id: &str| plan.tasks.iter().find(|t| t.id == id).unwrap().status;
        assert_eq!(status("TASK-001"), TaskStatus::Done, "round {round}");
        assert_eq!(status("TASK-003"), TaskStatus::Review, "round {round}");
    }
}
