#!/usr/bin/env bash
set -euo pipefail

URL="${1:-}"
NUM="${2:-100}"

if [ -z "$URL" ]; then
    echo "Usage: $0 <url> [num_requests]"
    echo "  Example: $0 http://192.168.1.86:8082/benchmark 200"
    exit 1
fi

SCRIPT_DIR="$(cd "$(dirname "$0")" && pwd)"

separator() { printf '\n%s\n\n' "========================================"; }

# --- Python ---
separator
echo "[Python]"
python3 "$SCRIPT_DIR/python/bench.py" "$URL" "$NUM"

# --- Node.js ---
separator
echo "[Node.js]"
node "$SCRIPT_DIR/node/bench.js" "$URL" "$NUM"

# --- Go ---
separator
echo "[Go]"
if [ ! -f "$SCRIPT_DIR/go/bench" ]; then
    echo "Building Go binary..."
    (cd "$SCRIPT_DIR/go" && go build -o bench .)
fi
"$SCRIPT_DIR/go/bench" "$URL" "$NUM"

# --- Rust ---
separator
echo "[Rust]"
RUST_BIN="$SCRIPT_DIR/rust/target/x86_64-unknown-linux-gnu/release/bench"
if [ ! -f "$RUST_BIN" ]; then
    echo "Building Rust binary..."
    (cd "$SCRIPT_DIR/rust" && cargo build --release --target x86_64-unknown-linux-gnu 2>&1)
fi
"$RUST_BIN" "$URL" "$NUM"
