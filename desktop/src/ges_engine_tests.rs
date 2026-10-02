use super::*;
use mstudio::preview_ges::{Layer, Plan};
use std::{process::Command, time::Instant};
static PLAYER_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());
#[cfg(windows)]
fn isolated(name: &str) -> bool {
    if std::env::var_os("MSTUDIO_GES_TEST_ROOT").is_some() {
        return false;
    }
    // Match the application's preview-worker lifetime. The Windows SDK can
    // retain media handles beyond pipeline disposal; process exit releases them.
    let root = std::env::temp_dir().join(format!("mstudio-ges-{}", mstudio::media::id()));
    let status = Command::new(std::env::current_exe().unwrap())
        .args(["--exact", name, "--nocapture"])
        .env("MSTUDIO_GES_TEST_ROOT", &root)
        .status()
        .unwrap();
    // Check cleanup in the parent, after the real worker has exited.
    std::fs::remove_dir_all(&root).unwrap();
    assert!(status.success(), "GES worker test failed: {status}");
    true
}
fn test_root(prefix: &str) -> std::path::PathBuf {
    #[cfg(windows)]
    if let Some(root) = std::env::var_os("MSTUDIO_GES_TEST_ROOT") {
        return root.into();
    }
    std::env::temp_dir().join(format!("{prefix}-{}", mstudio::media::id()))
}
fn wait_frame(p: &Player, target: u32) -> Vec<u8> {
    let deadline = Instant::now() + Duration::from_secs(12);
    while Instant::now() < deadline {
        let f = p.frame(0);
        if f.len() > 16 && u32::from_le_bytes(f[12..16].try_into().unwrap()) == target {
            return f;
        }
        p.control("status", 0).unwrap();
        std::thread::sleep(Duration::from_millis(10));
    }
    panic!("GES frame {target} timed out")
}
#[test]
fn ges_composites_seeks_replays_and_closes() {
    let _guard = PLAYER_TEST_LOCK.lock().unwrap();
    #[cfg(windows)]
    if isolated("ges_engine::tests::ges_composites_seeks_replays_and_closes") {
        return;
    }
    let root = test_root("mstudio-ges-test");
    std::fs::create_dir_all(&root).unwrap();
    let red = root.join("red.mp4");
    let png = root.join("caption.png");
    for (path, source, extra) in [
        (
            &red,
            "color=red:s=160x90:r=30:d=2",
            vec!["-c:v", "libx264", "-pix_fmt", "yuv420p"],
        ),
        (
            &png,
            "color=black@0:s=160x90,format=rgba,drawbox=x=60:y=30:w=40:h=30:color=white:t=fill:replace=1",
            vec!["-frames:v", "1"],
        ),
    ] {
        assert!(
            Command::new(mstudio::media::binary("ffmpeg"))
                .args(["-v", "error", "-y", "-f", "lavfi", "-i", source])
                .args(extra)
                .arg(path)
                .status()
                .unwrap()
                .success()
        );
    }
    let plan = Plan {
        width: 160,
        height: 90,
        fps: 30,
        duration: 2.,
        audio: None,
        files: vec![],
        layers: vec![
            Layer {
                path: red.to_string_lossy().into(),
                start: 0.,
                duration: 2.,
                trim: 0.,
                x: 0,
                y: 0,
                width: 160,
                height: 90,
                opacity: 1.,
            },
            Layer {
                // A transition overlay starting between output frames, as
                // produced by retimed clips (for example 12.69 s at 30 fps).
                path: red.to_string_lossy().into(),
                start: 0.69,
                duration: 0.5,
                trim: 0.,
                x: 0,
                y: 0,
                width: 160,
                height: 90,
                opacity: 1.,
            },
            Layer {
                path: png.to_string_lossy().into(),
                start: 0.5,
                duration: 1.,
                trim: 0.,
                x: 0,
                y: 0,
                width: 160,
                height: 90,
                opacity: 1.,
            },
        ],
    };
    let p = Player::open(plan.clone()).unwrap();
    let first = wait_frame(&p, 0);
    assert!(first[16] > 180 && first[17] < 40);
    p.control("seek", 30).unwrap();
    let frame = wait_frame(&p, 30);
    let center = 16 + (45 * 160 + 80) * 4;
    assert!(
        frame[center] > 180 && frame[center + 1] > 180,
        "caption missing"
    );
    assert!(
        frame[16] > 180 && frame[17] < 40,
        "caption transparency lost"
    );
    for target in [5, 40, 10, 50] {
        p.control("seek", target).unwrap();
        wait_frame(&p, target as u32);
    }
    let before = p.control("status", 0).unwrap().frame;
    std::thread::sleep(Duration::from_millis(120));
    assert_eq!(p.control("status", 0).unwrap().frame, before);
    assert!(p.control("rate", 0).is_err());
    assert!(p.control("rate", 9).is_err());
    // A seek must retain the selected preview rate, including fractional rates.
    for (step, minimum, maximum) in [(8, 18, 40), (1, 1, 8)] {
        p.control("rate", step).unwrap();
        p.control("seek", 0).unwrap();
        p.control("play", 0).unwrap();
        std::thread::sleep(Duration::from_millis(450));
        let position = p.control("pause", 0).unwrap().frame;
        assert!(
            (minimum..=maximum).contains(&position),
            "step {step}: frame {position}"
        );
    }
    p.control("seek", 0).unwrap();
    p.control("play", 0).unwrap();
    let changed = p.control("rate", 6).unwrap();
    assert!(changed.playing);
    std::thread::sleep(Duration::from_millis(200));
    let advanced = p.control("pause", 0).unwrap();
    assert!(advanced.frame > changed.frame && advanced.frame < 30);
    p.control("rate", 4).unwrap();
    for _ in 0..2 {
        p.control("seek", 0).unwrap();
        p.control("play", 0).unwrap();
        let start = Instant::now();
        loop {
            let s = p.control("status", 0).unwrap();
            if !s.playing && s.frame == 59 {
                break;
            }
            assert!(start.elapsed() < Duration::from_secs(5));
            std::thread::sleep(Duration::from_millis(20));
        }
    }
    // No sleeps between commands: replies must acknowledge completed seeks,
    // even when crossing layer boundaries immediately after a play/pause.
    for _ in 0..4 {
        for target in [14, 16, 20, 21, 35, 36, 44, 46, 0, 59] {
            p.control("pause", 0).unwrap();
            p.control("seek", target).unwrap();
            let frame = p.frame(0);
            assert_eq!(
                u32::from_le_bytes(frame[12..16].try_into().unwrap()),
                target as u32
            );
            assert!(frame[16] > 180 && frame[17] < 40, "black frame at {target}");
            p.control("play", 0).unwrap();
        }
    }
    drop(p);
    let next = Player::open(plan).unwrap();
    next.control("seek", 30).unwrap();
    wait_frame(&next, 30);
    drop(next);
    #[cfg(not(windows))]
    std::fs::remove_dir_all(root).unwrap();
}

