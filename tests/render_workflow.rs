use mstudio::{
    media::{self, binary, run},
    model::{Clip, RenderSpec, Track},
    render,
};
use std::process::Command;

#[test]
fn exports_trim_speed_silent_media_and_music() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("mstudio-test-{}", media::id()));
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
            "testsrc2=size=160x120:rate=30:duration=3",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=3",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&source))?;
    let silent = root.join("silent.mp4");
    run(Command::new(binary("ffmpeg"))
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=blue:size=160x120:rate=30:duration=1",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
        ])
        .arg(&silent))?;
    let tone = root.join("tone.wav");
    run(Command::new(binary("ffmpeg"))
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=220:duration=3",
        ])
        .arg(&tone))?;
    let a = media::import(&source, &root)?;
    let b = media::import(&silent, &root)?;
    let music = media::import(&tone, &root)?;
    assert!(a.has_audio);
    assert!(!b.has_audio);
    assert_eq!(music.kind, "audio");
    assert!(std::path::Path::new(&a.preview).exists());
    let clip = Clip {
        id: "a".into(),
        start: 0.,
        track_id: "v1".into(),
        asset_id: a.id.clone(),
        trim_in: 0.5,
        trim_out: 2.5,
        speed: 2.0,
        volume: 0.7,
        ..Default::default()
    };
    let spec = RenderSpec {
        clips: vec![
            clip.clone(),
            Clip {
                id: "b".into(),
                start: 1.,
                track_id: "v1".into(),
                asset_id: b.id.clone(),
                trim_in: 0.0,
                trim_out: 1.0,
                speed: 1.0,
                volume: 1.0,
                ..Default::default()
            },
            Clip {
                id: "music".into(),
                start: 0.,
                track_id: "a1".into(),
                asset_id: music.id.clone(),
                trim_in: 0.0,
                trim_out: 2.0,
                speed: 1.0,
                volume: 0.2,
                ..Default::default()
            },
        ],
        tracks: tracks(),
        width: 180,
        height: 320,
        fps: 30,
        ..Default::default()
    };
    let output = root.join("final.mp4");
    let mut progress = vec![];
    render::render(
        &spec,
        &[a.clone(), b, music],
        &root.join("work"),
        &output,
        |n, total| progress.push((n, total)),
    )?;
    let probe = media::probe(&output)?;
    let length: f64 = probe["format"]["duration"].as_str().unwrap().parse()?;
    assert!((length - 2.0).abs() < 0.15, "duration {length}");
    let streams = probe["streams"].as_array().unwrap();
    let video = streams.iter().find(|s| s["codec_type"] == "video").unwrap();
    assert_eq!(video["width"], 180);
    assert_eq!(video["height"], 320);
    assert!(streams.iter().any(|s| s["codec_type"] == "audio"));
    assert_eq!(progress.last(), Some(&(1, 1)));
    let mut invalid = clip;
    invalid.trim_out = 100.0;
    assert!(invalid.validate(&a).is_err());
    invalid.trim_out = 2.0;
    invalid.speed = f64::NAN;
    assert!(invalid.validate(&a).is_err());
    std::fs::remove_dir_all(root)?;
    Ok(())
}

#[test]
fn image_sequence_produces_exact_duration() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("mstudio-still-{}", media::id()));
    std::fs::create_dir_all(&root)?;
    let image = root.join("still.png");
    run(Command::new(binary("ffmpeg"))
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=c=red:size=100x200",
            "-frames:v",
            "1",
            "-update",
            "1",
        ])
        .arg(&image))?;
    let a = media::import(&image, &root)?;
    assert_eq!(a.kind, "image");
    let clip = Clip {
        id: "still".into(),
        start: 0.,
        track_id: "v1".into(),
        asset_id: a.id.clone(),
        trim_in: 0.0,
        trim_out: 1.5,
        speed: 1.0,
        volume: 0.0,
        ..Default::default()
    };
    let spec = RenderSpec {
        clips: vec![clip],
        tracks: tracks(),
        width: 160,
        height: 90,
        fps: 30,
        ..Default::default()
    };
    let out = root.join("still.mp4");
    render::render(&spec, &[a], &root.join("work"), &out, |_, _| {})?;
    let length: f64 = media::probe(&out)?["format"]["duration"]
        .as_str()
        .unwrap()
        .parse()?;
    assert!((length - 1.5).abs() < 0.1);
    std::fs::remove_dir_all(root)?;
    Ok(())
}

fn tracks() -> Vec<Track> {
    vec![
        Track {
            id: "v1".into(),
            kind: "video".into(),
            muted: false,
            hidden: false,
        },
        Track {
            id: "a1".into(),
            kind: "audio".into(),
            muted: false,
            hidden: false,
        },
    ]
}
