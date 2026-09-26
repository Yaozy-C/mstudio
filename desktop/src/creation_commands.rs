use crate::database::Store;
use base64::Engine;
use mstudio::{media, model::Asset};
use serde::Serialize;
use std::process::Command;
use tauri::Manager;

#[derive(Serialize)]
pub struct Voice {
    name: String,
    language: String,
}
fn installed_voices() -> Result<Vec<Voice>, String> {
    let out = Command::new("/usr/bin/say")
        .args(["-v", "?"])
        .output()
        .map_err(|e| e.to_string())?;
    if !out.status.success() {
        return Err("无法读取本机声音".into());
    }
    Ok(String::from_utf8_lossy(&out.stdout)
        .lines()
        .filter_map(|line| {
            let fields: Vec<_> = line.split('#').next()?.split_whitespace().collect();
            let i = fields
                .iter()
                .position(|v| v.len() >= 5 && v.as_bytes().get(2) == Some(&b'_'))?;
            Some(Voice {
                name: fields[..i].join(" "),
                language: fields[i].into(),
            })
        })
        .collect())
}
#[tauri::command]
pub async fn list_voices() -> Result<Vec<Voice>, String> {
    tauri::async_runtime::spawn_blocking(installed_voices)
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn generate_voice(
    app: tauri::AppHandle,
    project_id: String,
    text: String,
    voice: String,
    rate: u32,
) -> Result<Asset, String> {
    if text.trim().is_empty() || text.chars().count() > 8000 || !(80..=450).contains(&rate) {
        return Err("配音文字须为 1–8000 字，语速 80–450".into());
    }
    let guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        if !installed_voices()?.iter().any(|v| v.name == voice) {
            return Err("请选择已安装的声音".into());
        }
        let store = app.state::<Store>();
        let work = store.media_root().join("voice-work").join(media::id());
        crate::project_storage::track_file(&store, &project_id, &work)
            .map_err(|e| e.to_string())?;
        std::fs::create_dir_all(&work).map_err(|e| e.to_string())?;
        let result = (|| {
            let script = work.join("script.txt");
            let spoken = work.join("voice.aiff");
            let wav = work.join("配音.wav");
            std::fs::write(&script, text).map_err(|e| e.to_string())?;
            let out = Command::new("/usr/bin/say")
                .args(["-v", &voice, "-r", &rate.to_string(), "-f"])
                .arg(script)
                .arg("-o")
                .arg(&spoken)
                .output()
                .map_err(|e| e.to_string())?;
            if !out.status.success() {
                return Err("本机配音生成失败，请检查系统声音".into());
            }
            media::run(
                Command::new(media::binary("ffmpeg"))
                    .args(["-v", "error", "-y", "-i"])
                    .arg(spoken)
                    .args(["-ar", "48000", "-ac", "2"])
                    .arg(&wav),
            )
            .map_err(|e| e.to_string())?;
            let mut asset = media::import(&wav, &store.media_root()).map_err(|e| e.to_string())?;
            asset.generated = true;
            crate::project_storage::save_asset(&store, &project_id, &asset)
                .map_err(|e| e.to_string())?;
            Ok(asset)
        })();
        let _ = std::fs::remove_dir_all(work);
        result
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn store_caption_image(
    app: tauri::AppHandle,
    project_id: String,
    data: String,
) -> Result<String, String> {
    if data.len() > 12_000_000 {
        return Err("字幕图片过大".into());
    }
    let bytes = base64::engine::general_purpose::STANDARD
        .decode(
            data.strip_prefix("data:image/png;base64,")
                .ok_or("仅支持 PNG 字幕")?,
        )
        .map_err(|e| e.to_string())?;
    if bytes.len() < 24 || bytes[..8] != [137, 80, 78, 71, 13, 10, 26, 10] {
        return Err("字幕 PNG 无效".into());
    }
    let w = u32::from_be_bytes(bytes[16..20].try_into().unwrap());
    let h = u32::from_be_bytes(bytes[20..24].try_into().unwrap());
    if !(64..=3840).contains(&w) || !(64..=3840).contains(&h) {
        return Err("字幕尺寸无效".into());
    }
    let guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        let store = app.state::<Store>();
        let id = media::id();
        let dir = store.media_root().join("captions");
        std::fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
        let path = dir.join(format!("{id}.png"));
        crate::project_storage::track_file(&store, &project_id, &path)
            .map_err(|e| e.to_string())?;
        std::fs::write(&path, bytes).map_err(|e| e.to_string())?;
        let asset = Asset {
            missing: false,
            generated: true,
            id: id.clone(),
            name: "字幕画面".into(),
            kind: "image".into(),
            path: path.to_string_lossy().into(),
            preview: String::new(),
            duration: 1.,
            width: w,
            height: h,
            has_audio: false,
        };
        crate::project_storage::save_asset(&store, &project_id, &asset)
            .map_err(|e| e.to_string())?;
        Ok(id)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub async fn save_subtitles(text: String) -> Result<Option<String>, String> {
    if text.len() > 5_000_000 {
        return Err("字幕过大".into());
    }
    let file = rfd::AsyncFileDialog::new()
        .set_file_name("字幕.srt")
        .add_filter("SRT", &["srt"])
        .save_file()
        .await;
    if let Some(file) = file {
        std::fs::write(file.path(), text).map_err(|e| e.to_string())?;
        return Ok(Some(file.path().to_string_lossy().into()));
    }
    Ok(None)
}
