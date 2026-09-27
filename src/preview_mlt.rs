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
        if t.kind == "video" {
            for (name, params) in crate::visual::filters(c.visual.as_ref(), w, h)? {
                out += &format!(
                    "<filter in=\"{trim}\" out=\"{end}\">{}",
                    prop("mlt_service", format!("avfilter.{name}"))
                );
                for (key, value) in params {
                    out += &prop(&format!("av.{key}"), value);
                }
                out += "</filter>";
            }
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
        // PNG has one decoded frame. Repeat that frame instead of seeking the
        // decoder past EOF, which can yield an opaque white fallback image.
        out +=
            &format!("<entry producer=\"p{id}\" in=\"0\" out=\"0\" repeat=\"{len}\"/></playlist>");
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
#[path = "preview_mlt_tests.rs"]
mod tests;
