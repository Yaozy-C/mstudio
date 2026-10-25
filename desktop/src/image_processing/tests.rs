use super::*;
use mstudio::media;
use serde_json::json;
use std::{
    path::{Path, PathBuf},
    process::Command,
};

fn fixture() -> (Store, String) {
    let (store, _, _) = crate::project_service::tests::fixture();
    let work = store.root.join("synthetic");
    std::fs::create_dir_all(&work).unwrap();
    media::run(
        Command::new(media::binary("ffmpeg"))
            .args([
                "-v",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=35x33:rate=1",
                "-frames:v",
                "1",
                "-threads",
                "1",
            ])
            .arg(work.join("output.png")),
    )
    .unwrap();
    let assets = storage::save(
        &store,
        "p",
        "Synthetic image",
        &[work.join("output.png")],
        None,
    )
    .unwrap();
    (store, assets[0].id.clone())
}
fn cleanup(store: Store) {
    let root = store.root.clone();
    drop(store);
    std::fs::remove_dir_all(root).unwrap();
}
fn pixels(path: &Path) -> Vec<u8> {
    let result = Command::new(media::binary("ffmpeg"))
        .args(["-v", "error", "-i"])
        .arg(path)
        .args(["-frames:v", "1", "-pix_fmt", "rgb24", "-f", "rawvideo", "-"])
        .output()
        .unwrap();
    assert!(result.status.success());
    result.stdout
}

#[test]
fn failed_operations_and_wrong_project_inputs_do_not_publish_assets() {
    let (store, id) = fixture();
    let before = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    for operation in [
        Operation::Depth { size: 756 },
        Operation::Lut {
            cube: "invalid".into(),
            strength: 1.0,
        },
    ] {
        assert!(
            process(
                &store,
                "p",
                Request {
                    input_asset_id: id.clone(),
                    input_version_id: None,
                    name: "Test".into(),
                    operation
                },
                None,
            )
            .is_err()
        );
    }
    assert!(storage::input(&store, "p", "another-project-image").is_err());
    assert_eq!(
        crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap(),
        before
    );
    assert_eq!(store.assets().unwrap().len(), 1);
    assert_eq!(
        store
            .db
            .lock()
            .unwrap()
            .query_row("SELECT count(*) FROM image_processing_results", [], |r| r
                .get::<_, i64>(
                0
            ))
            .unwrap(),
        0
    );
    cleanup(store);
}

#[test]
fn lut_strength_controls_pixels_and_preserves_the_source() {
    let (store, id) = fixture();
    let source = storage::input(&store, "p", &id).unwrap();
    let original = pixels(&source);
    let source_bytes = std::fs::read(&source).unwrap();
    let mut outputs = Vec::new();
    let mut cube = "LUT_3D_SIZE 2\n".to_owned();
    for b in 0..2 {
        for g in 0..2 {
            for r in 0..2 {
                cube.push_str(&format!("{} {} {}\n", 1 - r, 1 - g, 1 - b));
            }
        }
    }
    for strength in [0.0, 0.5, 1.0] {
        let receipt = process(
            &store,
            "p",
            Request {
                input_asset_id: id.clone(),
                input_version_id: None,
                name: String::new(),
                operation: Operation::Lut {
                    cube: cube.clone(),
                    strength,
                },
            },
            None,
        )
        .unwrap();
        assert!(receipt.assets.is_empty());
        let version = &receipt.versions[0];
        assert_eq!(version.image.name, "未命名");
        assert_eq!(version.image.id, version.id);
        let db = store.db.lock().unwrap();
        let raw: String = db
            .query_row(
                "SELECT provenance FROM image_versions WHERE project_id='p' AND id=?1",
                [&version.id],
                |r| r.get(0),
            )
            .unwrap();
        let provenance: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(provenance["operation"]["cube"], cube);
        assert_eq!(provenance["operation"]["strength"], json!(strength));
        assert_eq!(provenance["engine"], "ffmpeg-lut3d-tetrahedral");
        drop(db);
        let project = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
        assert_eq!(project["assets"].as_array().unwrap().len(), 1);
        assert_eq!(
            Path::new(project["assets"][0]["path"].as_str().unwrap())
                .canonicalize()
                .unwrap(),
            source.canonicalize().unwrap()
        );
        assert_eq!(store.assets().unwrap().len(), 1);
        outputs.push(version.image.path.clone());
        let actual = pixels(Path::new(&version.image.path));
        for (value, input) in actual.iter().zip(&original) {
            let expected = f32::from(*input) * (1.0 - strength) + f32::from(255 - input) * strength;
            assert!((f32::from(*value) - expected).abs() <= 1.0);
        }
    }
    assert_eq!(std::fs::read(&source).unwrap(), source_bytes);
    crate::project_storage::remove(&store, "p").unwrap();
    assert!(outputs.iter().all(|path| !Path::new(path).exists()));
    cleanup(store);
}

