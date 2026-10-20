use super::{scan, tests::Fixture};
#[test]
fn saved_analysis_media_is_kept_even_without_a_project_reference() {
    let fixture = Fixture::new();
    let folder = fixture
        .root
        .join(crate::video_analysis::history_store::FOLDER)
        .join(uuid::Uuid::new_v4().to_string());
    std::fs::create_dir_all(&folder).unwrap();
    let path = folder.join("source.mp4");
    std::fs::write(&path, b"durable original").unwrap();
    std::fs::File::open(&path)
        .unwrap()
        .set_modified(std::time::SystemTime::now() - std::time::Duration::from_secs(3600))
        .unwrap();
    let db = fixture.store.conn().unwrap();
    let plan = scan::build(&db, &fixture.root, &fixture.data).unwrap();
    assert!(!plan.files.contains(&path));
    assert_eq!(plan.report.categories["media"].files, 1);
}
