use super::*;

fn source(id: &str, kind: &str, duration: f64) -> Source {
    (
        serde_json::from_value(serde_json::json!({
            "id": id, "name": id, "kind": kind, "duration": duration,
            "path": "", "preview": "", "width": 160, "height": 120, "hasAudio": false,
        }))
        .unwrap(),
        Reference {
            asset_id: id.into(),
            start: None,
            end: None,
        },
    )
}

#[test]
fn short_references_merge_in_order_and_images_keep_their_position() {
    let groups = plan(vec![
        source("image", "image", 0.),
        source("one", "video", 4. / 3.),
        source("other", "image", 0.),
        source("two", "video", 4. / 3.),
    ])
    .unwrap();
    let ids: Vec<Vec<_>> = groups
        .iter()
        .map(|g| g.iter().map(|s| s.0.id.as_str()).collect())
        .collect();
    assert_eq!(ids, vec![vec!["image"], vec!["one", "two"], vec!["other"]]);
    assert_eq!(
        plan(vec![source("one", "video", 2.), source("two", "video", 3.)])
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn merging_uses_trimmed_duration_and_rejects_a_total_below_two_seconds() {
    let mut trimmed = source("one", "video", 10.);
    trimmed.1.start = Some(5.);
    trimmed.1.end = Some(6.);
    assert_eq!(
        plan(vec![trimmed, source("two", "video", 1.)]).unwrap()[0].len(),
        2
    );
    assert!(plan(vec![source("one", "video", 4. / 3.)]).is_err());
    assert!(
        plan(vec![
            source("one", "video", 0.8),
            source("two", "video", 0.9)
        ])
        .is_err()
    );
}

struct Work(std::path::PathBuf);
impl Drop for Work {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.0);
    }
}

fn video(work: &Path, id: &str, color: &str, size: &str, audio: bool) -> Source {
    let path = work.join(format!("{id}.mp4"));
    let mut cmd = media::command("ffmpeg");
    cmd.args([
        "-v",
        "error",
        "-y",
        "-f",
        "lavfi",
        "-i",
        &format!("color=c={color}:s={size}:r=24:d=2"),
    ]);
    if audio {
        cmd.args([
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=440:sample_rate=44100:duration=2",
            "-c:a",
            "aac",
        ]);
    }
    media::run(
        cmd.args(["-c:v", "libx264", "-threads", "2", "-pix_fmt", "yuv420p"])
            .arg(&path),
    )
    .unwrap();
    let mut s = source(id, "video", 2.);
    s.0.path = path.to_string_lossy().into();
    s.0.has_audio = audio;
    s.1.start = Some(0.5);
    s.1.end = Some(0.5 + 4. / 3.);
    s
}

#[test]
fn encodes_two_short_clips_as_one_playable_video_with_trim_order_and_mixed_audio() {
    let work = Work(std::env::temp_dir().join(format!("reference-merge-{}", media::id())));
    std::fs::create_dir_all(&work.0).unwrap();
    for (audio, seconds) in [(false, 4. / 3.), (true, 4. / 3.), (false, 1.), (true, 1.)] {
        let mut one = video(&work.0, "one", "red", "160x120", audio);
        let mut two = video(&work.0, "two", "blue", "128x96", false);
        one.1.end = Some(0.5 + seconds);
        two.1.end = Some(0.5 + seconds);
        two.0.width = 128;
        two.0.height = 96;
        let (bytes, mime) = prepare_group(&[one, two], &work.0).unwrap();
        assert_eq!(mime, "video/mp4");
        assert!(!bytes.is_empty());
        let output = work.0.join("reference.mp4");
        let metadata = media::probe(&output).unwrap();
        let streams = metadata["streams"].as_array().unwrap();
        assert_eq!(streams.iter().any(|s| s["codec_type"] == "audio"), audio);
        let v = streams.iter().find(|s| s["codec_type"] == "video").unwrap();
        let duration = v["duration"].as_str().unwrap().parse::<f64>().unwrap();
        assert!((duration - seconds * 2.).abs() < 0.04, "{duration}");
        assert_eq!(v["width"], 160);
        assert_eq!(v["height"], 120);
        for (time, channel) in [("0.2", 0), ("1.8", 2)] {
            let frame = media::command("ffmpeg")
                .args(["-v", "error", "-ss", time, "-i"])
                .arg(&output)
                .args([
                    "-frames:v",
                    "1",
                    "-vf",
                    "scale=1:1",
                    "-pix_fmt",
                    "rgb24",
                    "-f",
                    "rawvideo",
                    "pipe:1",
                ])
                .output()
                .unwrap();
            assert!(frame.status.success());
            assert_eq!(frame.stdout.len(), 3);
            assert!(frame.stdout[channel] > 200, "{:?}", frame.stdout);
        }
    }
}
