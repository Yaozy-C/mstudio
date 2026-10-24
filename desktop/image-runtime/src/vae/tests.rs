use super::*;
use mlx_rs::Dtype;
use std::path::PathBuf;

#[test]
#[ignore = "requires installed real VAE weights and Metal"]
fn still_frame_encoder_and_decoder_match_original_3d_vae() {
    crate::initialize(crate::built_metallib()).unwrap();
    crate::select_device(crate::Device::Cpu);
    let directory = PathBuf::from(std::env::var("MSTUDIO_TEST_IMAGE_MODELS").unwrap());
    let mut weights = Weights::from_file(directory.join(crate::restoration::VAE)).unwrap();
    weights.cast_all(Dtype::Float32).unwrap();
    let vae = Vae::load(&weights).unwrap();
    let reference = Weights::from_file(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/vae-f32-reference.safetensors"),
    )
    .unwrap();
    for (actual, expected) in [
        (
            vae.encoder
                .forward(reference.require("pixels").unwrap())
                .unwrap(),
            "encoded",
        ),
        (
            vae.decoder
                .forward(reference.require("latent").unwrap())
                .unwrap(),
            "decoded",
        ),
    ] {
        let expected = reference.require(expected).unwrap();
        actual.eval().unwrap();
        expected.eval().unwrap();
        assert_eq!(actual.shape(), expected.shape());
        let errors: Vec<_> = actual
            .as_slice::<f32>()
            .iter()
            .zip(expected.as_slice::<f32>())
            .map(|(a, b)| (a - b).abs())
            .collect();
        let mean = errors.iter().sum::<f32>() / errors.len() as f32;
        let max = errors.into_iter().fold(0f32, f32::max);
        println!("VAE parity mean={mean}, max={max}");
        assert!(
            mean < 1e-4 && max < 2e-3,
            "VAE differs from original 3D reference: mean={mean}, max={max}"
        );
    }
    drop(vae);
    drop(weights);
    crate::select_device(crate::Device::Metal);
    let mut weights = Weights::from_file(directory.join(crate::restoration::VAE)).unwrap();
    weights.cast_all(Dtype::Float16).unwrap();
    let vae = Vae::load(&weights).unwrap();
    let pixels: Vec<f32> = (0..512 * 512 * 3).map(|i| (i as f32 / 37.).sin()).collect();
    let input = Array::from_slice(&pixels, &[1, 512, 512, 3])
        .as_dtype(Dtype::Float16)
        .unwrap();
    let latent = vae.encoder.forward(&input).unwrap();
    let actual = crate::tensor::host_array(&latent).unwrap();
    assert_eq!(actual.shape(), &[1, 64, 64, 16]);
    assert!(actual.as_slice::<f32>().iter().all(|v| v.is_finite()));
}
