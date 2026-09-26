#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
python3 scripts/bundle-mlt.py
(cd frontend && bun run build)
(cd desktop && ../frontend/node_modules/.bin/tauri build --bundles app)
rm -rf Mstudio.app
mv desktop/target/release/bundle/macos/Mstudio.app Mstudio.app