#[test]
fn ges_seeks_transparent_animated_captions() {
    let _guard = PLAYER_TEST_LOCK.lock().unwrap();
    #[cfg(windows)]
    if isolated("ges_engine::tests::ges_seeks_transparent_animated_captions") {
        return;
    }
    let root = test_root("mstudio-ges-caption");
    std::fs::create_dir_all(&root).unwrap();
    let mut frames = vec![];
    for (i, x) in [20, 100].iter().enumerate() {
        let path = root.join(format!("input-{i}.png"));
        mstudio::media::run(Command::new(mstudio::media::binary("ffmpeg")).args(["-v","error","-y","-f","lavfi","-i", &format!("color=black@0:s=1080x1920,format=rgba,drawbox=x={x}:y=30:w=20:h=20:color=white:t=fill:replace=1"),"-frames:v","1"]).arg(&path)).unwrap();
        frames.push((path, 0.5));
    }
    let video = root.join("caption.mov");
    mstudio::caption_animation::encode(&frames, 30, 2., true, &root, &video).unwrap();
    let red = root.join("red.png");
    mstudio::media::run(
        Command::new(mstudio::media::binary("ffmpeg"))
            .args([
                "-v",
                "error",
                "-y",
                "-f",
                "lavfi",
                "-i",
                "color=red:s=1080x1920",
                "-frames:v",
                "1",
            ])
            .arg(&red),
    )
    .unwrap();
    let layer = |path: &std::path::Path, start, duration| Layer {
        path: path.to_string_lossy().into(),
        start,
        duration,
        trim: 0.,
        x: 0,
        y: 0,
        width: 1080,
        height: 1920,
        opacity: 1.,
    };
    let player = Player::open(Plan {
        width: 1080,
        height: 1920,
        fps: 30,
        duration: 4.,
        audio: None,
        files: vec![],
        layers: vec![layer(&red, 0., 4.), layer(&video, 1., 2.)],
    })
    .unwrap();
    wait_frame(&player, 0);
    for (target, x, white) in [
        (38, 25, true),
        (53, 105, true),
        (68, 25, true),
        (8, 25, false),
        (98, 25, false),
        (38, 25, true),
    ] {
        player.control("seek", target).unwrap();
        let data = wait_frame(&player, target as u32);
        let at = 16 + (40 * 1080 + x) * 4;
        assert!(
            data[16] > 220 && data[17] < 30,
            "transparent layer blacked out the background"
        );
        assert_eq!(
            data[at + 1] > 220,
            white,
            "caption state incorrect at {target}"
        );
    }
    player.control("seek", 0).unwrap();
    player.control("play", 0).unwrap();
    let deadline = Instant::now() + Duration::from_secs(6);
    while Instant::now() < deadline {
        if player.control("status", 0).unwrap().frame >= 100 {
            break;
        }
        std::thread::sleep(Duration::from_millis(30));
    }
    assert!(player.control("status", 0).unwrap().frame >= 100);
    drop(player);
    #[cfg(not(windows))]
    std::fs::remove_dir_all(root).unwrap();
}
