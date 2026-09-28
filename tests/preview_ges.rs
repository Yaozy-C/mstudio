use mstudio::{
    media::{self, binary, run},
    model::{Clip, RenderSpec, Track},
    preview_ges,
};
use std::process::Command;
#[test]
fn ges_preparation_preserves_timing_effects_and_cache_identity() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("mstudio-ges-plan-{}", media::id()));
    std::fs::create_dir_all(&root)?;
    let source = root.join("red.mp4");
    run(Command::new(binary("ffmpeg"))
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=red:s=160x90:r=30:d=2",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&source))?;
    let asset = media::import(&source, &root)?;
    let original = std::fs::read(&asset.path)?;
    let mut spec = RenderSpec {
        width: 640,
        height: 360,
        fps: 30,
        tracks: vec![Track {
            id: "v".into(),
            kind: "video".into(),
            hidden: false,
            muted: true,
        }],
        clips: vec![Clip {
            id: "clip".into(),
            asset_id: asset.id.clone(),
            track_id: "v".into(),
            trim_in: 0.,
            trim_out: 2.,
            start: 0.5,
            speed: 2.,
            volume: 1.,
            visual: Some(serde_json::from_str(r#"{"effect":"grayscale"}"#)?),
            ..Default::default()
        }],
        captions: vec![],
    };
    let assets = vec![asset];
    let cache = root.join("cache");
    let plan = preview_ges::prepare(&spec, &assets, &cache, 640)?;
    assert_eq!(plan.duration, 1.5);
    assert_eq!(plan.layers[0].start, 0.5);
    assert_eq!(plan.layers[0].duration, 1.);
    assert!(plan.audio.is_none());
    let sharp = preview_ges::prepare(&spec, &assets, &cache, 1280)?;
    assert_eq!((sharp.width, sharp.height), (1280, 720));
    assert_eq!((sharp.layers[0].width, sharp.layers[0].height), (1280, 720));
    assert_ne!(sharp.layers[0].path, plan.layers[0].path);
    let file = &plan.layers[0].path;
    let modified = std::fs::metadata(file)?.modified()?;
    let probe = media::probe(std::path::Path::new(file))?;
    let duration: f64 = probe["format"]["duration"].as_str().unwrap().parse()?;
    assert!((duration - 1.).abs() < 0.04);
    let pixels = Command::new(binary("ffmpeg"))
        .args([
            "-v",
            "error",
            "-i",
            file,
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "-",
        ])
        .output()?;
    assert!(pixels.status.success());
    let rgb = &pixels.stdout[..3];
    assert!((rgb[0] as i16 - rgb[1] as i16).abs() < 4 && (rgb[1] as i16 - rgb[2] as i16).abs() < 4);
    let again = preview_ges::prepare(&spec, &assets, &cache, 640)?;
    assert_eq!(again.layers[0].path, *file);
    assert_eq!(std::fs::metadata(file)?.modified()?, modified);
    spec.clips[0].speed = 1.;
    let changed = preview_ges::prepare(&spec, &assets, &cache, 640)?;
    assert_ne!(changed.layers[0].path, *file);
    spec.tracks[0].hidden = true;
    assert!(
        preview_ges::prepare(&spec, &assets, &cache, 640)?
            .layers
            .is_empty()
    );
    assert_eq!(original, std::fs::read(&assets[0].path)?);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
