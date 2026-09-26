#!/bin/sh
set -eu
cd "$(dirname "$0")/.."
PROJECT_DIR=$(pwd)
(cd frontend && exec bun run dev) &
VITE_PID=$!
trap 'kill "$VITE_PID" 2>/dev/null || true' EXIT INT TERM
cd desktop
"$PROJECT_DIR/frontend/node_modules/.bin/tauri" dev "$@"
