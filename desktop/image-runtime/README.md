# Native image processing

Models load and execute in Rust through MLX/Metal on macOS. There is no Python,
PyTorch, ComfyUI, subprocess inference worker or virtual environment at runtime.
The native MLX kernels require macOS 26.2 or newer and Xcode's Metal toolchain to build.

Install the weights separately; inference jobs never download them:

```sh
cargo run --manifest-path desktop/Cargo.toml --example install_image_models -- /absolute/models
```

The installer verifies SHA-256 values from `models.json`. SeedVR2 uses the
3B transformer and original VAE. Super resolution uses Real-ESRGAN x2plus and
x4plus, with separate weights for each native scale. The safetensors contain
the original F32 PTH parameters without rounding or architecture changes.
Depth uses the official Depth Anything V2 Small HF checkpoint.

Configure `configure_image_processing` with:

```json
{"runtime":{"directory":"/absolute/models","device":"metal"}}
```

Select `metal` or `cpu` explicitly. `directory` contains only model files; neither
a Python executable nor a source checkout is needed. Invalid/missing files and
inference errors stop the job. No old configuration migration or alternate model
selection is provided. Other operating systems currently report unsupported native inference.

## Operations

`process_image({projectId, request: {inputAssetId, name, operation}})` returns
`{inputAssetId, assets, versions}`. Restoration, upscaling and LUT outputs are
stored in the desktop `image_versions` table, not registered as new assets.
Each version has its own ID and retains its parent version and parameters.
Only the explicit `select_image_reference` UI command changes the file resolved
by the stable project asset ID. Original files remain intact. Depth outputs
remain separate reference assets. Omitted or blank names use 未命名.

```json
{"kind":"depth","size":756}
{"kind":"restore","scale":1,"seed":42,"denoise":1,"vaeTile":512,"colorCorrection":"wavelet"}
{"kind":"upscale","scale":2,"tileSize":256}
{"kind":"lut","cube":"TITLE ...\nLUT_3D_SIZE ...\n...","strength":1}
```

- Depth Anything V2 Small: aspect-preserving inference, 8/16-bit PNG. Depth is relative inverse depth, white near, normalized per image.
- SeedVR2 same-size comprehensive restoration (`scale: 1` only): original FP16
  weights on Metal, one Euler/simple step, CFG 1. `denoise` ranges from 0 to 1
  and defaults to 1 for existing requests. It follows ComfyUI's single-step
  KSampler/simple schedule: 0.2 selects sigma 0.2/timestep 200, 0.4 selects
  sigma 0.5/timestep 500, and 0.6 and 1 both select sigma 1/timestep 1000.
  Lower strengths mix the source latent with noise before inference and scale
  the Euler update; this is not output-image blending. Zero saves the oriented
  original without VAE, DiT or color correction. This is not a
  dedicated pure-denoising operation. Color correction is independently selectable.
  The still
  VAE collapses temporal convolution kernels at load, runs 2D convolutions, and
  blends fixed tiles with the same cosine ramps and quarter-tile overlap. Spatially
  reversed kernels with MLX convolution preserve the original cross-correlation
  while selecting GEMM, avoiding overflow in FP16 Winograd intermediates.
- Real-ESRGAN super resolution (`kind: upscale`): native RRDBNet x2plus or x4plus,
  F32 inference, tiles of 128/256/512 source pixels with a 32-pixel context halo
  cropped off before assembly. x2 inputs with odd dimensions receive reflection
  padding, then the output is cropped to exactly 2× the original dimensions.
  Alpha is resized separately with Lanczos. No SeedVR2, VAE, DiT, random noise,
  separate denoising pass or face-restoration stage is invoked.
- LUT: bundled FFmpeg with tetrahedral interpolation, caller-supplied cube/strength.

Rust/MLX uses MLX's keyed Threefry normal noise, so a seed is reproducible within
this backend but does not imply pixel identity with the previous PyTorch output.
CPU inference uses float32. No quality classification, retries, regeneration,
face repair or additional sharpening is performed.

Project image previews expose AI restoration, independent upscaling, and LUT controls.
Their outputs appear in the asset's version selector, with split comparison
against the original. Browsing or creating a version does not select it as the
reference. The Agent, canvas, generation upload, native playback and export all
resolve a project asset ID through its user-selected version.
`configure_image_processing` with `runtime: null` opens a model-folder picker.
Explicit runtime configuration remains supported. LUT uses no model weights.
Depth is an ordinary image reference unless the selected generation endpoint
supports depth conditioning.

The runtime shares the desktop Cargo workspace, lockfile and build directory.

## Sources

The Rust DiT and depth operators are consumed from the pinned Apache-2.0
[SceneWorks inference](https://github.com/SceneWorks/inference) dependency.
The still-frame VAE architecture follows that implementation and
[SeedVR2](https://github.com/IceClear/SeedVR2), with the static-frame temporal
kernel collapse implemented in `src/vae`. MLX Rust bindings and the native MLX
library are pinned by Cargo.lock. Weight files are installed separately and are
not included in the source tree or application bundle.

The native Real-ESRGAN RRDBNet follows the Apache-2.0
[BasicSR architecture](https://github.com/XPixelGroup/BasicSR/blob/master/basicsr/archs/rrdbnet_arch.py).
The original [Real-ESRGAN](https://github.com/xinntao/Real-ESRGAN) weights retain
their BSD-3-Clause license. The pinned Kornia safetensors conversion is recorded
in `models.json`. The opt-in `original_pytorch_rrdbnet_parity` test compares both
native networks with golden outputs from the original PyTorch weights.
