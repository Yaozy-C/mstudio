//! Explicit clip placement, layered video and independently mixed audio.
use crate::{
    media::{binary, run},
    model::{Asset, RenderSpec},
};
use anyhow::{Context, Result, ensure};
use std::{
    collections::{HashMap, HashSet},
    path::Path,
    process::Command,
};

pub fn render(
    spec: &RenderSpec,
    assets: &[Asset],
    work: &Path,
    output: &Path,
    mut progress: impl FnMut(usize, usize),
) -> Result<()> {
    if spec.clips.iter().any(|c| c.transition.is_some()) {
        let mut prepared = spec.clone();
        let mut assets = assets.to_vec();
        crate::transitions::prepare(&mut prepared, &mut assets, &work.join("transitions"), false)?;
        return render(&prepared, &assets, work, output, progress);
    }
    ensure!(!spec.clips.is_empty(), "时间线没有片段");
    ensure!([24, 25, 30, 60].contains(&spec.fps), "不支持的帧率");
    ensure!(
        (64..=3840).contains(&spec.width)
            && (64..=3840).contains(&spec.height)
            && spec.width.is_multiple_of(2)
            && spec.height.is_multiple_of(2),
        "输出尺寸无效"
    );
    let assets: HashMap<_, _> = assets.iter().map(|a| (a.id.as_str(), a)).collect();
    let tracks: HashMap<_, _> = spec
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.as_str(), (i, t)))
        .collect();
    ensure!(
        tracks.len() == spec.tracks.len()
            && tracks
                .values()
                .all(|(_, t)| ["video", "audio"].contains(&t.kind.as_str())),
        "轨道无效或重复"
    );
    let mut ids = HashSet::new();
    let mut total: f64 = 0.;
    for c in &spec.clips {
        let asset = assets.get(c.asset_id.as_str()).context("时间线素材丢失")?;
        c.validate(asset)?;
        ensure!(ids.insert(&c.id), "片段 ID 重复");
        let (_, track) = tracks.get(c.track_id.as_str()).context("片段轨道丢失")?;
        ensure!(
            if track.kind == "video" {
                asset.kind != "audio"
            } else {
                asset.has_audio
            },
            "素材与轨道类型不符"
        );
        total = total.max(c.start + c.duration());
    }
    for c in &spec.captions {
        ensure!(
            c.start.is_finite()
                && c.end.is_finite()
                && c.start >= 0.
                && c.end > c.start
                && c.end <= 86400.,
            "字幕时间无效"
        );
        ensure!(
            assets
                .get(c.asset_id.as_str())
                .is_some_and(|a| a.kind == "image"),
            "字幕画面尚未准备"
        );
        total = total.max(c.end);
    }
    ensure!(total <= 86400., "成片过长");
    std::fs::create_dir_all(work)?;
    let mut cmd = Command::new(binary("ffmpeg"));
    cmd.args(["-v", "error", "-y", "-filter_complex_threads", "2"]);
    let mut graph = vec![format!(
        "color=c=black:s={}x{}:r={}:d={total},format=yuv420p[base]",
        spec.width, spec.height, spec.fps
    )];
    let mut audio = Vec::new();
    let mut layers = Vec::new();
    for (i, c) in spec.clips.iter().enumerate() {
        let a = assets[c.asset_id.as_str()];
        let (order, t) = tracks[c.track_id.as_str()];
        cmd.args(["-threads", "2"]);
        if a.kind == "image" {
            cmd.args(["-loop", "1", "-framerate", &spec.fps.to_string()]);
        }
        cmd.args([
            "-ss",
            &c.trim_in.to_string(),
            "-t",
            &(c.trim_out - c.trim_in).to_string(),
            "-i",
            &a.path,
        ]);
        let start = c.start;
        let end = start + c.duration();
        if t.kind == "video" && !t.hidden {
            let scale = c.scale.unwrap_or(1.);
            let w = ((spec.width as f64 * scale / 2.).round() as u32 * 2).max(2);
            let h = ((spec.height as f64 * scale / 2.).round() as u32 * 2).max(2);
            let filters = crate::visual::ffmpeg(c.visual.as_ref(), w as i64, h as i64);
            let filters = if filters.is_empty() {
                filters
            } else {
                format!("{filters},")
            };
            graph.push(format!("[{i}:v]setpts=(PTS-STARTPTS)/{},fps={},scale={w}:{h}:force_original_aspect_ratio=decrease,setsar=1,{filters}format=rgba,colorchannelmixer=aa={},setpts=PTS+{start}/TB[v{i}]", c.speed,spec.fps,c.opacity.unwrap_or(1.)));
            layers.push((order,i,format!("overlay=x=W*{}-w/2:y=H*{}-h/2:eof_action=pass:repeatlast=0:enable='gte(t,{start})*lt(t,{end})'",c.x.unwrap_or(0.5),c.y.unwrap_or(0.5))));
        }
        if a.has_audio && !t.muted && c.volume > 0. {
            graph.push(crate::audio_mix::clip_filter(c, i, &format!("a{i}")));
            audio.push(format!("[a{i}]"));
        }
    }
    layers.sort_by_key(|(order, i, _)| (*order, *i));
    let mut current = "base".to_string();
    for (_, i, filter) in layers {
        let next = format!("layer{i}");
        graph.push(format!("[{current}][v{i}]{filter}[{next}]"));
        current = next;
    }
    for (j, c) in spec.captions.iter().enumerate() {
        let i = spec.clips.len() + j;
        cmd.args([
            "-threads",
            "1",
            "-loop",
            "1",
            "-framerate",
            &spec.fps.to_string(),
            "-i",
            &assets[c.asset_id.as_str()].path,
        ]);
        graph.push(format!(
            "[{i}:v]scale={}:{},format=rgba[caption{j}]",
            spec.width, spec.height
        ));
        let next = format!("sub{j}");
        graph.push(format!(
            "[{current}][caption{j}]overlay=enable='gte(t,{})*lt(t,{})':shortest=0[{next}]",
            c.start, c.end
        ));
        current = next;
    }
    if audio.is_empty() {
        graph.push("anullsrc=r=48000:cl=stereo[mix]".into());
    } else {
        graph.push(format!(
            "{}amix=inputs={}:duration=longest:normalize=0,alimiter=limit=0.95:latency=1,apad[mix]",
            audio.join(""),
            audio.len()
        ));
    }
    let temp = work.join("composition.mp4");
    progress(0, 1);
    cmd.args([
        "-filter_complex",
        &graph.join(";"),
        "-map",
        &format!("[{current}]"),
        "-map",
        "[mix]",
        "-t",
        &total.to_string(),
        "-r",
        &spec.fps.to_string(),
        "-c:v",
        "libx264",
        "-threads",
        "2",
        "-preset",
        "fast",
        "-crf",
        "20",
        "-pix_fmt",
        "yuv420p",
        "-c:a",
        "aac",
        "-ar",
        "48000",
        "-ac",
        "2",
        "-movflags",
        "+faststart",
    ])
    .arg(&temp);
    run(&mut cmd)?;
    std::fs::rename(temp, output)?;
    progress(1, 1);
    Ok(())
}
