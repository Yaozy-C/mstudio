use crate::database::Store;
use base64::Engine;
use mstudio::{media, model::Asset};
use serde::Deserialize;
use tauri::Manager;
#[derive(Deserialize)]
pub struct Frame {
    data: String,
    duration: f64,
}
#[tauri::command]
pub async fn store_caption_animation(
    app: tauri::AppHandle,
    project_id: String,
    frames: Vec<Frame>,
    fps: u32,
    duration: f64,
    loop_animation: bool,
) -> Result<String, String> {
    if frames.is_empty()
        || frames.len() > 2002
        || frames.iter().map(|f| f.data.len()).sum::<usize>() > 128_000_000
    {
        return Err("字幕动画数据过大".into());
    }
    let guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        let id = media::id();
        let dir = store.media_root().join("captions");
        let work = dir.join(format!("{id}-work"));
        let output = dir.join(format!("{id}.mov"));
        let result = (|| -> anyhow::Result<String> {
            crate::project_storage::track_file(&store, &project_id, &work)?;
            crate::project_storage::track_file(&store, &project_id, &output)?;
            std::fs::create_dir_all(&work)?;
            let mut sources = vec![];
            let mut dimensions = None;
            for (i, frame) in frames.into_iter().enumerate() {
                anyhow::ensure!(frame.data.len() <= 12_000_000, "字幕帧过大");
                let bytes = base64::engine::general_purpose::STANDARD.decode(
                    frame
                        .data
                        .strip_prefix("data:image/png;base64,")
                        .ok_or_else(|| anyhow::anyhow!("字幕帧必须为 PNG"))?,
                )?;
                anyhow::ensure!(
                    bytes.len() >= 24 && bytes[..8] == [137, 80, 78, 71, 13, 10, 26, 10],
                    "字幕 PNG 无效"
                );
                let w = u32::from_be_bytes(bytes[16..20].try_into()?);
                let h = u32::from_be_bytes(bytes[20..24].try_into()?);
                anyhow::ensure!(
                    (64..=3840).contains(&w) && (64..=3840).contains(&h),
                    "字幕尺寸无效"
                );
                anyhow::ensure!(
                    dimensions.is_none() || dimensions == Some((w, h)),
                    "字幕帧尺寸不一致"
                );
                dimensions = Some((w, h));
                let path = work.join(format!("source-{i}.png"));
                std::fs::write(&path, bytes)?;
                sources.push((path, frame.duration));
            }
            mstudio::caption_animation::encode(
                &sources,
                fps,
                duration,
                loop_animation,
                &work,
                &output,
            )?;
            let (width, height) = dimensions.unwrap();
            let asset = Asset {
                id: id.clone(),
                name: "动态字幕".into(),
                kind: "video".into(),
                path: output.to_string_lossy().into(),
                preview: String::new(),
                width,
                height,
                duration,
                has_audio: false,
                generated: true,
                missing: false,
            };
            crate::project_storage::save_asset(&store, &project_id, &asset)?;
            Ok(id)
        })();
        let _ = std::fs::remove_dir_all(work);
        if result.is_err() {
            let _ = std::fs::remove_file(output);
        }
        result.map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
