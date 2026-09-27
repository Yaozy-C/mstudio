use super::{
    attachments::{self, Reference},
    context, history,
};
use crate::database::Store;
use mstudio::model::Asset;
use serde_json::{Value, json};
pub(super) fn fixture() -> (std::path::PathBuf, Store, Value) {
    let root = std::env::temp_dir().join(format!("mstudio-attachments-{}", mstudio::media::id()));
    let store = Store::open(root.clone()).unwrap();
    std::fs::create_dir_all(root.join("previews")).unwrap();
    std::fs::create_dir_all(root.join("assets")).unwrap();
    store
        .db
        .lock()
        .unwrap()
        .execute("INSERT INTO projects VALUES('p','test','{}',0)", [])
        .unwrap();
    let mut assets = vec![];
    for (id, kind) in [("image", "image"), ("video", "video"), ("audio", "audio")] {
        let preview = root.join("previews").join(format!("{id}.jpg"));
        std::fs::write(&preview, [0xff, 0xd8, 0xff, 0xd9]).unwrap();
        let ext = match kind {
            "image" => "jpg",
            "video" => "mp4",
            _ => "mp3",
        };
        let path = root.join("assets").join(format!("{id}.{ext}"));
        std::fs::write(&path, b"test-source-content").unwrap();
        let asset = Asset {
            missing: false,
            generated: false,
            id: id.into(),
            name: format!("test-{id}"),
            kind: kind.into(),
            path: path.to_string_lossy().into(),
            preview: preview.to_string_lossy().into(),
            duration: 8.,
            width: 100,
            height: 100,
            has_audio: kind == "audio",
        };
        crate::project_storage::save_asset(&store, "p", &asset).unwrap();
        assets.push(asset);
    }
    let doc = json!({"id":"p","assets":assets,"nodes":[{"id":"script","kind":"text","title":"script attachment","text":"Keep the main character."},{"id":"shot","kind":"shot","title":"shot attachment","text":"Only change the light.","resultAssetId":"video"}]});
    store
        .db
        .lock()
        .unwrap()
        .execute(
            "UPDATE projects SET document=?1 WHERE id='p'",
            [doc.to_string()],
        )
        .unwrap();
    (root, store, doc)
}
pub(super) fn multimodal() -> super::config::Profile {
    super::config::Profile {
        adapter: "gemini-native".into(),
        inputs: super::config::Inputs {
            image: true,
            audio: true,
            video: true,
            document: true,
        },
        ..Default::default()
    }
}

#[test]
fn image_models_receive_video_references_for_on_demand_frames_not_fake_thumbnails() {
    let (root, store, doc) = fixture();
    let mut profile = super::config::Profile::default();
    profile.inputs.image = true;
    let result = attachments::payload(
        &store,
        &doc,
        "检查调色",
        &[Reference {
            kind: "asset".into(),
            id: "video".into(),
        }],
        &profile,
    )
    .unwrap();
    let text = result.to_string();
    assert!(text.contains("mstudio_read_image"));
    assert!(text.contains("metadata only"));
    assert!(!text.contains("data:image"));
    assert!(!text.contains("test-source-content"));
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
fn reference(kind: &str, id: &str) -> Reference {
    Reference {
        kind: kind.into(),
        id: id.into(),
    }
}
#[test]
fn explicit_attachments_are_project_owned_bounded_and_persist_without_image_bytes() {
    let (root, store, doc) = fixture();
    let refs = vec![
        reference("node", "script"),
        reference("asset", "image"),
        reference("node", "shot"),
        reference("asset", "audio"),
    ];
    let payload =
        attachments::payload(&store, &doc, "Revise this storyboard", &refs, &multimodal()).unwrap();
    assert_eq!(payload[0]["attachments"].as_array().unwrap().len(), 4);
    assert_eq!(
        payload
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["type"] == "image_url")
            .count(),
        1
    );
    let text = payload[0]["text"].as_str().unwrap();
    assert!(text.contains("Keep the main character."));
    assert!(text.contains("original content"));
    assert_eq!(
        payload
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["type"] == "media")
            .count(),
        1
    );
    let stored = context::text_only(&payload);
    history::append(
        &store,
        "p",
        "Revise this storyboard",
        &stored,
        "Done",
        "test",
    )
    .unwrap();
    let restored = history::read(&store, "p").unwrap();
    assert_eq!(
        restored[0].payload[0]["attachments"],
        payload[0]["attachments"]
    );
    assert!(!restored[0].payload.to_string().contains("data:image"));
    assert!(context::assemble(&restored, payload, json!({})).is_ok());
    assert!(attachments::payload(&store, &doc, "Edit clip", &refs, &Default::default()).is_err());
    assert!(!stored.to_string().contains("base64"));
    for invalid in [
        reference("node", "deleted"),
        reference("asset", "another-project"),
        reference("file", "/etc/passwd"),
    ] {
        assert!(attachments::payload(&store, &doc, "Inspect", &[invalid], &multimodal()).is_err());
    }
    let excessive = (0..13)
        .map(|_| reference("asset", "image"))
        .collect::<Vec<_>>();
    assert!(attachments::payload(&store, &doc, "Inspect", &excessive, &multimodal()).is_err());
    let duplicate = attachments::payload(
        &store,
        &doc,
        "Inspect",
        &[reference("asset", "video"), reference("node", "shot")],
        &multimodal(),
    )
    .unwrap();
    assert_eq!(
        duplicate
            .as_array()
            .unwrap()
            .iter()
            .filter(|p| p["type"] == "media")
            .count(),
        1
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
#[test]
fn explicit_timeline_attachment_contains_exact_clip_and_linked_dialogue() {
    let (root, store, mut doc) = fixture();
    doc["nodes"][1]["shot"] =
        json!({"planId":"plan","order":3,"duration":6,"dialogue":"keep this line"});
    doc["clips"] = json!([{"id":"cut","assetId":"video","shotId":"shot","start":11,"trimIn":2,"trimOut":8,"speed":1,"trackId":"v1"}]);
    let value = attachments::payload(
        &store,
        &doc,
        "fix the lid",
        &[reference("clip", "cut")],
        &multimodal(),
    )
    .unwrap();
    assert!(value.to_string().contains("keep this line"));
    assert!(value.to_string().contains("trimIn"));
    assert!(
        attachments::payload(
            &store,
            &doc,
            "fix",
            &[reference("clip", "missing")],
            &multimodal()
        )
        .is_err()
    );
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn shot_reference_is_text_and_directory_without_implicitly_reading_frames() {
    let (root, store, mut doc) = fixture();
    doc["nodes"][1]["shot"] = json!({"frames":[{"assetId":"image"}],"takes":[{"assetId":"video"}]});
    doc["nodes"][1]["references"] = json!([{"assetId":"image","purpose":"product"}]);
    let payload = attachments::payload(
        &store,
        &doc,
        "Review shot",
        &[reference("node", "shot")],
        &Default::default(),
    )
    .unwrap();
    assert_eq!(payload.as_array().unwrap().len(), 1);
    assert!(payload[0]["attachments"][0]["assetId"].is_null());
    assert!(
        payload[0]["text"]
            .as_str()
            .unwrap()
            .contains("Only change the light.")
    );
    assert!(payload[0]["text"].as_str().unwrap().contains("image"));
    std::fs::remove_dir_all(root).unwrap();
}
