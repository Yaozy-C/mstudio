use mstudio::{
    caption_animation, composition,
    media::{self, binary, run},
    model::{Caption, Clip, RenderSpec, Track},
};
use std::{path::Path, process::Command};
fn frame(path: &Path, t: f64) -> Vec<u8> {
    let out = Command::new(binary("ffmpeg"))
        .args(["-v", "error", "-ss", &t.to_string(), "-i"])
        .arg(path)
        .args([
            "-frames:v",
            "1",
            "-f",
            "rawvideo",
            "-pix_fmt",
            "rgba",
            "pipe:1",
        ])
        .output()
        .unwrap();
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    out.stdout
}
fn pixel(data: &[u8], x: usize, y: usize) -> &[u8] {
    &data[(y * 160 + x) * 4..(y * 160 + x) * 4 + 4]
}
#[test]
fn transparent_animation_repeats_holds_and_exports_at_caption_start() -> anyhow::Result<()> {
    let root = std::env::temp_dir().join(format!("mstudio-caption-animation-{}", media::id()));
    std::fs::create_dir_all(&root)?;
    let mut frames = vec![];
    for (i, x) in [20, 100].iter().enumerate() {
        let p = root.join(format!("input-{i}.png"));
        run(Command::new(binary("ffmpeg")).args(["-v","error","-y","-f","lavfi","-i", &format!("color=black@0:s=160x120,format=rgba,drawbox=x={x}:y=40:w=20:h=20:color=white:t=fill:replace=1"),"-frames:v","1"]).arg(&p))?;
        frames.push((p, 0.5));
    }
    let video = root.join("animation.mov");
    caption_animation::encode(&frames, 30, 2., true, &root, &video)?;
    let first = frame(&video, 0.25);
    assert_eq!(pixel(&first, 0, 0)[3], 0);
    assert_eq!(pixel(&first, 25, 45), &[255, 255, 255, 255]);
    assert_eq!(pixel(&frame(&video, 0.75), 105, 45), &[255, 255, 255, 255]);
    assert_eq!(pixel(&frame(&video, 1.25), 25, 45), &[255, 255, 255, 255]);
    let hold = root.join("hold.mov");
    caption_animation::encode(&frames, 30, 2., false, &root, &hold)?;
    assert_eq!(pixel(&frame(&hold, 1.75), 105, 45), &[255, 255, 255, 255]);
    let bg = root.join("base.png");
    run(Command::new(binary("ffmpeg"))
        .args([
            "-v",
            "error",
            "-y",
            "-f",
            "lavfi",
            "-i",
            "color=red:s=160x120",
            "-frames:v",
            "1",
        ])
        .arg(&bg))?;
    let assets = vec![media::import(&bg, &root)?, media::import(&video, &root)?];
    let spec = RenderSpec {
        width: 160,
        height: 120,
        fps: 30,
        tracks: vec![Track {
            id: "v".into(),
            kind: "video".into(),
            muted: false,
            hidden: false,
        }],
        clips: vec![Clip {
            id: "clip".into(),
            asset_id: assets[0].id.clone(),
            track_id: "v".into(),
            trim_out: 4.,
            speed: 1.,
            volume: 0.,
            ..Default::default()
        }],
        captions: vec![Caption {
            start: 1.,
            end: 3.,
            text: "test".into(),
            asset_id: assets[1].id.clone(),
        }],
    };
    mstudio::preview_validate::validate(&spec, &assets, 0)?;
    let output = root.join("export.mp4");
    composition::render(&spec, &assets, &root.join("render"), &output, |_, _| {})?;
    for (t, x) in [(1.25, 25), (1.75, 105), (2.25, 25)] {
        let f = frame(&output, t);
        assert!(pixel(&f, x, 45)[1] > 220, "animated pixel missing at {t}");
        assert!(
            pixel(&f, 0, 0)[0] > 220 && pixel(&f, 0, 0)[1] < 20,
            "alpha lost at {t}"
        );
    }
    assert!(pixel(&frame(&output, 0.25), 25, 45)[1] < 20);
    assert!(pixel(&frame(&output, 3.25), 25, 45)[1] < 20);
    std::fs::remove_dir_all(root)?;
    Ok(())
}
