use super::*;
use std::{
    process::Command,
    time::{Duration, Instant},
};
#[test]
fn validates_layout_stride_and_file_and_cleans_up() {
    assert!(SharedFrames::create(0, 10).is_err());
    assert!(SharedFrames::create(3841, 2160).is_err());
    let owner = SharedFrames::create(2, 2).unwrap();
    let descriptor = owner.descriptor();
    let reader = SharedFrames::open(descriptor.clone()).unwrap();
    let padded = [
        1, 2, 3, 4, 5, 6, 7, 8, 99, 99, 99, 99, 9, 10, 11, 12, 13, 14, 15, 16,
    ];
    assert!(owner.publish(1, 8, &padded[..8], 12).is_err());
    owner.publish(1, 8, &padded, 12).unwrap();
    reader
        .with_frame(Some(1), |bytes| {
            assert_eq!(
                &bytes[16..],
                &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
            );
            assert_eq!(u32::from_le_bytes(bytes[12..16].try_into().unwrap()), 8);
        })
        .unwrap();
    assert!(reader.with_frame(Some(2), |_| ()).is_none());
    drop(owner);
    assert!(!descriptor.path.exists());
    // An already mapped peer remains valid after unlinking.
    assert!(reader.with_frame(Some(1), |_| ()).is_some());
}
#[test]
fn child_writer() {
    let Ok(path) = std::env::var("MSTUDIO_TEST_FRAME_MAPPING") else {
        return;
    };
    let frames = SharedFrames::open(Descriptor {
        path: path.into(),
        width: 64,
        height: 32,
    })
    .unwrap();
    for seq in 1..=1000 {
        let data = vec![(seq % 251) as u8; 64 * 32 * 4];
        assert!(frames.publish(seq, seq * 2, &data, 64 * 4).unwrap());
        std::thread::sleep(Duration::from_micros(50));
    }
}
#[test]
fn separate_process_upload_never_tears_a_pinned_frame() {
    let owner = SharedFrames::create(64, 32).unwrap();
    let mut child = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "preview_shared::tests::child_writer",
            "--nocapture",
        ])
        .env("MSTUDIO_TEST_FRAME_MAPPING", &owner.descriptor.path)
        .stdout(std::process::Stdio::null())
        .spawn()
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(10);
    let mut seen = 0;
    while seen < 1000 && Instant::now() < deadline {
        owner.with_frame(None, |bytes| {
            let seq = u32::from_le_bytes(bytes[..4].try_into().unwrap());
            std::thread::sleep(Duration::from_micros(100));
            assert!(bytes[16..].iter().all(|byte| *byte == (seq % 251) as u8));
            assert_eq!(
                u32::from_le_bytes(bytes[12..16].try_into().unwrap()),
                seq * 2
            );
            seen = seq;
        });
    }
    if seen != 1000 {
        let _ = child.kill();
    }
    assert!(child.wait().unwrap().success());
    assert_eq!(seen, 1000);
}
