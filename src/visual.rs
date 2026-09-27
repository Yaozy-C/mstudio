//! Shared filter definitions keep native preview and exported video consistent.
use serde::{Deserialize, Serialize};
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Effect {
    #[default]
    None,
    Grayscale,
    Sepia,
    Blur,
    Vignette,
}
fn one() -> f64 {
    1.
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Visual {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub grade: Option<crate::grading::Grade>,
    #[serde(default)]
    pub brightness: f64,
    #[serde(default = "one")]
    pub contrast: f64,
    #[serde(default = "one")]
    pub saturation: f64,
    #[serde(default)]
    pub temperature: f64,
    #[serde(default)]
    pub effect: Effect,
}
impl Visual {
    pub fn validate(&self) -> anyhow::Result<()> {
        if let Some(grade) = &self.grade {
            grade.validate()?;
        }
        for (v, low, high) in [
            (self.brightness, -0.5, 0.5),
            (self.contrast, 0.5, 1.5),
            (self.saturation, 0., 2.),
            (self.temperature, -1., 1.),
        ] {
            anyhow::ensure!(
                v.is_finite() && (low..=high).contains(&v),
                "调色参数超出范围"
            );
        }
        Ok(())
    }
}
pub type Filter = (&'static str, Vec<(&'static str, String)>);
pub fn filters(visual: Option<&Visual>, width: i64, height: i64) -> anyhow::Result<Vec<Filter>> {
    let Some(v) = visual else {
        return Ok(vec![]);
    };
    let mut result = Vec::new();
    if v.brightness != 0. || v.contrast != 1. || v.saturation != 1. {
        result.push((
            "eq",
            vec![
                ("brightness", v.brightness),
                ("contrast", v.contrast),
                ("saturation", v.saturation),
            ],
        ));
    }
    if v.temperature != 0. {
        result.push((
            "colorbalance",
            vec![
                ("rs", v.temperature * 0.2),
                ("bs", -v.temperature * 0.2),
                ("rm", v.temperature * 0.15),
                ("bm", -v.temperature * 0.15),
                ("pl", 1.),
            ],
        ));
    }
    match v.effect {
        Effect::None => {}
        Effect::Grayscale => result.push(("hue", vec![("s", 0.)])),
        Effect::Sepia => result.push((
            "colorchannelmixer",
            vec![
                ("rr", 0.393),
                ("rg", 0.769),
                ("rb", 0.189),
                ("gr", 0.349),
                ("gg", 0.686),
                ("gb", 0.168),
                ("br", 0.272),
                ("bg", 0.534),
                ("bb", 0.131),
            ],
        )),
        Effect::Blur => result.push((
            "boxblur",
            vec![
                (
                    "luma_radius",
                    (width.min(height) as f64 * 0.008).round().clamp(1., 20.),
                ),
                ("luma_power", 1.),
                ("chroma_radius", 1.),
                ("chroma_power", 1.),
            ],
        )),
        Effect::Vignette => result.push(("vignette", vec![("angle", std::f64::consts::PI / 5.)])),
    }
    let mut result: Vec<Filter> = result
        .into_iter()
        .map(|(n, p)| (n, p.into_iter().map(|(k, v)| (k, v.to_string())).collect()))
        .collect();
    if let Some(grade) = &v.grade {
        result.insert(
            0,
            (
                "lut3d",
                vec![
                    (
                        "file",
                        crate::grade_lut::bake(grade)?
                            .to_string_lossy()
                            .into_owned(),
                    ),
                    ("interp", "tetrahedral".into()),
                ],
            ),
        );
    }
    Ok(result)
}
pub fn ffmpeg(visual: Option<&Visual>, width: i64, height: i64) -> anyhow::Result<String> {
    Ok(filters(visual, width, height)?
        .iter()
        .map(|(name, params)| {
            format!(
                "{name}={}",
                params
                    .iter()
                    .map(|(k, v)| format!(
                        "{k}='{}'",
                        v.replace('\\', "\\\\")
                            .replace('\'', "'\\''")
                            .replace(':', "\\:")
                    ))
                    .collect::<Vec<_>>()
                    .join(":")
            )
        })
        .collect::<Vec<_>>()
        .join(","))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn defaults_and_validation() {
        let mut v: Visual = serde_json::from_str("{}").unwrap();
        assert!(ffmpeg(Some(&v), 640, 360).unwrap().is_empty());
        v.brightness = f64::NAN;
        assert!(v.validate().is_err());
        v.brightness = 0.2;
        v.effect = Effect::Grayscale;
        assert!(ffmpeg(Some(&v), 640, 360).unwrap().contains("hue=s='0'"));
    }
}
