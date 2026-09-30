#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
python3 scripts/check_skills.py
python3 scripts/check_source_release.py
python3 scripts/test_source_release.py
python3 scripts/check_source_size.py
cargo fmt --all -- --check
cargo fmt --manifest-path desktop/Cargo.toml -- --check
(cd frontend && ./node_modules/.bin/prettier --check src vite.config.ts '*.json' index.html && bun run build && bun test)
cargo clippy --locked --all-targets -- -D warnings
cargo test --locked
cargo clippy --manifest-path desktop/Cargo.toml --locked --all-targets -- -D warnings
GST_PLUGIN_SYSTEM_PATH_1_0="$PWD/desktop/native/ges-dev-plugins" GST_PLUGIN_PATH_1_0= GST_REGISTRY_FORK=no cargo test --manifest-path desktop/Cargo.toml --locked --bin mstudio-desktop
