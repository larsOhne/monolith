#!/usr/bin/env bash
set -euo pipefail

SCRIPT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"

# Rust toolchain
source "$HOME/.cargo/env"

# Python venv
source "$SCRIPT_DIR/.venv/bin/activate"

# Start Python backend in background; capture PID for cleanup
monolith-server --host 127.0.0.1 &
BACKEND_PID=$!
trap 'kill "$BACKEND_PID" 2>/dev/null' EXIT

echo "Backend PID: $BACKEND_PID"

# Launch desktop (blocks until window is closed)
"$SCRIPT_DIR/desktop/target/debug/monolith-desktop"
