use anyhow::{Result, ensure};

/// ComfyUI KSampler: steps=1, simple scheduler, SeedVR2 FLOW shift=1.
/// Select the final nonzero sigma from the expanded schedule, not an RGB blend.
pub(crate) fn sigma(denoise: f64) -> Result<f32> {
    ensure!(
        denoise.is_finite() && (0.0..=1.0).contains(&denoise),
        "Denoise 须为 0–1"
    );
    if denoise == 0.0 {
        return Ok(0.0);
    }
    let steps = (1.0 / denoise).floor().min(1000.0);
    // The simple scheduler selects from 1000 discrete sigmas (0.001..=1).
    Ok(((1000.0 / steps).ceil() / 1000.0) as f32)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_comfy_single_step_simple_schedule() {
        for (strength, expected) in [
            (0.0, 0.0),
            (0.1, 0.1),
            (0.2, 0.2),
            (0.3, 0.334),
            (0.4, 0.5),
            (0.5, 0.5),
            (0.6, 1.0),
            (1.0, 1.0),
        ] {
            assert_eq!(sigma(strength).unwrap(), expected);
        }
        for invalid in [-0.1, 1.1, f64::NAN, f64::INFINITY] {
            assert!(sigma(invalid).is_err());
        }
    }
}
