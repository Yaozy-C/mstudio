//! Bounded frame reads reuse the existing image tool and multimodal result path.
use crate::database::Store;
use anyhow::{Context, Result, ensure};
use base64::{Engine, engine::general_purpose::STANDARD};
use mstudio::{model::Asset, visual::Visual};
use serde_json::{Value, json};
use std::path::Path;

pub fn parts(
    store: &Store,
    asset: &Asset,
    doc: &Value,
    args: &Value,
) -> Result<(String, Vec<Value>)> {
    let time = args["time"].as_f64().context(
        "Video reads require time in seconds: source time for assets, clip-local time with clipId",
    )?;
    ensure!(time.is_finite() && time >= 0., "Invalid frame time");
    let (source_time, visual, label) = if let Some(id) = args.get("clipId") {
        let id = id.as_str().context("clipId must be a clip ID")?;
        let clip = doc["clips"]
            .as_array()
            .and_then(|a| a.iter().find(|c| c["id"] == id))
            .context("Clip not found")?;
        ensure!(
            clip["assetId"] == asset.id,
            "Clip does not reference the specified asset"
        );
        let start = clip["trimIn"].as_f64().context("Invalid clip trim")?;
        let end = clip["trimOut"].as_f64().context("Invalid clip trim")?;
        let speed = clip["speed"].as_f64().context("Invalid clip speed")?;
        ensure!(
            speed > 0. && time < (end - start) / speed,
            "Time outside visible clip range (end excluded)"
        );
        let visual: Option<Visual> =
            serde_json::from_value(clip.get("visual").cloned().unwrap_or(Value::Null))?;
        if let Some(v) = &visual {
            v.validate()?;
        }
        (
            start + time * speed,
            visual,
            format!("clip:{id}@{time:.6}s/graded"),
        )
    } else {
        (time, None, format!("asset:{}@{time:.6}s/source", asset.id))
    };
    ensure!(
        source_time.is_finite() && source_time >= 0. && source_time < asset.duration,
        "Frame time outside source duration (end excluded)"
    );
    let path = Path::new(&asset.path).canonicalize()?;
    ensure!(
        path.starts_with(store.media_root().join("assets").canonicalize()?),
        "Asset path is outside the application media library"
    );
    let grade = mstudio::visual::ffmpeg(visual.as_ref(), asset.width as i64, asset.height as i64)?;
    frame(&path, source_time, &grade, label)
}

pub(super) fn frame(
    path: &Path,
    source_time: f64,
    grade: &str,
    label: String,
) -> Result<(String, Vec<Value>)> {
    let scale = "scale=960:960:force_original_aspect_ratio=decrease,setsar=1";
    let filter = if grade.is_empty() {
        scale.to_owned()
    } else {
        format!("{grade},{scale}")
    };
    let output = mstudio::media::command("ffmpeg")
        .args([
            "-v",
            "error",
            "-nostdin",
            "-threads",
            "2",
            "-ss",
            &source_time.to_string(),
            "-i",
        ])
        .arg(path)
        .args([
            "-map",
            "0:v:0",
            "-frames:v",
            "1",
            "-an",
            "-vf",
            &filter,
            "-f",
            "image2pipe",
            "-c:v",
            "mjpeg",
            "-q:v",
            "3",
            "pipe:1",
        ])
        .output()
        .context("Cannot start FFmpeg frame extraction")?;
    ensure!(
        output.status.success() && !output.stdout.is_empty(),
        "Cannot decode the specified frame: {}",
        String::from_utf8_lossy(&output.stderr)
            .chars()
            .take(600)
            .collect::<String>()
    );
    ensure!(
        output.stdout.len() <= 2 * 1024 * 1024,
        "Extracted image exceeds size limit"
    );
    let url = format!("data:image/jpeg;base64,{}", STANDARD.encode(output.stdout));
    Ok((
        label,
        vec![json!({"type":"image_url","image_url":{"url":url}})],
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn actual_video_seeking_applies_trim_speed_and_grade_without_creating_assets() {
        let root = std::env::temp_dir().join(format!("mstudio-frame-{}", mstudio::media::id()));
        let store = Store::open(root.clone()).unwrap();
        let dir = store.media_root().join("assets");
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("source.mp4");
        mstudio::media::run(
            mstudio::media::command("ffmpeg")
                .args([
                    "-v",
                    "error",
                    "-y",
                    "-f",
                    "lavfi",
                    "-i",
                    "testsrc2=size=160x120:rate=10:duration=3",
                    "-c:v",
                    "libx264",
                ])
                .arg(&path),
        )
        .unwrap();
        let asset: Asset = serde_json::from_value(json!({"id":"v","name":"test","kind":"video","path":path,"preview":"","duration":3,"width":160,"height":120,"hasAudio":false})).unwrap();
        let mut doc = json!({"clips":[{"id":"c","assetId":"v","trimIn":1,"trimOut":3,"speed":2}]});
        let source = parts(&store, &asset, &doc, &json!({"time":1.5})).unwrap();
        let clip = parts(&store, &asset, &doc, &json!({"clipId":"c","time":0.25})).unwrap();
        assert_eq!(source.1, clip.1, "clip time must map to exact source time");
        let beginning = parts(&store, &asset, &doc, &json!({"time":0})).unwrap();
        assert_ne!(beginning.1, clip.1);
        doc["clips"][0]["visual"] = json!({"effect":"grayscale"});
        let grade = parts(&store, &asset, &doc, &json!({"clipId":"c","time":0.25})).unwrap();
        assert_ne!(grade.1, clip.1, "grading must affect returned pixels");
        for args in [
            json!({}),
            json!({"time":-1}),
            json!({"time":3}),
            json!({"time":0,"clipId":"missing"}),
            json!({"time":1,"clipId":"c"}),
        ] {
            assert!(parts(&store, &asset, &doc, &args).is_err(), "{args}");
        }
        assert_eq!(
            std::fs::read_dir(&dir).unwrap().count(),
            1,
            "frame reads must not pollute asset library"
        );
        drop(store);
        std::fs::remove_dir_all(root).unwrap();
    }
}
