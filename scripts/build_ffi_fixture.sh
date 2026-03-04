#!/usr/bin/env bash
# Build all FFI-mode fixtures and generate bindings into binding_tests/generated/.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
OUT_DIR="$REPO_ROOT/binding_tests/generated"

build_ffi_fixture() {
  local name="$1"
  local ns="$2"  # Rust crate/namespace name (underscored)

  local fixture_dir="$REPO_ROOT/fixtures/$name"
  local wasm_path="$fixture_dir/wasm/target/wasm32-unknown-unknown/release/$ns.wasm"

  echo "Building $name wasm..."
  (cd "$fixture_dir/wasm" && cargo build --target wasm32-unknown-unknown --release)

  echo "Generating FFI bindings for $name..."
  cargo run --manifest-path "$REPO_ROOT/Cargo.toml" -- generate \
    "$fixture_dir/src/$ns.udl" \
    --wasm "$wasm_path" \
    --out-dir "$OUT_DIR"
}

build_ffi_fixture "ffi-basic"    "ffi_basic"
build_ffi_fixture "ffi-compound" "ffi_compound"
build_ffi_fixture "ffi-errors"   "ffi_errors"

echo "FFI fixtures built. Files in $OUT_DIR:"
ls -la "$OUT_DIR"/ffi_*.* "$OUT_DIR"/uniffi_runtime.* 2>/dev/null || true
