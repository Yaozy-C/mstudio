use mstudio::{
    media::{self, binary, run},
    proxy,
};
use std::process::Command;
#[test]
fn preview_proxy_preserves_time_audio_and_original_and_reuses_cache() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("mstudio-proxy-test-{}", media::id()));
    std::fs::create_dir_all(&root)?;
    let source = root.join("source.mp4");
    run(Command::new(binary("ffmpeg"))
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "testsrc2=size=1280x720:rate=30:duration=2",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
            "-c:v",
            "libx264",
            "-preset",
            "ultrafast",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&source))?;
    let asset = media::import(&source, &root)?;
    let original = std::fs::read(&asset.path)?;
    let output = proxy::prepare(&asset, &root)?;
    let metadata = media::probe(&output)?;
    let streams = metadata["streams"].as_array().unwrap();
    let video = streams.iter().find(|s| s["codec_type"] == "video").unwrap();
    assert!(video["width"].as_u64().unwrap() <= 640);
    assert!(video["height"].as_u64().unwrap() <= 640);
    assert_eq!(video["codec_name"], "h264");
    assert!(streams.iter().any(|s| s["codec_type"] == "audio"));
    let duration = metadata["format"]["duration"]
        .as_str()
        .unwrap()
        .parse::<f64>()?;
    assert!((duration - asset.duration).abs() < 0.05);
    assert_eq!(original, std::fs::read(&asset.path)?);
    let modified = std::fs::metadata(&output)?.modified()?;
    assert_eq!(proxy::prepare(&asset, &root)?, output);
    assert_eq!(std::fs::metadata(&output)?.modified()?, modified);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
