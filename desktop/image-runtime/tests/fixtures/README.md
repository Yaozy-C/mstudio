# VAE numerical reference

`vae-f32-reference.safetensors` contains synthetic inputs and reference activations,
not model weights or user images. It was produced with PyTorch 2.14.1 on CPU using
the original 3D SeedVR2 VAE in ComfyUI revision
`7cebca52227c63b4180ceed448cbd7ad22903b6d`, without the 2D convolution adaptation.

- Checkpoint: `seedvr2_ema_vae_fp16.safetensors`, SHA-256
  `20678548f420d98d26f11442d3528f8b8c94e57ee046ef93dbb7633da8612ca1`.
- Weights were loaded strictly into float32. Inference used `MemoryState.DISABLED`.
- Pixels: `sin(arange(3*32*40)/37)`, reshaped to `[1,3,1,32,40]`.
- Latent: `cos(arange(16*4*5)/13)`, reshaped to `[1,16,1,4,5]`.
- Encoded reference: first half of encoder output channels, multiplied by `0.9152`.
- Decoded reference: decoder output from `latent/0.9152`.
- All four tensors are stored as contiguous NHWC after removing the temporal axis.

The Rust test compares both encoder and decoder against these independent values,
then exercises a 512×512 FP16 Metal encode to cover the Winograd overflow regression.
Run with installed model files:

```sh
MSTUDIO_TEST_IMAGE_MODELS=/absolute/models cargo test --manifest-path desktop/image-runtime/Cargo.toml -- --include-ignored --test-threads=1
```
