//! Editable SDR color recipe. Pixel-local operations are baked once, not per video frame.
//! Tone weights, hue bands and wheels adapt the author's mlight photo color pipeline.
use anyhow::{Result, ensure};
use serde::{Deserialize, Serialize};
#[cfg(test)]
#[path = "grading_tests.rs"]
mod tests;

fn curves() -> [[f64; 3]; 4] {
    [[0.25, 0.5, 0.75]; 4]
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default, rename_all = "camelCase", deny_unknown_fields)]
pub struct Grade {
    pub exposure: f64,
    pub temperature: f64,
    pub tint: f64,
    pub contrast: f64,
    pub saturation: f64,
    pub vibrance: f64,
    pub shadows: f64,
    pub highlights: f64,
    pub whites: f64,
    pub blacks: f64,
    /// Eight hue bands: red, orange, yellow, green, aqua, blue, purple, magenta.
    pub hsl: [[f64; 3]; 8],
    /// Master, R, G, B; y at x=.25, .5, .75; endpoints are fixed at 0 and 1.
    pub curves: [[f64; 3]; 4],
    /// Shadow, midtone, highlight; hue degrees, saturation percent, luminance percent.
    pub wheels: [[f64; 3]; 3],
    pub balance: f64,
}
impl Default for Grade {
    fn default() -> Self {
        Self {
            exposure: 0.,
            temperature: 0.,
            tint: 0.,
            contrast: 0.,
            saturation: 0.,
            vibrance: 0.,
            shadows: 0.,
            highlights: 0.,
            whites: 0.,
            blacks: 0.,
            hsl: [[0.; 3]; 8],
            curves: curves(),
            wheels: [[0.; 3]; 3],
            balance: 0.,
        }
    }
}
impl Grade {
    pub fn validate(&self) -> Result<()> {
        let range = |v: f64, lo: f64, hi: f64| v.is_finite() && v >= lo && v <= hi;
        ensure!(range(self.exposure, -3., 3.), "曝光须为 -3 至 3 EV");
        ensure!(
            [
                self.temperature,
                self.tint,
                self.contrast,
                self.saturation,
                self.vibrance,
                self.shadows,
                self.highlights,
                self.whites,
                self.blacks,
                self.balance
            ]
            .into_iter()
            .chain(self.hsl.into_iter().flatten())
            .all(|v| range(v, -100., 100.)),
            "调色数值须为 -100 至 100"
        );
        ensure!(
            self.curves
                .iter()
                .all(|c| c.iter().all(|&v| range(v, 0., 1.)) && c[0] <= c[1] && c[1] <= c[2]),
            "曲线控制点须在 0–1 内单调递增"
        );
        ensure!(
            self.wheels.iter().all(|w| range(w[0], 0., 360.)
                && range(w[1], 0., 100.)
                && range(w[2], -100., 100.)),
            "色轮数值无效"
        );
        Ok(())
    }
    /// Input and output are normalized display RGB. Internal tone math uses linear sRGB.
    /// This is an SDR look transform, not a camera Log/HDR input transform.
    pub fn pixel(&self, rgb: [f64; 3]) -> [f64; 3] {
        let gains = [
            self.temperature * 0.005 + self.tint * 0.0015,
            -self.tint * 0.003,
            -self.temperature * 0.005 + self.tint * 0.0015,
        ];
        let mut p = std::array::from_fn(|i| linear(rgb[i]) * 2f64.powf(self.exposure + gains[i]));
        let luminance = luma(p);
        if luminance > 0. {
            p = p.map(|v| v * (luminance / 0.18).powf(2f64.powf(self.contrast / 200.) - 1.));
        }
        let guide = srgb(luma(p).max(0.)).clamp(0., 1.);
        let gain = 2f64.powf(
            (self.shadows * (1. - smooth(guide / 0.6))
                + self.highlights * smooth((guide - 0.4) / 0.6))
                / 100.,
        );
        p = p.map(|v| v * gain);
        let guide = srgb(luma(p).max(0.)).clamp(0., 1.);
        p = p.map(|v| {
            v * 2f64.powf((self.whites * guide.powi(4) + self.blacks * (1. - guide).powi(4)) / 200.)
        });
        let gray = luma(p);
        p = p.map(|v| srgb(gray + (v - gray) * (1. + self.saturation / 100.)));
        for (i, v) in p.iter_mut().enumerate() {
            *v = curve(curve(*v, self.curves[0]), self.curves[i + 1]);
        }
        if self.vibrance != 0. || self.hsl.iter().flatten().any(|&v| v != 0.) {
            let floor = p.into_iter().fold(0., f64::min);
            let scale = p.into_iter().fold(1., f64::max) - floor;
            let mut h = to_hsl(p.map(|v| (v - floor) / scale));
            let mut delta = [0.; 3];
            let mut total: f64 = 0.;
            for (band, center) in self
                .hsl
                .iter()
                .zip([0., 30., 60., 120., 180., 240., 270., 300.])
            {
                let distance = ((h[0] - center + 180.).rem_euclid(360.) - 180.).abs();
                let w = smooth(1. - distance / 60.);
                total += w;
                for i in 0..3 {
                    delta[i] += band[i] * w;
                }
            }
            let chroma = h[1];
            h[0] += delta[0] / total.max(1.) * 0.3;
            h[1] = (h[1]
                * (1. + delta[1] / total.max(1.) / 100.)
                * (1. + self.vibrance / 100. * (1. - h[1])))
                .clamp(0., 1.);
            h[2] = (h[2] + delta[2] / total.max(1.) / 100. * chroma * 0.25).clamp(0., 1.);
            p = from_hsl(h).map(|v| v * scale + floor);
        }
        let lum = (luma(p) + self.balance / 250.).clamp(0., 1.);
        for (wheel, weight) in
            self.wheels
                .iter()
                .zip([(1. - lum).powi(2), 2. * lum * (1. - lum), lum.powi(2)])
        {
            let tint = from_hsl([wheel[0], 1., 0.5]);
            let gray = luma(tint);
            for i in 0..3 {
                p[i] += weight * ((tint[i] - gray) * wheel[1] * 0.003 + wheel[2] * 0.0015);
            }
        }
        p.map(|v| v.clamp(0., 1.))
    }
}
fn luma(p: [f64; 3]) -> f64 {
    p[0] * 0.2126 + p[1] * 0.7152 + p[2] * 0.0722
}
fn smooth(v: f64) -> f64 {
    let t = v.clamp(0., 1.);
    t * t * (3. - 2. * t)
}
fn linear(v: f64) -> f64 {
    if v <= 0.04045 {
        v / 12.92
    } else {
        ((v + 0.055) / 1.055).powf(2.4)
    }
}
fn srgb(v: f64) -> f64 {
    if v <= 0.0031308 {
        v * 12.92
    } else {
        1.055 * v.powf(1. / 2.4) - 0.055
    }
}
fn curve(v: f64, c: [f64; 3]) -> f64 {
    let points = [0., c[0], c[1], c[2], 1.];
    let i = ((v * 4.).floor() as isize).clamp(0, 3) as usize;
    points[i] + (v * 4. - i as f64) * (points[i + 1] - points[i])
}
fn to_hsl(p: [f64; 3]) -> [f64; 3] {
    let max = p.into_iter().fold(f64::NEG_INFINITY, f64::max);
    let min = p.into_iter().fold(f64::INFINITY, f64::min);
    let d = max - min;
    let l = (max + min) / 2.;
    if d < 1e-7 {
        return [0., 0., l];
    }
    let h = if max == p[0] {
        (p[1] - p[2]) / d
    } else if max == p[1] {
        (p[2] - p[0]) / d + 2.
    } else {
        (p[0] - p[1]) / d + 4.
    };
    [
        (h * 60.).rem_euclid(360.),
        d / (1. - (2. * l - 1.).abs()).max(1e-7),
        l,
    ]
}
fn from_hsl([h, s, l]: [f64; 3]) -> [f64; 3] {
    let c = (1. - (2. * l - 1.).abs()) * s;
    let hue = h.rem_euclid(360.) / 60.;
    let x = c * (1. - (hue % 2. - 1.).abs());
    let p = match hue as u8 {
        0 => [c, x, 0.],
        1 => [x, c, 0.],
        2 => [0., c, x],
        3 => [0., x, c],
        4 => [x, 0., c],
        _ => [c, 0., x],
    };
    p.map(|v| v + l - c / 2.)
}
