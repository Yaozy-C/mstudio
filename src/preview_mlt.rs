//! Translate the saved edit into one MLT graph. No independent per-clip clocks.
use crate::model::{Asset, RenderSpec};
use anyhow::{Context, Result, ensure};
use std::collections::HashMap;

fn xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
fn prop(name: &str, value: impl std::fmt::Display) -> String {
    format!(
        "<property name=\"{name}\">{}</property>",
        xml(&value.to_string())
    )
}
pub fn graph(spec: &RenderSpec, assets: &[Asset]) -> Result<String> {
    ensure!([24, 25, 30, 60].contains(&spec.fps), "不支持的预览帧率");
    ensure!(
        (64..=3840).contains(&spec.width) && (64..=3840).contains(&spec.height),
        "预览尺寸无效"
    );
    let assets: HashMap<_, _> = assets.iter().map(|a| (a.id.as_str(), a)).collect();
    let tracks: HashMap<_, _> = spec
        .tracks
        .iter()
        .enumerate()
        .map(|(i, t)| (t.id.as_str(), (i, t)))
        .collect();
    ensure!(tracks.len() == spec.tracks.len(), "轨道 ID 重复");
    let mut ordered = Vec::new();
    let mut total: f64 = 0.;
    for (i, c) in spec.clips.iter().enumerate() {
        let a = assets.get(c.asset_id.as_str()).context("预览素材不存在")?;
        ensure!(!a.missing, "预览素材缺失：{}", a.name);
        c.validate(a)?;
        let (order, t) = tracks.get(c.track_id.as_str()).context("预览轨道不存在")?;
        ensure!(t.kind == "video" || t.kind == "audio", "轨道类型无效");
        total = total.max(c.start + c.duration());
        ordered.push((*order, i, c, *a, *t));
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
                .is_some_and(|a| a.kind == "image" && !a.missing),
            "字幕画面尚未准备"
        );
        total = total.max(c.end);
    }
    ensure!(total > 0. && total <= 86400., "预览时间线为空或过长");
    ordered.sort_by_key(|(order, i, ..)| (*order, *i));
    let f = spec.fps as f64;
    let frames = (total * f).ceil() as i64;
    let last = frames - 1;
    // Keep the project's aspect ratio; output is a preview, not an export.
    let ratio = 640. / spec.width.max(spec.height) as f64;
    let w = ((spec.width as f64 * ratio / 2.).round() as i64 * 2).max(2);
    let h = ((spec.height as f64 * ratio / 2.).round() as i64 * 2).max(2);
    let mut out = format!(
        "<mlt LC_NUMERIC=\"C\" producer=\"timeline\"><profile frame_rate_num=\"{}\" frame_rate_den=\"1\" width=\"{w}\" height=\"{h}\" progressive=\"1\" sample_aspect_num=\"1\" sample_aspect_den=\"1\" display_aspect_num=\"{}\" display_aspect_den=\"{}\" colorspace=\"709\"/>",
        spec.fps, spec.width, spec.height
    );
    out += &format!(
        "<producer id=\"black\" in=\"0\" out=\"{last}\">{}{}{}</producer>",
        prop("mlt_service", "color"),
        prop("resource", "black"),
        prop("length", frames)
    );
    let mut track_xml = "<track producer=\"black\"/>".to_string();
    let mut transitions = String::new();
    for (n, (_, _, c, a, t)) in ordered.iter().enumerate() {
        let id = n + 1;
        let start = (c.start * f).round() as i64;
        let len = (c.duration() * f).round().max(1.) as i64;
        let trim = (c.trim_in / c.speed * f).round() as i64;
        let end = trim + len - 1;
        out += &format!("<producer id=\"p{id}\" in=\"0\" out=\"{end}\">");
        let warped = a.kind != "image" && (c.speed - 1.).abs() > 0.00001;
        out += &prop("mlt_service", if warped { "timewarp" } else { "avformat" });
        out += &prop(
            "resource",
            if warped {
                format!("{}:{}", c.speed, a.path)
            } else {
                a.path.clone()
            },
        );
        if warped {
            out += &prop("warp_speed", c.speed);
            out += &prop("warp_resource", &a.path);
        }
        out += &prop("eof", "pause");
        out += &prop("length", end + 1);
        out += &prop("threads", 2);
        // A zero-gain stream must be excluded, not left as an unmixed silent input.
        let audible = a.has_audio && !t.muted && c.volume > 0.;
        if !audible {
            out += &prop("audio_index", -1);
        }
        if t.kind == "audio" || t.hidden {
            out += &prop("video_index", -1);
        }
        let fade_in = c.fade_in.unwrap_or(0.);
        let fade_out = c.fade_out.unwrap_or(0.);
        let mut gain = c.volume.to_string();
        if fade_in > 0. {
            gain = format!("({gain})*min(1,max(0,t/{fade_in}))");
        }
        if fade_out > 0. {
            gain = format!("({gain})*min(1,max(0,({}-t)/{fade_out}))", c.duration());
        }
        out += &format!(
            "<filter in=\"{trim}\" out=\"{end}\">{}{}{}{}</filter></producer>",
            prop("mlt_service", "avfilter.volume"),
            prop("av.volume", gain),
            prop("av.eval", "frame"),
            prop("position", "filter")
        );
        out += &format!("<playlist id=\"list{id}\">");
        if start > 0 {
            out += &format!("<blank length=\"{start}\"/>");
        }
        out += &format!("<entry producer=\"p{id}\" in=\"{trim}\" out=\"{end}\"/></playlist>");
        let hide = if t.kind == "audio" || t.hidden {
            if audible { "video" } else { "both" }
        } else if !audible {
            "audio"
        } else {
            "none"
        };
        track_xml += &format!("<track producer=\"list{id}\" hide=\"{hide}\"/>");
        if audible {
            transitions += &format!(
                "<transition>{}{}{}{}{}</transition>",
                prop("mlt_service", "mix"),
                prop("a_track", 0),
                prop("b_track", id),
                prop("always_active", 1),
                prop("sum", 1)
            );
        }
        if t.kind == "video" && !t.hidden && a.kind != "audio" {
            let scale = c.scale.unwrap_or(1.);
            let rect = format!(
                "{} {} {} {} {}",
                (c.x.unwrap_or(0.5) - scale / 2.) * w as f64,
                (c.y.unwrap_or(0.5) - scale / 2.) * h as f64,
                scale * w as f64,
                scale * h as f64,
                c.opacity.unwrap_or(1.)
            );
            transitions += &affine(id, &rect);
        }
    }
    for (i, c) in spec.captions.iter().enumerate() {
        let id = ordered.len() + i + 1;
        let len = ((c.end - c.start) * f).round().max(1.) as i64;
        let start = (c.start * f).round() as i64;
        out += &format!(
            "<producer id=\"p{id}\" out=\"{}\">{}{}{}{}</producer><playlist id=\"list{id}\">",
            len - 1,
            prop("mlt_service", "avformat"),
            prop("resource", &assets[c.asset_id.as_str()].path),
            prop("eof", "pause"),
            prop("length", len)
        );
        if start > 0 {
            out += &format!("<blank length=\"{start}\"/>");
        }
        out += &format!(
            "<entry producer=\"p{id}\" in=\"0\" out=\"{}\"/></playlist>",
            len - 1
        );
        track_xml += &format!("<track producer=\"list{id}\" hide=\"audio\"/>");
        transitions += &affine(id, &format!("0 0 {w} {h} 1"));
    }
    out += &format!(
        "<tractor id=\"timeline\" in=\"0\" out=\"{last}\">{track_xml}{transitions}</tractor></mlt>"
    );
    Ok(out)
}
fn affine(track: usize, rect: &str) -> String {
    format!(
        "<transition>{}{}{}{}{}{}{}</transition>",
        prop("mlt_service", "affine"),
        prop("a_track", 0),
        prop("b_track", track),
        prop("always_active", 1),
        prop("distort", 0),
        prop("fill", 1),
        prop("rect", rect)
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::{Clip, Track};
    fn fixture() -> (RenderSpec, Vec<Asset>) {
        (
            RenderSpec {
                width: 1080,
                height: 1920,
                fps: 30,
                tracks: vec![Track {
                    id: "v1".into(),
                    kind: "video".into(),
                    muted: false,
                    hidden: false,
                }],
                clips: vec![Clip {
                    id: "c".into(),
                    asset_id: "a".into(),
                    trim_in: 2.,
                    trim_out: 6.,
                    speed: 2.,
                    volume: 0.5,
                    start: 1.,
                    track_id: "v1".into(),
                    ..Default::default()
                }],
                captions: vec![],
            },
            vec![Asset {
                id: "a".into(),
                name: "test".into(),
                kind: "video".into(),
                path: "/tmp/a&b\".mp4".into(),
                preview: String::new(),
                duration: 10.,
                width: 480,
                height: 832,
                has_audio: true,
                missing: false,
                generated: false,
            }],
        )
    }
    #[test]
    fn cuts_speed_and_gaps_use_project_frames() {
        let (s, a) = fixture();
        let text = graph(&s, &a).unwrap();
        assert!(text.contains("<blank length=\"30\"/>"));
        assert!(text.contains("<entry producer=\"p1\" in=\"30\" out=\"89\"/>"));
        assert!(text.contains("2:/tmp/a&amp;b&quot;.mp4"));
        assert!(text.contains("id=\"timeline\" in=\"0\" out=\"89\""));
        assert!(text.contains("name=\"sum\">1"));
    }
    #[test]
    fn hidden_video_retains_audio_but_muted_video_remains_visible() {
        let (mut s, a) = fixture();
        s.tracks[0].hidden = true;
        let text = graph(&s, &a).unwrap();
        assert!(text.contains("hide=\"video\""));
        assert!(!text.contains("name=\"mlt_service\">affine"));
        assert!(text.contains("name=\"mlt_service\">mix"));
        s.tracks[0].hidden = false;
        s.tracks[0].muted = true;
        let text = graph(&s, &a).unwrap();
        assert!(text.contains("hide=\"audio\""));
        assert!(text.contains("name=\"mlt_service\">affine"));
        assert!(!text.contains("name=\"mlt_service\">mix"));
    }
    #[test]
    fn zero_gain_does_not_override_other_audio_tracks() {
        let (mut s, a) = fixture();
        s.clips[0].volume = 0.;
        let text = graph(&s, &a).unwrap();
        assert!(text.contains("hide=\"audio\""));
        assert!(text.contains("name=\"mlt_service\">affine"));
        s.tracks[0].kind = "audio".into();
        let text = graph(&s, &a).unwrap();
        assert!(text.contains("hide=\"both\""));
        assert!(text.contains("name=\"audio_index\">-1"));
        assert!(!text.contains("name=\"mlt_service\">mix"));
    }
    #[test]
    fn invalid_or_missing_media_fails_explicitly() {
        let (mut s, mut a) = fixture();
        a[0].missing = true;
        assert!(graph(&s, &a).is_err());
        a[0].missing = false;
        s.clips[0].speed = f64::NAN;
        assert!(graph(&s, &a).is_err());
    }
}
