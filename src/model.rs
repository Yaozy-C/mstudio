use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Asset {
    #[serde(default)]
    pub missing: bool,
    #[serde(default)]
    pub generated: bool,
    pub id: String,
    pub name: String,
    pub kind: String,
    pub path: String,
    pub preview: String,
    pub duration: f64,
    pub width: u32,
    pub height: u32,
    pub has_audio: bool,
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Clip {
    #[serde(default)]
    pub transition: Option<crate::transitions::Transition>,
    #[serde(default)]
    pub visual: Option<crate::visual::Visual>,
    pub id: String,
    pub asset_id: String,
    pub trim_in: f64,
    pub trim_out: f64,
    pub speed: f64,
    pub volume: f64,
    pub start: f64,
    pub track_id: String,
    pub x: Option<f64>,
    pub y: Option<f64>,
    pub scale: Option<f64>,
    pub opacity: Option<f64>,
    pub fade_in: Option<f64>,
    pub fade_out: Option<f64>,
}
impl Clip {
    pub fn duration(&self) -> f64 {
        (self.trim_out - self.trim_in) / self.speed
    }
    pub fn validate(&self, asset: &Asset) -> anyhow::Result<()> {
        if let Some(v) = &self.visual {
            v.validate()?;
        }
        anyhow::ensure!(
            self.trim_in.is_finite()
                && self.trim_out.is_finite()
                && self.speed.is_finite()
                && self.volume.is_finite(),
            "剪辑参数必须为有限数值"
        );
        anyhow::ensure!((0.25..=4.0).contains(&self.speed), "速度范围为 0.25–4 倍");
        anyhow::ensure!((0.0..=1.0).contains(&self.volume), "音量范围为 0–100%");
        anyhow::ensure!(
            self.trim_in >= 0.0 && self.trim_out > self.trim_in,
            "片段出点必须大于入点"
        );
        anyhow::ensure!(
            asset.kind == "image" || self.trim_out <= asset.duration + 0.05,
            "裁切超出源素材时长"
        );
        anyhow::ensure!(self.duration() <= 3600.0, "单片段不能超过一小时");
        for (value, min, max) in [
            (Some(self.start), 0.0, 86400.0),
            (self.x, -1.0, 2.0),
            (self.y, -1.0, 2.0),
            (self.scale, 0.05, 4.0),
            (self.opacity, 0.0, 1.0),
            (self.fade_in, 0.0, 3600.0),
            (self.fade_out, 0.0, 3600.0),
        ] {
            if let Some(v) = value {
                anyhow::ensure!(
                    v.is_finite() && (min..=max).contains(&v),
                    "时间或叠加参数超出范围"
                );
            }
        }
        Ok(())
    }
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RenderSpec {
    pub clips: Vec<Clip>,
    pub width: u32,
    pub height: u32,
    pub fps: u32,
    pub tracks: Vec<Track>,
    pub captions: Vec<Caption>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Track {
    pub id: String,
    pub kind: String,
    #[serde(default)]
    pub muted: bool,
    #[serde(default)]
    pub hidden: bool,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Caption {
    pub start: f64,
    pub end: f64,
    pub text: String,
    #[serde(default)]
    pub asset_id: String,
}
