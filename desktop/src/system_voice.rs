//! Platform voice synthesis, returning a file for the shared import pipeline.
use std::path::{Path, PathBuf};
#[cfg(target_os = "macos")]
use std::process::Command;

#[derive(serde::Serialize, serde::Deserialize)]
pub struct Voice {
    pub name: String,
    language: String,
}
#[cfg(target_os = "macos")]
pub fn installed_voices() -> Result<Vec<Voice>, String> {
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

#[cfg(windows)]
pub fn installed_voices() -> Result<Vec<Voice>, String> {
    let output =
        crate::windows_host::powershell(include_str!("system_voice.ps1"), r#"{"action":"list"}"#)?;
    serde_json::from_str(&output).map_err(|e| e.to_string())
}
#[cfg(not(any(windows, target_os = "macos")))]
pub fn installed_voices() -> Result<Vec<Voice>, String> {
    Ok(vec![])
}

pub fn synthesize(work: &Path, text: &str, voice: &str, rate: u32) -> Result<PathBuf, String> {
    #[cfg(target_os = "macos")]
    {
        let script = work.join("script.txt");
        let output = work.join("voice.aiff");
        std::fs::write(&script, text).map_err(|e| e.to_string())?;
        let status = Command::new("/usr/bin/say")
            .args(["-v", voice, "-r", &rate.to_string(), "-f"])
            .arg(script)
            .arg("-o")
            .arg(&output)
            .output()
            .map_err(|e| e.to_string())?;
        if !status.status.success() {
            return Err("本机配音生成失败，请检查系统声音".into());
        }
        Ok(output)
    }
    #[cfg(windows)]
    {
        let output = work.join("voice.wav");
        let request = serde_json::json!({"action":"speak", "text":text, "voice":voice,
            "rate":rate, "output":output});
        crate::windows_host::powershell(include_str!("system_voice.ps1"), &request.to_string())?;
        Ok(output)
    }
    #[cfg(not(any(windows, target_os = "macos")))]
    {
        let _ = (work, text, voice, rate);
        Err("System speech is unavailable".into())
    }
}

#[cfg(all(test, windows))]
mod tests {
    use super::*;

    #[test]
    fn windows_voice_writes_wave_file_from_literal_unicode_input() {
        let voices = installed_voices().unwrap();
        let Some(voice) = voices.first() else {
            eprintln!("No system voice installed; voice enumeration passed");
            return;
        };
        let root = std::env::temp_dir().join(format!("mstudio 配音 {}", mstudio::media::id()));
        std::fs::create_dir_all(&root).unwrap();
        let output = synthesize(&root, "Hello 世界. Quotes: \" ' $ ;", &voice.name, 180).unwrap();
        let bytes = std::fs::read(output).unwrap();
        assert!(bytes.len() > 44);
        assert_eq!(&bytes[..4], b"RIFF");
        assert_eq!(&bytes[8..12], b"WAVE");
        std::fs::remove_dir_all(root).unwrap();
    }
}
