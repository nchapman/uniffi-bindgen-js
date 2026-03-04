#!/usr/bin/env bash
# Build the FFI-mode fixture and generate bindings into binding_tests/generated/.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIXTURE_DIR="$REPO_ROOT/fixtures/ffi-basic"
OUT_DIR="$REPO_ROOT/binding_tests/generated"

# 1. Build the wasm
echo "Building ffi-basic wasm..."
(cd "$FIXTURE_DIR/wasm" && cargo build --target wasm32-unknown-unknown --release)

# 2. Generate bindings with --wasm
echo "Generating FFI bindings..."
cargo run --manifest-path "$REPO_ROOT/Cargo.toml" -- generate \
  "$FIXTURE_DIR/src/ffi_basic.udl" \
  --wasm "$FIXTURE_DIR/wasm/target/wasm32-unknown-unknown/release/ffi_basic.wasm" \
  --out-dir "$OUT_DIR"

echo "FFI fixture built. Files in $OUT_DIR:"
ls -la "$OUT_DIR"/ffi_basic.* "$OUT_DIR"/uniffi_runtime.* 2>/dev/null || true