#[test]
fn restoration_options_accept_old_requests_and_validate_tile_and_q8() {
    let old = json!({"kind":"restore","scale":1,"seed":42,"vaeTile":512});
    let operation: Operation = serde_json::from_value(old.clone()).unwrap();
    assert_eq!(
        serde_json::to_value(&operation).unwrap()["colorCorrection"],
        "wavelet"
    );
    assert_eq!(serde_json::to_value(&operation).unwrap()["denoise"], 1.0);
    for denoise in [0.0, 0.2, 0.4, 0.6, 1.0] {
        let mut request = old.clone();
        request["denoise"] = json!(denoise);
        let operation: Operation = serde_json::from_value(request).unwrap();
        operation.validate().unwrap();
        assert_eq!(serde_json::to_value(operation).unwrap()["denoise"], denoise);
    }
    for denoise in [-0.1, 1.1] {
        let mut request = old.clone();
        request["denoise"] = json!(denoise);
        assert!(
            serde_json::from_value::<Operation>(request)
                .unwrap()
                .validate()
                .is_err()
        );
    }
    for mode in ["wavelet", "lab", "none"] {
        let mut request = old.clone();
        request["colorCorrection"] = json!(mode);
        let operation: Operation = serde_json::from_value(request).unwrap();
        operation.validate().unwrap();
        assert_eq!(
            serde_json::to_value(operation).unwrap()["colorCorrection"],
            mode
        );
    }
    let mut invalid = old.clone();
    invalid["colorCorrection"] = json!("unsupported");
    assert!(serde_json::from_value::<Operation>(invalid).is_err());
    assert!(
        !serde_json::to_value(operation).unwrap()["quantize8"]
            .as_bool()
            .unwrap()
    );
    for tile in [256, 384, 512] {
        let mut request = old.clone();
        request["vaeTile"] = json!(tile);
        request["quantize8"] = json!(true);
        serde_json::from_value::<Operation>(request)
            .unwrap()
            .validate()
            .unwrap();
    }
    for tile in [0, 288, 4096] {
        let mut request = old.clone();
        request["vaeTile"] = json!(tile);
        assert!(
            serde_json::from_value::<Operation>(request)
                .unwrap()
                .validate()
                .is_err()
        );
    }
}

#[test]
fn upscale_requests_are_separate_from_restoration_and_reject_mixed_parameters() {
    for scale in [2, 4] {
        let operation: Operation =
            serde_json::from_value(json!({"kind":"upscale","scale":scale,"tileSize":256})).unwrap();
        operation.validate().unwrap();
        let old: Operation =
            serde_json::from_value(json!({"kind":"restore","scale":scale,"seed":42,"vaeTile":512}))
                .unwrap();
        assert!(old.validate().is_err());
    }
    assert!(
        serde_json::from_value::<Operation>(
            json!({"kind":"upscale","scale":2,"tileSize":256,"vaeTile":512,"seed":42})
        )
        .is_err()
    );
    for (scale, tile) in [(1, 256), (3, 256), (2, 0), (4, 257)] {
        let operation: Operation =
            serde_json::from_value(json!({"kind":"upscale","scale":scale,"tileSize":tile}))
                .unwrap();
        assert!(operation.validate().is_err());
    }
}

