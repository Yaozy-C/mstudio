use mstudio::{
    audio_mix,
    media::{self, binary, run},
    model::{Caption, Clip, RenderSpec, Track},
    render,
};
use std::{path::Path, process::Command};
fn frame(path: &Path, time: &str) -> Vec<u8> {
    let out = Command::new(binary("ffmpeg"))
        .args(["-v", "error", "-ss", time, "-i"])
        .arg(path)
        .args([
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgb24",
            "pipe:1",
        ])
        .output()
        .unwrap();
    assert!(out.status.success());
    out.stdout
}
fn pixel(data: &[u8], x: usize, y: usize) -> [u8; 3] {
    data[(y * 160 + x) * 3..(y * 160 + x) * 3 + 3]
        .try_into()
        .unwrap()
}
#[test]
fn layered_composition_matches_positions_captions_and_delayed_audio() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("mstudio-layers-{}", media::id()));
    std::fs::create_dir_all(&root)?;
    for (name, color, seconds) in [("base", "blue", 4), ("top", "red", 1)] {
        run(Command::new(binary("ffmpeg"))
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                &format!("color=c={color}:s=160x120:r=30:d={seconds}"),
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
            ])
            .arg(root.join(format!("{name}.mp4"))))?;
    }
    let tone = root.join("tone.wav");
    run(Command::new(binary("ffmpeg"))
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:duration=2",
        ])
        .arg(&tone))?;
    let caption = root.join("caption.png");
    run(Command::new(binary("ffmpeg")).args(["-v","error","-y","-f","lavfi","-i","color=c=black@0:s=160x120,format=rgba,drawbox=x=4:y=4:w=12:h=12:color=white:t=fill:replace=1","-frames:v","1","-update","1"]).arg(&caption))?;
    let assets = [
        media::import(&root.join("base.mp4"), &root)?,
        media::import(&root.join("top.mp4"), &root)?,
        media::import(&tone, &root)?,
        media::import(&caption, &root)?,
    ];
    let clip = |id: &str, asset: usize, track: &str, start: f64, out: f64| Clip {
        id: id.into(),
        asset_id: assets[asset].id.clone(),
        track_id: track.into(),
        start,
        trim_out: out,
        speed: 1.,
        volume: 0.5,
        ..Default::default()
    };
    let tracks = vec![
        Track {
            id: "v1".into(),
            kind: "video".into(),
            hidden: false,
            muted: false,
        },
        Track {
            id: "v2".into(),
            kind: "video".into(),
            hidden: false,
            muted: false,
        },
        Track {
            id: "a1".into(),
            kind: "audio".into(),
            hidden: false,
            muted: false,
        },
    ];
    let mut overlay = clip("top", 1, "v2", 1., 1.);
    overlay.scale = Some(0.5);
    let mut voice = clip("voice", 2, "a1", 1., 2.);
    voice.fade_in = Some(0.2);
    voice.fade_out = Some(0.2);
    let spec = RenderSpec {
        clips: vec![clip("base", 0, "v1", 0., 4.), overlay, voice],
        tracks,
        captions: vec![Caption {
            start: 2.,
            end: 3.,
            text: "test".into(),
            asset_id: assets[3].id.clone(),
        }],
        width: 160,
        height: 120,
        fps: 30,
    };
    let out = root.join("final.mp4");
    render::render(&spec, &assets, &root.join("work"), &out, |_, _| {})?;
    let metadata = media::probe(&out)?;
    let length: f64 = metadata["format"]["duration"].as_str().unwrap().parse()?;
    assert!((length - 4.).abs() < 0.08);
    let before = frame(&out, "0.5");
    assert!(pixel(&before, 80, 60)[2] > 230);
    let during = frame(&out, "1.5");
    assert!(pixel(&during, 80, 60)[0] > 230);
    assert!(pixel(&during, 10, 60)[2] > 230);
    let after = frame(&out, "2.5");
    assert!(pixel(&after, 80, 60)[2] > 230);
    assert!(pixel(&after, 8, 8).iter().all(|c| *c > 220));
    let ended = frame(&out, "3.2");
    assert!(pixel(&ended, 8, 8)[2] > 230 && pixel(&ended, 8, 8)[0] < 20);
    let audio = Command::new(binary("ffmpeg"))
        .args(["-v", "error", "-i"])
        .arg(&out)
        .args(["-vn", "-f", "f32le", "-ar", "48000", "-ac", "1", "pipe:1"])
        .output()?;
    assert!(audio.status.success());
    let samples: Vec<f32> = audio
        .stdout
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let rms = |from: f64, to: f64| {
        let values = &samples[(from * 48000.) as usize..(to * 48000.) as usize];
        (values.iter().map(|x| x * x).sum::<f32>() / values.len() as f32).sqrt()
    };
    assert!(rms(0.2, 0.8) < 0.001);
    assert!(rms(1.3, 2.7) > 0.02);
    assert!(rms(3.2, 3.8) < 0.001);
    // Preview and export share exact trims, delay and fades; the mix remains
    // alive through silent tails so clip boundaries never replace its clock.
    let preview = root.join("preview.m4a");
    assert!(audio_mix::preview(&spec, &assets, &preview)?);
    let preview_meta = media::probe(&preview)?;
    let preview_length: f64 = preview_meta["format"]["duration"]
        .as_str()
        .unwrap()
        .parse()?;
    assert!((preview_length - 4.1).abs() < 0.06);
    let mixed = Command::new(binary("ffmpeg"))
        .args(["-v", "error", "-i"])
        .arg(&preview)
        .args(["-vn", "-f", "f32le", "-ar", "48000", "-ac", "1", "pipe:1"])
        .output()?;
    assert!(mixed.status.success());
    let mixed_samples: Vec<f32> = mixed
        .stdout
        .chunks_exact(4)
        .map(|b| f32::from_le_bytes(b.try_into().unwrap()))
        .collect();
    let difference = samples
        .iter()
        .zip(&mixed_samples)
        .take(4 * 48000)
        .map(|(a, b)| (a - b).powi(2))
        .sum::<f32>()
        / (4 * 48000) as f32;
    assert!(
        difference.sqrt() < 0.002,
        "preview audio diverged from export"
    );
    let mut muted = spec.clone();
    muted.tracks.iter_mut().for_each(|track| track.muted = true);
    assert!(!audio_mix::preview(
        &muted,
        &assets,
        &root.join("silent.m4a")
    )?);
    assert!(!root.join("silent.m4a").exists());
    let mut invalid = spec;
    invalid.clips[0].start = f64::NAN;
    assert!(
        render::render(
            &invalid,
            &assets,
            &root.join("invalid"),
            &root.join("bad.mp4"),
            |_, _| {}
        )
        .is_err()
    );
    assert!(!root.join("bad.mp4").exists());
    std::fs::remove_dir_all(root)?;
    Ok(())
}
