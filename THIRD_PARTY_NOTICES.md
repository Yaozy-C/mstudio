# Third-party notices

## Provider brand icons

The SVG marks in `frontend/src/assets/providers/` originate from LobeHub Icons (`@lobehub/icons-static-svg`), distributed under the MIT License. See [upstream](https://github.com/lobehub/lobe-icons) and the [retained license](frontend/src/assets/providers/LICENSE). Brand names and trademarks belong to their respective owners; inclusion does not imply endorsement.

## Dependencies and native runtime

Rust dependencies are recorded in both `Cargo.lock` files; frontend dependencies are recorded in `frontend/bun.lock`. Dependencies retain their own licenses. Development requires separately installed GStreamer/GES and FFmpeg. The macOS application bundle includes FFmpeg/ffprobe executables, GStreamer/GES plugins and their transitive dynamic libraries. No downloaded native SDK, dependency library or compiled application is included in this source repository.

The project MIT License applies to project-owned code, not to third-party components. The native bundle script copies available license/notice files and Homebrew build metadata from the actual staged dependency packages into `gstreamer/licenses`, and records binary provenance and FFmpeg build configuration in `gstreamer/runtime.json`. These materials are an inventory, not a complete corresponding-source distribution. Before distributing a compiled application, inventory the actual transitive runtime libraries, preserve their required notices and satisfy the requirements of their exact build configurations. This source publication does not certify a binary distribution.

## Windows runtime

Windows packages include the official GStreamer MSVC runtime/plugins and app-local Visual C++ runtime files from the build toolchain. FFmpeg/ffprobe are the pinned Gyan essentials build, which identifies itself as GPLv3. Available package notices are retained under `licenses/`; runtime versions and FFmpeg configuration are recorded in `gstreamer/runtime.json`. This build inventory does not replace corresponding-source and other redistribution requirements.
