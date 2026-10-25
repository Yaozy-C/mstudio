use super::*;

#[test]
fn zero_denoise_preserves_rgba_without_loading_models() {
    let work = std::env::temp_dir().join(format!("mstudio-zero-denoise-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    let source = work.join("input.png");
    let original = image::RgbaImage::from_fn(35, 33, |x, y| {
        image::Rgba([(x * 7) as u8, (y * 7) as u8, 128, (x + y) as u8])
    });
    original.save(&source).unwrap();
    let paths = restore_with_options(
        &source,
        &work.join("no-models"),
        Device::Metal,
        RestoreOptions {
            scale: 1,
            seed: 42,
            denoise: 0.0,
            vae_tile: 512,
            quantize8: false,
            color_correction: ColorCorrection::Wavelet,
        },
        &work,
    )
    .unwrap();
    assert_eq!(image::open(&paths[0]).unwrap().to_rgba8(), original);
    assert_eq!(image::open(&source).unwrap().to_rgba8(), original);
    std::fs::remove_dir_all(work).unwrap();
}

#[test]
#[ignore = "requires explicitly installed local restoration weights and Metal"]
fn denoise_changes_inference_and_preserves_single_step_equivalence() {
    initialize(built_metallib()).unwrap();
    let models = PathBuf::from(std::env::var("MSTUDIO_TEST_IMAGE_MODELS").unwrap());
    let work = std::env::temp_dir().join(format!("mstudio-denoise-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    let source = work.join("input.png");
    image::RgbImage::from_fn(112, 80, |x, y| {
        image::Rgb([(x * 2) as u8, (y * 3) as u8, ((x + y) % 256) as u8])
    })
    .save(&source)
    .unwrap();
    let mut outputs = Vec::new();
    for denoise in [0.2, 0.4, 0.6, 1.0] {
        let output = work.join(denoise.to_string());
        std::fs::create_dir_all(&output).unwrap();
        let paths = restore_with_options(
            &source,
            &models,
            Device::Metal,
            RestoreOptions {
                scale: 1,
                seed: 42,
                denoise,
                vae_tile: 256,
                quantize8: false,
                color_correction: ColorCorrection::None,
            },
            &output,
        )
        .unwrap();
        let result = image::open(&paths[0]).unwrap().to_rgb8();
        assert_eq!(result.dimensions(), (112, 80));
        eprintln!("denoise={denoise}: completed");
        outputs.push(result.into_raw());
    }
    assert_ne!(outputs[0], outputs[1]);
    assert_ne!(outputs[1], outputs[2]);
    assert_eq!(outputs[2], outputs[3]);
    std::fs::remove_dir_all(work).unwrap();
}

#[test]
#[ignore = "requires explicitly installed local restoration weights and Metal"]
fn repeated_restoration_releases_model_memory_and_q8_runs() {
    let _ = tracing_subscriber::fmt().with_test_writer().try_init();
    initialize(built_metallib()).unwrap();
    let directory = PathBuf::from(std::env::var("MSTUDIO_TEST_IMAGE_MODELS").unwrap());
    let work = std::env::temp_dir().join(format!("mstudio-restoration-{}", std::process::id()));
    std::fs::create_dir_all(&work).unwrap();
    let source = work.join("input.png");
    image::RgbImage::from_fn(112, 80, |x, y| {
        image::Rgb([(x * 2) as u8, (y * 3) as u8, ((x + y) % 256) as u8])
    })
    .save(&source)
    .unwrap();
    let options = RestoreOptions {
        scale: 1,
        seed: 42,
        denoise: 1.0,
        vae_tile: 256,
        quantize8: false,
        color_correction: ColorCorrection::default(),
    };
    let mut baseline = None;
    let mut original = None;
    for run in 0..3 {
        let output = work.join(format!("run-{run}"));
        std::fs::create_dir_all(&output).unwrap();
        let paths = restore_with_options(
            &source,
            &directory,
            Device::Metal,
            RestoreOptions {
                quantize8: run == 2,
                ..options
            },
            &output,
        )
        .unwrap();
        let image = image::open(&paths[0]).unwrap().to_rgb8();
        assert_eq!(image.dimensions(), (112, 80));
        let active = mlx_rs::memory::get_active_memory();
        let cache = mlx_rs::memory::get_cache_memory();
        eprintln!(
            "restore run={run} q8={} cleanup_active={active} cache={cache}",
            run == 2
        );
        if run == 0 {
            baseline = Some(active);
            original = Some(image.into_raw());
        } else {
            assert!(
                active <= baseline.unwrap() + 64 * 1024 * 1024,
                "Model memory grew across completed requests: {active}"
            );
            if run == 1 {
                assert_eq!(image.into_raw(), *original.as_ref().unwrap());
            }
        }
        assert!(cache <= 256 * 1024 * 1024);
    }
    std::fs::remove_dir_all(work).unwrap();
}
