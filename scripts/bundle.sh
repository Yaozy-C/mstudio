#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
export MACOSX_DEPLOYMENT_TARGET=26.0
python3 scripts/bundle-ges.py
(cd frontend && bun run build)
(cd desktop && ../frontend/node_modules/.bin/tauri build --bundles app)
python3 scripts/fix-ges-bundle.py
cargo build --manifest-path desktop/Cargo.toml --locked --release --example bundled_media
python3 scripts/check-macos-bundle.py desktop/target/release/bundle/macos/Mstudio.app desktop/target/release/examples/bundled_media
rm -rf Mstudio.app
mv desktop/target/release/bundle/macos/Mstudio.app Mstudio.app
