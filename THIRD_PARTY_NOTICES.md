# Third-party notices

## Provider brand icons

The SVG marks in `frontend/src/assets/providers/` originate from LobeHub Icons (`@lobehub/icons-static-svg`), distributed under the MIT License. See [upstream](https://github.com/lobehub/lobe-icons) and the [retained license](frontend/src/assets/providers/LICENSE). Brand names and trademarks belong to their respective owners; inclusion does not imply endorsement.

## Dependencies and native runtime

Rust dependencies are recorded in both `Cargo.lock` files; frontend dependencies are recorded in `frontend/bun.lock`. Dependencies retain their own licenses. MLT is downloaded at build time with a pinned version and checksum; FFmpeg, SDL2 and other native libraries are installed separately. No downloaded native SDK, dependency library or compiled application is included in this source repository.

The project MIT License applies to project-owned code, not to third-party components. The native bundle script currently copies MLT license files only. Before distributing a compiled application, inventory the actual transitive runtime libraries, preserve their required notices and satisfy the requirements of their exact build configurations. This source publication does not certify a binary distribution.
