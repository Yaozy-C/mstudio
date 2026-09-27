//! A small compositing recipe, compiled to xfade expressions; no executable user strings.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
fn one() -> f64 {
    1.
}
fn center() -> [f64; 2] {
    [0.5, 0.5]
}
fn curve() -> Vec<[f64; 2]> {
    vec![[0., 0.], [1., 1.]]
}
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum Mask {
    #[default]
    Uniform,
    Linear,
    Radial,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Design {
    pub mask: Mask,
    pub angle: f64,
    pub center: [f64; 2],
    pub feather: f64,
    /// Ordered time/progress points. Both endpoints are required.
    pub curve: Vec<[f64; 2]>,
    pub outgoing_zoom: f64,
    pub incoming_zoom: f64,
    pub outgoing_offset: [f64; 2],
    pub incoming_offset: [f64; 2],
}
impl Default for Design {
    fn default() -> Self {
        Self {
            mask: Mask::Uniform,
            angle: 0.,
            center: center(),
            feather: 0.1,
            curve: curve(),
            outgoing_zoom: one(),
            incoming_zoom: one(),
            outgoing_offset: [0.; 2],
            incoming_offset: [0.; 2],
        }
    }
}
impl Design {
    pub fn validate(&self) -> Result<()> {
        let range = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
        ensure!(
            range(self.angle, -180., 180.)
                && range(self.feather, 0.001, 1.)
                && self.center.iter().all(|&v| range(v, 0., 1.))
                && [self.outgoing_zoom, self.incoming_zoom]
                    .into_iter()
                    .all(|v| range(v, 1., 4.))
                && self
                    .outgoing_offset
                    .iter()
                    .chain(&self.incoming_offset)
                    .all(|&v| range(v, -1., 1.)),
            "自定义转场数值超出范围"
        );
        ensure!(
            (2..=8).contains(&self.curve.len())
                && self.curve.first() == Some(&[0., 0.])
                && self.curve.last() == Some(&[1., 1.])
                && self.curve.iter().flatten().all(|&v| range(v, 0., 1.))
                && self
                    .curve
                    .windows(2)
                    .all(|w| w[1][0] > w[0][0] && w[1][1] >= w[0][1]),
            "进度曲线须含 2–8 个递增点，从 [0,0] 到 [1,1]"
        );
        Ok(())
    }
    pub fn expression(&self) -> Result<String> {
        self.validate()?;
        // FFmpeg xfade's P descends from 1 to 0; recipe time ascends from 0 to 1.
        let t = "(1-P)";
        let progress = self.progress(t);
        let p = format!("({progress})");
        let distance = match self.mask {
            Mask::Uniform => None,
            Mask::Linear => {
                let (sin, cos) = self.angle.to_radians().sin_cos();
                Some(format!(
                    "((X/W-0.5)*{cos}+(Y/H-0.5)*{sin})/{}+0.5",
                    cos.abs() + sin.abs()
                ))
            }
            Mask::Radial => {
                let [cx, cy] = self.center;
                Some(format!(
                    "hypot(X/W-{cx},Y/H-{cy})/{}",
                    cx.max(1. - cx).hypot(cy.max(1. - cy))
                ))
            }
        };
        let blend = distance.map_or(p.clone(), |distance| {
            format!(
                "clip(({p}*{}-({distance}))/{},0,1)",
                1. + self.feather,
                self.feather
            )
        });
        Ok(format!("A*(1-({blend}))+B*({blend})"))
    }
    fn progress(&self, t: &str) -> String {
        let mut progress = "1".to_owned();
        for pair in self.curve.windows(2).rev() {
            let [a, b] = [pair[0], pair[1]];
            let segment = format!(
                "({}+({}-{})*({t}-{})/({}-{}))",
                a[1], b[1], a[1], a[0], b[0], a[0]
            );
            progress = format!("if(lte({t},{}),{segment},{progress})", b[0]);
        }
        progress
    }
    /// Perspective evaluates geometry once per frame, with native bilinear sampling.
    /// Doing the same geometry in xfade's per-pixel interpreter is much slower.
    pub fn motion_filter(&self, incoming: bool, duration: f64, fps: u32) -> Result<String> {
        self.validate()?;
        let (zoom, offset) = if incoming {
            (self.incoming_zoom, self.incoming_offset)
        } else {
            (self.outgoing_zoom, self.outgoing_offset)
        };
        if zoom == 1. && offset == [0.; 2] {
            return Ok(String::new());
        }
        // Perspective numbers its first output frame 1; xfade starts at time 0.
        let t = self.progress(&format!("clip((on-1)/{},0,1)", duration * fps as f64));
        let t = if incoming {
            format!("(1-({t}))")
        } else {
            format!("({t})")
        };
        let z = format!("(1+{}*{t})", zoom - 1.);
        let mut parameters = Vec::new();
        for (corner, (x, y)) in [(0., 0.), (1., 0.), (0., 1.), (1., 1.)]
            .into_iter()
            .enumerate()
        {
            for (axis, size, v, center, offset) in [
                ("x", "W", x, self.center[0], offset[0]),
                ("y", "H", y, self.center[1], offset[1]),
            ] {
                parameters.push(format!(
                    "{axis}{corner}='{size}*({center}+({v}-{center}-{offset}*{t})/{z})'"
                ));
            }
        }
        Ok(format!(
            ",perspective=sense=source:eval=frame:interpolation=linear:{}",
            parameters.join(":")
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn expression_is_bounded_and_only_compiled_from_valid_parameters() {
        let mut d = Design {
            incoming_zoom: 2.,
            outgoing_offset: [-0.2, 0.],
            ..Default::default()
        };
        let expr = d.expression().unwrap();
        assert!(expr.contains("(1-P)"));
        assert!(expr.len() < 20_000);
        d.curve = vec![[0., 0.], [0.5, 0.9], [0.8, 0.2], [1., 1.]];
        assert!(d.expression().is_err());
        d.curve = curve();
        d.feather = 0.;
        assert!(d.expression().is_err());
        assert!(serde_json::from_str::<Design>(r#"{"expr":"movie=foo"}"#).is_err());
    }
}
