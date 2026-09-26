use super::*;
use crate::projects::write_document;
use serde_json::json;
use std::process::Command;

struct Fixture(Store);
impl Fixture {
    fn new() -> Self {
        let store =
            Store::open(std::env::temp_dir().join(format!("mstudio-import-{}", media::id())))
                .unwrap();
        for id in ["one", "two"] {
            write_document(
                &store,
                json!({"id":id,"name":id,"assets":[],"nodes":[]}),
                true,
            )
            .unwrap();
        }
        Self(store)
    }
    fn file(&self, name: &str, bytes: &[u8]) -> PathBuf {
        let path = self.0.root.join(name);
        std::fs::write(&path, bytes).unwrap();
        path
    }
    fn clean_work(&self) {
        assert_eq!(
            std::fs::read_dir(self.0.root.join("imports"))
                .map(|r| r.count())
                .unwrap_or(0),
            0
        );
        let count: i64 = self
            .0
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM project_files", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);
    }
}
impl Drop for Fixture {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0.root);
    }
}

#[test]
fn chunked_file_keeps_name_and_is_owned_by_project() {
    let f = Fixture::new();
    let id = transfer::begin(&f.0, "one", "参考说明.md".into(), 6).unwrap();
    transfer::append(&f.0, "one", &id, 0, b"abc").unwrap();
    transfer::append(&f.0, "one", &id, 3, b"def").unwrap();
    let asset = transfer::finish(&f.0, "one", &id).unwrap();
    assert_eq!(asset.name, "参考说明.md");
    assert_eq!(asset.kind, "text");
    assert_eq!(std::fs::read(&asset.path).unwrap(), b"abcdef");
    assert_eq!(f.0.assets().unwrap().len(), 1);
    f.clean_work();
    project_storage::remove(&f.0, "one").unwrap();
    assert!(!Path::new(&asset.path).exists());
    assert!(f.0.assets().unwrap().is_empty());
}

#[test]
fn invalid_and_out_of_order_transfers_are_rejected_and_can_be_cancelled() {
    let f = Fixture::new();
    assert!(transfer::begin(&f.0, "one", "bad.exe".into(), 5).is_err());
    assert!(transfer::begin(&f.0, "one", "huge.txt".into(), 120_001).is_err());
    assert!(transfer::begin(&f.0, "one", "empty.txt".into(), 0).is_err());
    let id = transfer::begin(&f.0, "one", "test.txt".into(), 3).unwrap();
    assert!(transfer::append(&f.0, "two", &id, 0, b"a").is_err());
    assert!(transfer::append(&f.0, "one", "../escape", 0, b"a").is_err());
    assert!(transfer::append(&f.0, "one", &id, 1, b"a").is_err());
    assert!(transfer::append(&f.0, "one", &id, 0, b"abcd").is_err());
    transfer::append(&f.0, "one", &id, 0, b"ab").unwrap();
    assert!(transfer::append(&f.0, "one", &id, 0, b"a").is_err());
    assert!(transfer::cancel(&f.0, "two", &id).is_err());
    transfer::cancel(&f.0, "one", &id).unwrap();
    f.clean_work();
}

#[test]
fn failed_finish_cleans_incomplete_and_invalid_files() {
    let f = Fixture::new();
    let partial = transfer::begin(&f.0, "one", "test.txt".into(), 8).unwrap();
    transfer::append(&f.0, "one", &partial, 0, b"partial").unwrap();
    assert!(transfer::finish(&f.0, "one", &partial).is_err());
    let invalid = transfer::begin(&f.0, "one", "fake.png".into(), 5).unwrap();
    transfer::append(&f.0, "one", &invalid, 0, b"image").unwrap();
    assert!(transfer::finish(&f.0, "one", &invalid).is_err());
    assert!(f.0.assets().unwrap().is_empty());
    f.clean_work();
}

#[test]
fn batch_import_preserves_successes_and_reports_each_failure() {
    let f = Fixture::new();
    let original = f.file("notes.txt", b"reference notes");
    let invalid = f.file("broken.pdf", b"not a PDF");
    let result = import_paths(
        &f.0,
        "one",
        vec![original.clone(), invalid, f.0.root.clone()],
    );
    assert_eq!(result.assets.len(), 1);
    assert_eq!(result.errors.len(), 2);
    assert_eq!(std::fs::read(original).unwrap(), b"reference notes");
    f.clean_work();
}

#[test]
fn imports_image_audio_video_and_pdf_with_previews_and_ownership() {
    let f = Fixture::new();
    let mut sources = vec![];
    for (name, input, options) in [
        (
            "reference.png",
            "color=c=blue:s=32x32",
            vec!["-frames:v", "1", "-update", "1"],
        ),
        ("voice.wav", "sine=frequency=440", vec!["-t", "0.1"]),
        (
            "scene.mp4",
            "color=c=blue:s=32x32",
            vec!["-t", "0.1", "-pix_fmt", "yuv420p"],
        ),
    ] {
        let source = f.0.root.join(name);
        media::run(
            Command::new(media::binary("ffmpeg"))
                .args(["-v", "error", "-y", "-f", "lavfi", "-i", input])
                .args(options)
                .arg(&source),
        )
        .unwrap();
        sources.push(source);
    }
    sources.push(f.file("brief.pdf", b"%PDF-1.7\nfixture"));
    let result = import_paths(&f.0, "one", sources);
    assert!(result.errors.is_empty(), "{:?}", result.errors);
    assert_eq!(
        result
            .assets
            .iter()
            .map(|a| a.kind.as_str())
            .collect::<Vec<_>>(),
        vec!["image", "audio", "video", "document"]
    );
    for asset in &result.assets {
        assert!(Path::new(&asset.path).exists());
        if matches!(asset.kind.as_str(), "image" | "video") {
            assert!(Path::new(&asset.preview).exists());
        }
    }
    f.clean_work();
    project_storage::remove(&f.0, "one").unwrap();
    for asset in result.assets {
        assert!(!Path::new(&asset.path).exists());
        if !asset.preview.is_empty() {
            assert!(!Path::new(&asset.preview).exists());
        }
    }
}

#[tokio::test]
async fn deleting_project_cleans_in_flight_uploads() {
    let f = Fixture::new();
    let id = transfer::begin(&f.0, "one", "test.txt".into(), 3).unwrap();
    transfer::append(&f.0, "one", &id, 0, b"a").unwrap();
    project_storage::remove(&f.0, "one").unwrap();
    assert!(project_storage::working(&f.0, "one").await.is_err());
    f.clean_work();
}
