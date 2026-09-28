use super::*;
use mstudio::preview_ges::{Layer, Plan};
use std::{process::Command, time::Instant};
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
    let root = std::env::temp_dir().join(format!("mstudio-ges-test-{}", mstudio::media::id()));
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
    drop(p);
    let next = Player::open(plan).unwrap();
    next.control("seek", 30).unwrap();
    wait_frame(&next, 30);
    drop(next);
    std::fs::remove_dir_all(root).unwrap();
}
