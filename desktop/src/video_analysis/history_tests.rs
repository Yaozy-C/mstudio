use super::{history_store::*, *};
use crate::assistant::standalone_analysis::Analysis;

fn sample(video: &Video) -> Record {
    Record {
        id: String::new(), created_at: 0,
        context: ContextInfo { title: "Original film".into(), work_id: Some("mora-mora".into()), source_url: Some("https://example.com/film".into()), model_name: "Original model".into() },
        video: video.clone(), question: "Focus on sound".into(), model_id: "removed-model".into(), language: "zh-CN".into(),
        result: serde_json::from_value::<Analysis>(serde_json::json!({"summary":"Observed content", "access":"Original", "segments":[{"start":0,"end":1,"title":"Opening","visual":"Action","speech":"","onScreenText":"","sound":"Music","purpose":"Setup"}],"craft":{"idea":"Idea","visuals":"Look","camera":"Shot","language":"Words","sound":"Music"}})).unwrap(),
    }
}
#[test]
fn durable_records_survive_session_release_database_reopen_and_storage_move() {
    let root = std::env::temp_dir().join(format!("analysis-history-{}", uuid::Uuid::new_v4()));
    let cache = root.join("cache");
    std::fs::create_dir_all(&cache).unwrap();
    let source = cache.join("original.mp4");
    let output = mstudio::media::command("ffmpeg")
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=size=32x32:duration=1",
            "-c:v",
            "libx264",
        ])
        .arg(&source)
        .output()
        .unwrap();
    assert!(output.status.success());
    let session = Session {
        directory: cache.clone(),
        video: Video {
            id: uuid::Uuid::new_v4().to_string(),
            name: "original.mp4".into(),
            path: source.to_string_lossy().into(),
            duration: 1.,
            width: 32,
            height: 32,
            has_audio: false,
        },
    };
    let dbpath = root.join("records.sqlite3");
    let db = rusqlite::Connection::open(&dbpath).unwrap();
    init(&db).unwrap();
    let first = stage(&root, &session, sample(&session.video)).unwrap();
    let first_id = insert(&root, &db, &first).unwrap();
    let mut next = sample(&session.video);
    next.question = "Revised focus".into();
    let second = stage(&root, &session, next).unwrap();
    let second_id = insert(&root, &db, &second).unwrap();
    assert_ne!(first_id, second_id);
    drop(session);
    assert!(!cache.exists());
    drop(db);
    let db = rusqlite::Connection::open(&dbpath).unwrap();
    let page = list(&db, &root, "", 0).unwrap();
    assert_eq!(page.total, 2);
    assert_eq!(page.items[0].id, second_id);
    assert!(
        page.items
            .iter()
            .all(|row| !row.missing_video && row.poster.is_some())
    );
    let record = read(&db, &first_id).unwrap();
    assert_eq!(record.question, "Focus on sound");
    assert_eq!(record.context.model_name, "Original model");
    assert_eq!(record.result.segments[0].sound, "Music");
    assert!(read(&db, "../outside").is_err());
    assert_eq!(list(&db, &root, "Revised", 0).unwrap().total, 1);
    assert_eq!(list(&db, &root, "%", 0).unwrap().total, 0);
    let moved = root.join("moved");
    std::fs::create_dir(&moved).unwrap();
    std::fs::rename(root.join(FOLDER), moved.join(FOLDER)).unwrap();
    assert!(media_path(&moved, &record).unwrap().is_file());
    assert!(!list(&db, &moved, "", 0).unwrap().items[0].missing_video);
    std::fs::remove_file(media_path(&moved, &record).unwrap()).unwrap();
    assert!(list(&db, &moved, "Focus on sound", 0).unwrap().items[0].missing_video);
    assert_eq!(
        read(&db, &first_id).unwrap().result.summary,
        "Observed content"
    );
    drop(db);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn history_media_is_intentional_storage_and_paths_cannot_escape_the_record() {
    assert!(crate::storage::FOLDERS.contains(&FOLDER));
    let video = Video {
        id: "test".into(),
        name: "a".into(),
        path: "../../outside.mp4".into(),
        duration: 1.,
        width: 32,
        height: 32,
        has_audio: false,
    };
    let mut record = sample(&video);
    record.id = uuid::Uuid::new_v4().to_string();
    assert!(media_path(std::path::Path::new("/tmp"), &record).is_err());
}

#[test]
fn history_is_paginated_without_hiding_older_results() {
    let db = rusqlite::Connection::open_in_memory().unwrap();
    init(&db).unwrap();
    let root = std::path::Path::new("/unavailable-storage");
    let video = Video {
        id: "x".into(),
        name: "a".into(),
        path: "source.mp4".into(),
        duration: 1.,
        width: 32,
        height: 32,
        has_audio: false,
    };
    for index in 0..45 {
        let mut record = sample(&video);
        record.id = uuid::Uuid::new_v4().to_string();
        record.created_at = index;
        insert(root, &db, &record).unwrap();
    }
    let first = list(&db, root, "", 0).unwrap();
    let second = list(&db, root, "", 40).unwrap();
    assert_eq!(first.total, 45);
    assert_eq!(first.items.len(), 40);
    assert_eq!(second.items.len(), 5);
    assert_eq!(first.items[0].created_at, 44);
    assert_eq!(second.items[0].created_at, 4);
    assert!(
        second
            .items
            .iter()
            .all(|item| first.items.iter().all(|other| item.id != other.id))
    );
}
