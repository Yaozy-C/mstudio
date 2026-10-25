# Third-party notices

## Local image processing

Native image processing uses the pinned Apache-2.0 SceneWorks Rust MLX inference crates for SeedVR2 DiT and Depth Anything V2, and the pinned pmetal MLX Rust bindings with Apple's native MLX library. The still-frame VAE architecture in `desktop/image-runtime/src/vae` follows SceneWorks' SeedVR2 implementation and the Apache-2.0 upstream SeedVR2 architecture, adapted to collapse temporal kernels and run two-dimensional convolutions on still images. Dependency versions and source revisions are recorded in Cargo.lock. Model URLs and SHA-256 digests are recorded in `desktop/image-runtime/models.json`; model weights are installed separately. See `desktop/image-runtime/README.md`. No Python runtime is shipped or invoked for inference.

Native Real-ESRGAN super resolution follows the Apache-2.0 BasicSR RRDBNet
architecture, Copyright 2021 Xintao Wang (https://github.com/XPixelGroup/BasicSR).
Real-ESRGAN x2plus/x4plus weights are distributed under BSD-3-Clause, Copyright
2021 Xintao Wang (https://github.com/xinntao/Real-ESRGAN). The retained license
is `desktop/image-runtime/licenses/Real-ESRGAN-BSD-3-Clause`. The pinned Kornia
safetensors conversion contains the original F32 model tensors; conversion
URLs and SHA-256 digests are recorded in `desktop/image-runtime/models.json`.

## Provider brand icons

The SVG marks in `frontend/src/assets/providers/` originate from LobeHub Icons (`@lobehub/icons-static-svg`), distributed under the MIT License. See [upstream](https://github.com/lobehub/lobe-icons) and the [retained license](frontend/src/assets/providers/LICENSE). Brand names and trademarks belong to their respective owners; inclusion does not imply endorsement.

## Dependencies and native runtime

Rust dependencies are recorded in the `Cargo.lock` files; frontend dependencies are recorded in `frontend/bun.lock`. Dependencies retain their own licenses. The native model licenses are retained in `desktop/image-runtime/licenses` and packaged under `mlx/licenses`. Development requires separately installed GStreamer/GES and FFmpeg. The macOS application bundle includes FFmpeg/ffprobe executables, GStreamer/GES plugins and their transitive dynamic libraries. No downloaded native SDK, dependency library or compiled application is included in this source repository.

The project licensing transition in LICENSE applies only to new project-owned changes. Historical MIT grants remain effective; see LICENSE-MIT-LEGACY. Third-party components retain their own licenses. The native bundle script copies available license/notice files and Homebrew build metadata from the actual staged dependency packages into `gstreamer/licenses`, and records binary provenance and FFmpeg build configuration in `gstreamer/runtime.json`. These materials are an inventory, not a complete corresponding-source distribution. Before distributing a compiled application, inventory the actual transitive runtime libraries, preserve their required notices and satisfy the requirements of their exact build configurations. This source publication does not certify a binary distribution.

## Windows runtime

Windows packages include the official GStreamer MSVC runtime/plugins and app-local Visual C++ runtime files from the build toolchain. FFmpeg/ffprobe are the pinned Gyan essentials build, which identifies itself as GPLv3. Available package notices are retained under `licenses/`; runtime versions and FFmpeg configuration are recorded in `gstreamer/runtime.json`. This build inventory does not replace corresponding-source and other redistribution requirements.
