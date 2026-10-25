use crate::{
    assistant::{config::Profile, media_input},
    database::Store,
};
use anyhow::{Context, ensure};
use rig_core::message::{Message, UserContent};
use serde_json::{Value, json};
#[cfg(test)]
#[path = "image_read_observation_tests.rs"]
mod observation_tests;
#[path = "transition_frame.rs"]
mod transition_frame;
#[path = "video_frame.rs"]
mod video_frame;

pub fn read(store: &Store, project: &str, turn: &str, profile: &Profile, args: &Value) -> Value {
    let result = (|| -> anyhow::Result<Value> {
        let id = args["assetId"]
            .as_str()
            .filter(|id| !id.is_empty() && id.len() < 100)
            .context("Use a real assetId returned by inspect")?;
        ensure!(
            profile.inputs.image,
            "Image input is disabled for this chat model. Select an image-capable chat model in settings; no image sent."
        );
        let doc = crate::project_service::load(&store.db.lock().unwrap(), project)?;
        crate::previs::media::find(&doc, id).context(
            "Asset is outside this project; obtain a current-project assetId from inspect",
        )?;
        let assets = crate::previs::media::resolved(store, &doc)?;
        let asset = assets
            .iter()
            .find(|a| a.id == id)
            .context("Asset record not found")?;
        let (image_id, parts) = if args["transition"] == true {
            transition_frame::parts(store, project, &doc, args)?
        } else if asset.kind == "video" {
            video_frame::parts(store, asset, &doc, args)?
        } else {
            ensure!(
                asset.kind == "image",
                "This tool supports only images or video frames"
            );
            ensure!(
                args.get("clipId").is_none() && args.get("time").is_none(),
                "Still images do not accept video-frame parameters"
            );
            (id.to_owned(), media_input::parts(store, asset, profile)?)
        };
        let Message::User { content } =
            crate::assistant::agent::convert(&json!({"role":"user","content":parts}))
        else {
            anyhow::bail!("Image conversion failed");
        };
        let image = content
            .into_iter()
            .find_map(|p| match p {
                UserContent::Image(i) => Some(i),
                _ => None,
            })
            .context("Image content missing")?;
        // Record the snapshot used to produce these pixels, never a newer state
        // loaded after decoding. A concurrent edit must still cause a conflict.
        let mut keys = std::collections::BTreeSet::from([format!("asset:{id}")]);
        if let Some(clip_id) = args["clipId"].as_str() {
            keys.insert(format!("clip:{clip_id}"));
            if args["transition"] == true
                && let Some(left_id) = doc["clips"]
                    .as_array()
                    .and_then(|clips| clips.iter().find(|clip| clip["id"] == clip_id))
                    .and_then(|clip| clip["transition"]["fromClipId"].as_str())
            {
                keys.insert(format!("clip:{left_id}"));
                if let Some(asset_id) = doc["clips"]
                    .as_array()
                    .and_then(|clips| clips.iter().find(|clip| clip["id"] == left_id))
                    .and_then(|clip| clip["assetId"].as_str())
                {
                    keys.insert(format!("asset:{asset_id}"));
                }
            }
        }
        let mut db = store.db.lock().unwrap();
        let tx = db.transaction()?;
        crate::project_service::observations::remember(&tx, project, turn, &keys, &doc)?;
        tx.commit()?;
        // Use the same canonical multimodal tool-result path as restored images.
        Ok(json!({"ok":true,"imageId":image_id,"__offloadedImage":image}))
    })();
    result.unwrap_or_else(|e| json!({"error":e.to_string(),"code":"IMAGE_READ_FAILED"}))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn transition_reader_returns_composited_pixels_and_preserves_project_assets() {
        let root = std::env::temp_dir().join(format!("mstudio-seam-read-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let media = store.media_root().join("assets");
        std::fs::create_dir_all(&media).unwrap();
        let mut assets = vec![];
        for color in ["red", "blue"] {
            let path = media.join(format!("{color}.mp4"));
            mstudio::media::run(
                mstudio::media::command("ffmpeg")
                    .args([
                        "-v",
                        "error",
                        "-y",
                        "-f",
                        "lavfi",
                        "-i",
                        &format!("color={color}:s=160x120:r=30:d=1"),
                        "-c:v",
                        "libx264",
                    ])
                    .arg(&path),
            )
            .unwrap();
            let asset = json!({"id":color,"name":color,"kind":"video","path":path,"preview":"","duration":1,"width":160,"height":120,"hasAudio":false});
            store
                .db
                .lock()
                .unwrap()
                .execute(
                    "INSERT INTO assets VALUES(?1,?2)",
                    [color, &asset.to_string()],
                )
                .unwrap();
            assets.push(asset);
        }
        let clip = |id: &str, start: f64| json!({"id":id,"assetId":id,"start":start,"trimIn":0,"trimOut":1,"speed":1,"volume":0,"trackId":"v1"});
        let mut right = clip("blue", 1.);
        right["transition"] = json!({"fromClipId":"red","kind":"custom","duration":0.4,"design":{"curve":[[0,0],[1,1]]}});
        let doc = json!({"assets":assets,"clips":[clip("red",0.),right],"width":160,"height":120,"fps":30,"tracks":[{"id":"v1","kind":"video"}],"captions":[]});
        store
            .db
            .lock()
            .unwrap()
            .execute(
                "INSERT INTO projects VALUES('p','Test',?1,0)",
                [doc.to_string()],
            )
            .unwrap();
        let mut profile = Profile::default();
        profile.inputs.image = true;
        let args = json!({"assetId":"blue","clipId":"blue","time":0.2,"transition":true});
        let result = read(&store, "p", "turn", &profile, &args);
        assert_eq!(result["ok"], true, "{result}");
        assert!(result["imageId"].as_str().unwrap().contains("composited"));
        let observed: Vec<String> = store
            .db
            .lock()
            .unwrap()
            .prepare("SELECT target FROM project_observations WHERE turn_id='turn' ORDER BY target")
            .unwrap()
            .query_map([], |row| row.get(0))
            .unwrap()
            .collect::<rusqlite::Result<_>>()
            .unwrap();
        assert_eq!(
            observed,
            ["asset:blue", "asset:red", "clip:blue", "clip:red"]
        );
        let source = read(
            &store,
            "p",
            "source",
            &profile,
            &json!({"assetId":"blue","time":0.2}),
        );
        assert_ne!(result["__offloadedImage"], source["__offloadedImage"]);
        assert_eq!(store.assets().unwrap().len(), 2);
        assert!(
            read(
                &store,
                "p",
                "failed",
                &profile,
                &json!({"assetId":"red","clipId":"blue","time":0.2,"transition":true})
            )
            .get("error")
            .is_some()
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
    #[test]
    fn generated_project_image_returns_pixels_and_enforces_scope_and_model_inputs() {
        let root =
            std::env::temp_dir().join(format!("mstudio-image-read-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let media = store.media_root().join("assets");
        std::fs::create_dir_all(&media).unwrap();
        let path = media.join("result.png");
        std::fs::write(&path, [137, 80, 78, 71]).unwrap();
        let asset = json!({"id":"generated-1","name":"result.png","kind":"image","path":path,"preview":"","duration":0,"width":1,"height":1,"hasAudio":false,"generated":true,"missing":false});
        {
            let db = store.db.lock().unwrap();
            db.execute(
                "INSERT INTO assets VALUES('generated-1',?1)",
                [asset.to_string()],
            )
            .unwrap();
            db.execute(
                "INSERT INTO projects VALUES('p','Test',?1,0)",
                [json!({"assets":[asset]}).to_string()],
            )
            .unwrap();
            db.execute("INSERT INTO projects VALUES('other','Other','{}',0)", [])
                .unwrap();
        }
        let mut profile = Profile::default();
        profile.inputs.image = true;
        let args = json!({"assetId":"generated-1"});
        let result = read(&store, "p", "turn", &profile, &args);
        assert_eq!(result["ok"], true, "{result}");
        let image: rig_core::message::Image =
            serde_json::from_value(result["__offloadedImage"].clone()).unwrap();
        assert!(serde_json::to_string(&image).unwrap().contains("iVBORw=="));
        assert!(
            read(&store, "other", "failed", &profile, &args)
                .get("error")
                .is_some()
        );
        assert!(
            read(
                &store,
                "p",
                "failed",
                &profile,
                &json!({"assetId":"result.png"})
            )
            .get("error")
            .is_some()
        );
        profile.inputs.image = false;
        assert!(
            read(&store, "p", "failed", &profile, &args)
                .get("error")
                .is_some()
        );
        assert_eq!(
            store
                .db
                .lock()
                .unwrap()
                .query_row(
                    "SELECT count(*) FROM project_observations WHERE turn_id='failed'",
                    [],
                    |row| row.get::<_, i64>(0)
                )
                .unwrap(),
            0
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