#[test]
#[ignore = "requires installed ESRGAN weights and Metal; deliberately excludes SeedVR2 files"]
fn esrgan_upscale_needs_no_restoration_models_and_records_its_own_engine() {
    #[cfg(target_os = "macos")]
    mstudio_image_runtime::initialize(mstudio_image_runtime::built_metallib()).unwrap();
    let installed = PathBuf::from(std::env::var("MSTUDIO_TEST_IMAGE_MODELS").unwrap());
    let (store, id) = fixture();
    let models = store.root.join("esrgan-only");
    std::fs::create_dir(&models).unwrap();
    for scale in [2, 4] {
        let name = format!("RealESRGAN_x{scale}plus.safetensors");
        std::fs::copy(installed.join(&name), models.join(&name)).unwrap();
    }
    let runtime = Runtime {
        directory: models,
        device: spec::Device::Metal,
    };
    let source = storage::input(&store, "p", &id).unwrap();
    let original = std::fs::read(&source).unwrap();
    let before = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    for scale in [2, 4] {
        let receipt = process(
            &store,
            "p",
            Request {
                input_asset_id: id.clone(),
                input_version_id: None,
                name: "ESRGAN".into(),
                operation: Operation::Upscale {
                    scale,
                    tile_size: 128,
                },
            },
            Some(&runtime),
        )
        .unwrap();
        assert_eq!(
            (
                receipt.versions[0].image.width,
                receipt.versions[0].image.height
            ),
            (35 * scale, 33 * scale)
        );
        let raw: String = store
            .db
            .lock()
            .unwrap()
            .query_row(
                "SELECT provenance FROM image_versions WHERE id=?1",
                [&receipt.versions[0].id],
                |r| r.get(0),
            )
            .unwrap();
        let provenance: serde_json::Value = serde_json::from_str(&raw).unwrap();
        assert_eq!(provenance["engine"], format!("realesrgan-x{scale}plus"));
        assert_eq!(
            provenance["operation"],
            json!({"kind":"upscale","scale":scale,"tileSize":128})
        );
        assert_eq!(provenance["inputAssetId"], id);
    }
    assert_eq!(std::fs::read(source).unwrap(), original);
    let after = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
    assert_eq!(after["nodes"], before["nodes"]);
    assert_eq!(after["clips"], before["clips"]);
    cleanup(store);
}

#[test]
#[ignore = "requires explicitly installed local image models; no network or generation"]
fn local_models_process_depth_restoration_and_independent_upscale() {
    #[cfg(target_os = "macos")]
    mstudio_image_runtime::initialize(mstudio_image_runtime::built_metallib()).unwrap();
    let runtime = Runtime {
        directory: PathBuf::from(std::env::var("MSTUDIO_TEST_IMAGE_MODELS").unwrap()),
        device: serde_json::from_value(json!(std::env::var("MSTUDIO_TEST_IMAGE_DEVICE").unwrap()))
            .unwrap(),
    };
    let (store, id) = fixture();
    for operation in [
        Operation::Depth { size: 56 },
        Operation::Restore {
            scale: 1,
            seed: 42,
            denoise: 1.0,
            vae_tile: 512,
            quantize8: false,
            color_correction: mstudio_image_runtime::ColorCorrection::None,
        },
        Operation::Upscale {
            scale: 2,
            tile_size: 256,
        },
        Operation::Upscale {
            scale: 4,
            tile_size: 256,
        },
    ] {
        let scale = match &operation {
            Operation::Restore { scale, .. } | Operation::Upscale { scale, .. } => *scale,
            _ => 1,
        };
        let is_depth = matches!(operation, Operation::Depth { .. });
        let receipt = process(
            &store,
            "p",
            Request {
                input_asset_id: id.clone(),
                input_version_id: None,
                name: "Model output".into(),
                operation,
            },
            Some(&runtime),
        )
        .unwrap();
        let output = if is_depth {
            &receipt.assets[0]
        } else {
            &receipt.versions[0].image
        };
        assert_eq!((output.width, output.height), (35 * scale, 33 * scale));
        assert!(Path::new(&output.path).is_file());
        if is_depth {
            assert_eq!(receipt.assets.len(), 2);
        }
    }
    cleanup(store);
}
