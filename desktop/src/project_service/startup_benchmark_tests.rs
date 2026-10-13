//! Run only against an isolated SQLite backup with copied reference-media.
use super::*;
use std::{path::PathBuf, time::Instant};

#[test]
#[ignore = "requires MSTUDIO_STARTUP_BENCH_ROOT pointing to an isolated temporary copy"]
fn profile_startup_copy() {
    let root = PathBuf::from(std::env::var("MSTUDIO_STARTUP_BENCH_ROOT").unwrap());
    assert!(root.starts_with(std::env::temp_dir()) || root.starts_with("/private/tmp"));
    assert_eq!(
        std::fs::read_to_string(root.join(".mstudio-startup-benchmark")).unwrap(),
        "isolated-copy"
    );
    let db = Connection::open(root.join("mstudio.sqlite3")).unwrap();
    assert_eq!(crate::storage::load(&db, &root).unwrap().directory, root);
    drop(db);
    let start = Instant::now();
    let store = Store::open(root).unwrap();
    let open_ms = start.elapsed().as_millis();
    println!("STARTUP_PROFILE_OPEN open_ms={open_ms}");
    let start = Instant::now();
    let projects = store.projects().unwrap().len();
    let list_ms = start.elapsed().as_millis();
    let start = Instant::now();
    let summaries = store.project_summaries().unwrap();
    let summary_ms = start.elapsed().as_millis();
    println!(
        "STARTUP_PROFILE_LIBRARY summary_ms={summary_ms} summary_bytes={} full_bytes={}",
        serde_json::to_vec(&summaries).unwrap().len(),
        serde_json::to_vec(&store.projects().unwrap())
            .unwrap()
            .len()
    );
    let start = Instant::now();
    production::recover(&store).unwrap();
    println!(
        "STARTUP_PROFILE open_ms={open_ms} list_ms={list_ms} recover_ms={} projects={projects}",
        start.elapsed().as_millis()
    );
}
