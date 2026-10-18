//! Visible source endpoint, independent of a longer container/audio duration.
use anyhow::{Context, Result, ensure};
use serde_json::Value;

fn number(value: &Value) -> Option<f64> {
    value.as_f64().or_else(|| value.as_str()?.parse().ok())
}

pub fn video_end(metadata: &Value) -> Result<f64> {
    let video = metadata["streams"]
        .as_array()
        .and_then(|streams| {
            streams
                .iter()
                .find(|s| s["codec_type"] == "video" && s["disposition"]["attached_pic"] != 1)
        })
        .context("素材没有可读取的画面轨道")?;
    let duration = number(&video["duration"])
        .or_else(|| {
            let (n, d) = video["time_base"].as_str()?.split_once('/')?;
            Some(number(&video["duration_ts"])? * n.parse::<f64>().ok()? / d.parse::<f64>().ok()?)
        })
        .context("无法确定有效画面时长，未裁切素材")?;
    // Input seeks use time relative to the container start, not absolute PTS.
    let end = number(&video["start_time"]).unwrap_or(0.) + duration
        - number(&metadata["format"]["start_time"]).unwrap_or(0.);
    ensure!(
        duration.is_finite() && duration > 0. && end.is_finite() && end > 0.,
        "有效画面时长无效"
    );
    Ok(end)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn uses_picture_duration_and_ignores_audio_tail_and_cover_art() {
        let metadata = json!({"format":{"duration":"5.184","start_time":"0"},"streams":[
            {"codec_type":"video","duration":"99","disposition":{"attached_pic":1}},
            {"codec_type":"audio","duration":"5.184"},
            {"codec_type":"video","duration":"5.166666666666667","start_time":"0"}
        ]});
        assert!((video_end(&metadata).unwrap() - 124. / 24.).abs() < 1e-9);
    }

    #[test]
    fn respects_pts_origin_and_refuses_to_guess_from_container_duration() {
        let metadata = json!({"format":{"start_time":"10"},"streams":[
            {"codec_type":"video","duration_ts":124000,"time_base":"1/24000","start_time":"10.1"}
        ]});
        assert!((video_end(&metadata).unwrap() - (124. / 24. + 0.1)).abs() < 1e-9);
        for duration in [json!("N/A"), json!("NaN"), json!(-1)] {
            assert!(
                video_end(&json!({"format":{"duration":"8"},"streams":[
                    {"codec_type":"video","duration":duration}
                ]}))
                .is_err()
            );
        }
    }
}
