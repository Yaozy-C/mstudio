use super::*;

#[test]
fn color_modes_match_independent_pytorch_reference_and_preserve_crop() {
    crate::initialize(crate::built_metallib()).unwrap();
    crate::select_device(crate::Device::Cpu);
    let fixture: serde_json::Value = serde_json::from_str(include_str!("reference.json")).unwrap();
    let (w, h) = (
        fixture["width"].as_u64().unwrap() as usize,
        fixture["height"].as_u64().unwrap() as usize,
    );
    let reference = RgbImage::from_fn(w as u32, h as u32, |x, y| {
        image::Rgb(std::array::from_fn(|c| {
            let i = (y as usize * w + x as usize) * 3 + c;
            ((i * 29 + (i / 7) * 13 + 41) % 256) as u8
        }))
    });
    for (method, key) in [
        (ColorCorrection::Wavelet, "wavelet"),
        (ColorCorrection::Lab, "lab"),
        (ColorCorrection::None, "none"),
    ] {
        // Sentinel padding must not enter correction or change at crop edges.
        let mut decoded = HostImage {
            width: w + 5,
            height: h + 3,
            channels: 3,
            values: vec![-7.; (w + 5) * (h + 3) * 3],
        };
        for y in 0..h {
            for x in 0..w {
                for c in 0..3 {
                    let i = (y * w + x) * 3 + c;
                    decoded.values[(y * decoded.width + x) * 3 + c] =
                        ((i * 73 + 19) % 997) as f32 / 498. - 1.;
                }
            }
        }
        let before = decoded.values.clone();
        apply(&mut decoded, &reference, method).unwrap();
        if method == ColorCorrection::None {
            assert_eq!(decoded.values, before);
        } else {
            for sample in fixture["samples"][key].as_array().unwrap() {
                let x = sample["x"].as_u64().unwrap() as usize;
                let y = sample["y"].as_u64().unwrap() as usize;
                for c in 0..3 {
                    let actual = decoded.values[(y * decoded.width + x) * 3 + c];
                    let expected = sample["rgb"][c].as_f64().unwrap() as f32;
                    assert!(
                        (actual - expected).abs() < 0.00005,
                        "{key} ({x},{y},{c}): {actual} != {expected}"
                    );
                }
            }
        }
        for y in 0..decoded.height {
            for x in 0..decoded.width {
                for c in 0..3 {
                    let value = decoded.values[(y * decoded.width + x) * 3 + c];
                    if y >= h || x >= w {
                        assert_eq!(value, -7.);
                    } else {
                        assert!(value.is_finite() && (-1.01..=1.01).contains(&value));
                    }
                }
            }
        }
    }
}

#[test]
#[ignore = "requires explicit local source/reference/output paths; applies color only"]
fn color_modes_on_saved_portrait() {
    crate::initialize(crate::built_metallib()).unwrap();
    crate::select_device(crate::Device::Cpu);
    let content = crate::load_image(std::path::Path::new(
        &std::env::var("MSTUDIO_COLOR_SOURCE").unwrap(),
    ))
    .unwrap();
    let reference = crate::load_image(std::path::Path::new(
        &std::env::var("MSTUDIO_COLOR_REFERENCE").unwrap(),
    ))
    .unwrap();
    assert_eq!(content.dimensions(), reference.dimensions());
    let output = std::path::PathBuf::from(std::env::var("MSTUDIO_COLOR_OUTPUT").unwrap());
    std::fs::create_dir_all(&output).unwrap();
    for (method, name) in [
        (ColorCorrection::Wavelet, "wavelet"),
        (ColorCorrection::Lab, "lab"),
        (ColorCorrection::None, "none"),
    ] {
        let mut decoded = HostImage {
            width: content.width() as usize,
            height: content.height() as usize,
            channels: 3,
            values: content
                .as_raw()
                .iter()
                .map(|v| *v as f32 / 127.5 - 1.)
                .collect(),
        };
        apply(&mut decoded, &reference, method).unwrap();
        decoded
            .rgb8(content.width(), content.height())
            .unwrap()
            .save(output.join(format!("{name}.png")))
            .unwrap();
    }
}
