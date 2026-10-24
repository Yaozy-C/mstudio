# Native image processing

Models load and execute in Rust through MLX/Metal on macOS. There is no Python,
PyTorch, ComfyUI, subprocess inference worker or virtual environment at runtime.
The native MLX kernels require macOS 26.2 or newer and Xcode's Metal toolchain to build.

Install the weights separately; inference jobs never download them:

```sh
cargo run --manifest-path desktop/Cargo.toml --example install_image_models -- /absolute/models
```

The installer verifies SHA-256 values from `models.json`. SeedVR2 uses the
1.4B six-layer distilled sharp transformer and original VAE. Depth uses the
official Depth Anything V2 Small HF checkpoint.

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
`{inputAssetId, assets}`. Every output is a new registered project asset. Input
files remain intact. Omitted or blank names use 未命名.

```json
{"kind":"depth","size":756}
{"kind":"restore","scale":2,"seed":42,"vaeTile":512}
{"kind":"lut","cube":"TITLE ...\nLUT_3D_SIZE ...\n...","strength":1}
```

- Depth Anything V2 Small: aspect-preserving inference, 8/16-bit PNG. Depth is relative inverse depth, white near, normalized per image.
- SeedVR2 restoration and 2×/4× upscaling: original FP16 weights on Metal, one
  Euler/simple step, CFG 1, timestep 1000, color correction disabled. The still
  VAE collapses temporal convolution kernels at load, runs 2D convolutions, and
  blends fixed tiles with the same cosine ramps and quarter-tile overlap. Spatially
  reversed kernels with MLX convolution preserve the original cross-correlation
  while selecting GEMM, avoiding overflow in FP16 Winograd intermediates.
- LUT: bundled FFmpeg with tetrahedral interpolation, caller-supplied cube/strength.

Rust/MLX uses MLX's keyed Threefry normal noise, so a seed is reproducible within
this backend but does not imply pixel identity with the previous PyTorch output.
CPU inference uses float32. No quality classification, retries, regeneration,
color grading, face repair or additional sharpening is performed.

The desktop commands expose individual processing operations through the existing
bridge. Generation workflow and Agent tool integration are handled separately.
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
