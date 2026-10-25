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
    let assets = storage::save(&store, "p", "Synthetic image", &[work.join("output.png")]).unwrap();
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
                name: String::new(),
                operation: Operation::Lut {
                    cube: cube.clone(),
                    strength,
                },
            },
            None,
        )
        .unwrap();
        assert_eq!(receipt.assets[0].name, "未命名");
        let project = crate::project_service::load(&store.db.lock().unwrap(), "p").unwrap();
        assert!(
            project["assets"]
                .as_array()
                .unwrap()
                .iter()
                .any(|a| a["id"] == receipt.assets[0].id && a["inLibrary"] == true)
        );
        outputs.push(receipt.assets[0].path.clone());
        let actual = pixels(Path::new(&receipt.assets[0].path));
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
#[ignore = "requires explicitly installed local image models; no network or generation"]
fn local_models_process_depth_and_seedvr2_restoration() {
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
            vae_tile: 512,
        },
        Operation::Restore {
            scale: 2,
            seed: 42,
            vae_tile: 512,
        },
        Operation::Restore {
            scale: 4,
            seed: 42,
            vae_tile: 512,
        },
    ] {
        let scale = match &operation {
            Operation::Restore { scale, .. } => *scale,
            _ => 1,
        };
        let is_depth = matches!(operation, Operation::Depth { .. });
        let receipt = process(
            &store,
            "p",
            Request {
                input_asset_id: id.clone(),
                name: "Model output".into(),
                operation,
            },
            Some(&runtime),
        )
        .unwrap();
        assert_eq!(
            (receipt.assets[0].width, receipt.assets[0].height),
            (35 * scale, 33 * scale)
        );
        assert!(storage::input(&store, "p", &receipt.assets[0].id).is_ok());
        if is_depth {
            assert_eq!(receipt.assets.len(), 2);
        }
    }
    cleanup(store);
}
