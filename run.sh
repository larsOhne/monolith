#!/usr/bin/env bash
# Run the Monolith desktop (Tauri + React).
# The Python backend is spawned automatically by the Tauri app at startup;
# this script just builds (if needed) and launches the app.
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
DESKTOP="$SCRIPT_DIR/desktop"

# Rust toolchain
source "$HOME/.cargo/env" 2>/dev/null || true

# Python venv (needed so the backend subprocess can find 'monolith-server')
source "$SCRIPT_DIR/.venv/bin/activate"

cd "$DESKTOP"

if [[ "${1:-}" == "--dev" ]]; then
  # Dev mode: hot-reloading Vite + Tauri
  cargo tauri dev
elif [[ "${1:-}" == "--build" ]]; then
  # Production build
  cargo tauri build
else
  # Default: run the already-built debug binary directly
  BINARY="$DESKTOP/src-tauri/target/debug/monolith-desktop"
  if [[ ! -f "$BINARY" ]]; then
    echo "Binary not found — building first (this takes a while)…"
    cargo tauri build --debug
  fi
  exec "$BINARY"
fi
