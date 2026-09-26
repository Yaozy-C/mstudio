//! Bounded waveform data, generated once per source revision, never on the UI thread.
use crate::database::Store;
use std::{
    hash::{Hash, Hasher},
    io::{BufReader, Read},
    process::{Command, Stdio},
};
use tauri::Manager;
#[tauri::command]
pub async fn audio_waveform(
    app: tauri::AppHandle,
    project_id: String,
    asset_id: String,
) -> Result<Vec<f32>, String> {
    let guard = crate::project_storage::working(&app.state::<Store>(), &project_id)
        .await
        .map_err(|e| e.to_string())?;
    tauri::async_runtime::spawn_blocking(move || {
        let _guard = guard;
        static LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
        let _lock = LOCK.lock().map_err(|e| e.to_string())?;
        let store = app.state::<Store>();
        let asset = store
            .assets()
            .map_err(|e| e.to_string())?
            .into_iter()
            .find(|a| a.id == asset_id)
            .ok_or("素材不存在")?;
        if !asset.has_audio || asset.missing {
            return Err("素材没有可读取的音频".into());
        }
        let meta = std::fs::metadata(&asset.path).map_err(|e| e.to_string())?;
        let mut hash = std::collections::hash_map::DefaultHasher::new();
        asset.path.hash(&mut hash);
        meta.len().hash(&mut hash);
        meta.modified().map_err(|e| e.to_string())?.hash(&mut hash);
        asset.duration.to_bits().hash(&mut hash);
        let folder = store.media_root().join("proxies");
        std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
        let path = folder.join(format!("waveform-v1-{:x}.json", hash.finish()));
        crate::project_storage::track_file(&store, &project_id, &path)
            .map_err(|e| e.to_string())?;
        if let Ok(data) = std::fs::read(&path)
            && let Ok(peaks) = serde_json::from_slice::<Vec<f32>>(&data)
            && peaks.len() == 2048
            && peaks
                .iter()
                .all(|v| v.is_finite() && (0.0..=1.0).contains(v))
        {
            return Ok(peaks);
        }
        let mut child = Command::new(mstudio::media::binary("ffmpeg"))
            .args([
                "-v",
                "error",
                "-nostdin",
                "-i",
                &asset.path,
                "-vn",
                "-ac",
                "2",
                "-ar",
                "8000",
                "-f",
                "f32le",
                "pipe:1",
            ])
            .stdout(Stdio::piped())
            .stderr(Stdio::null())
            .spawn()
            .map_err(|e| e.to_string())?;
        let input = BufReader::new(child.stdout.take().ok_or("无法读取波形")?);
        let peaks = match sample_peaks(input, asset.duration) {
            Ok(peaks) => peaks,
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                return Err(e.to_string());
            }
        };
        if !child.wait().map_err(|e| e.to_string())?.success() {
            return Err("音频波形解码失败".into());
        }
        let data = serde_json::to_vec(&peaks).map_err(|e| e.to_string())?;
        std::fs::write(&path, data).map_err(|e| e.to_string())?;
        Ok(peaks)
    })
    .await
    .map_err(|e| e.to_string())?
}

fn sample_peaks(mut input: impl Read, duration: f64) -> std::io::Result<Vec<f32>> {
    let mut peaks = vec![0.0f32; 2048];
    let mut frame = [0u8; 8];
    let mut index = 0usize;
    let frames = (duration * 8000.).max(1.);
    loop {
        match input.read_exact(&mut frame) {
            Ok(()) => {
                let peak = f32::from_le_bytes(frame[..4].try_into().unwrap())
                    .abs()
                    .max(f32::from_le_bytes(frame[4..].try_into().unwrap()).abs());
                let bin = ((index as f64 / frames * 2048.) as usize).min(2047);
                if peak.is_finite() {
                    peaks[bin] = peaks[bin].max(peak.min(1.));
                }
                index += 1;
            }
            Err(e) if e.kind() == std::io::ErrorKind::UnexpectedEof => break,
            Err(e) => {
                return Err(e);
            }
        }
    }

    Ok(peaks)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn stereo_peaks_remain_bounded_and_keep_source_positions() {
        let mut pcm = Vec::new();
        for i in 0..8000 {
            let level: f32 = if i < 4000 { 0. } else { 0.5 };
            pcm.extend_from_slice(&level.to_le_bytes());
            pcm.extend_from_slice(&(-level).to_le_bytes());
        }
        let peaks = sample_peaks(&pcm[..], 1.).unwrap();
        assert_eq!(peaks.len(), 2048);
        assert!(peaks[..1024].iter().all(|v| *v == 0.));
        assert!(peaks[1024..].iter().all(|v| *v == 0.5));
    }
}
