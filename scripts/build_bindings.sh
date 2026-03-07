#!/usr/bin/env bash
# Build generated bindings for all smoke-test fixtures.
#
# Usage: ./scripts/build_bindings.sh
#
# Requires: cargo, wasm-pack, pnpm, wasm32-unknown-unknown target
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GENERATED_DIR="$REPO_ROOT/binding_tests/generated"

# UDL fixtures: use wasm-pack to build WASM, then generate from UDL.
build_udl_fixture() {
  local fixture="$1"
  local namespace="$2"
  local wasm_crate="$REPO_ROOT/fixtures/${fixture}/wasm"

  echo "==> [${fixture}] Compiling WASM (wasm-pack)..."
  (cd "$wasm_crate" && wasm-pack build --target web --out-name "${namespace}_bg" --out-dir "$GENERATED_DIR")

  echo "==> [${fixture}] Generating TypeScript bindings (UDL)..."
  cargo run -p uniffi-bindgen-js -- generate \
    --out-dir "$GENERATED_DIR" \
    "$REPO_ROOT/fixtures/${fixture}/src/${namespace}.udl"
}

# FFI fixtures: cargo build to WASM, then generate from compiled .wasm.
build_ffi_fixture() {
  local fixture="$1"
  local namespace="$2"
  local wasm_crate="$REPO_ROOT/fixtures/${fixture}/wasm"

  echo "==> [${fixture}] Compiling WASM (FFI)..."
  (cd "$wasm_crate" && RUSTFLAGS="-C link-arg=--export-table -C link-arg=--growable-table" \
    cargo build --target wasm32-unknown-unknown --release)

  echo "==> [${fixture}] Generating TypeScript bindings (WASM)..."
  cargo run -p uniffi-bindgen-js -- generate \
    --out-dir "$GENERATED_DIR" \
    "$wasm_crate/target/wasm32-unknown-unknown/release/${namespace}.wasm"
}

# --- UDL fixtures (wasm-bindgen crates) ---
build_udl_fixture "simple-fns"      "simple_fns"
build_udl_fixture "arithmetic"      "arithmetic"
build_udl_fixture "geometry"        "geometry"
build_udl_fixture "counter"         "counter"
build_udl_fixture "rich-errors"     "rich_errors"
build_udl_fixture "custom-types"    "custom_types"
build_udl_fixture "rename-exclude"  "rename_exclude"
build_udl_fixture "traits"          "traits"
build_udl_fixture "callbacks"       "callbacks"
build_udl_fixture "type-zoo"        "type_zoo"
build_udl_fixture "keywords-demo"   "keywords_demo"

# --- FFI fixtures (UniFFI crates) ---
build_ffi_fixture "ffi-basic"       "ffi_basic"
build_ffi_fixture "ffi-compound"    "ffi_compound"
build_ffi_fixture "ffi-errors"      "ffi_errors"
build_ffi_fixture "ffi-callbacks"   "ffi_callbacks"
build_ffi_fixture "ffi-async"       "ffi_async"
build_ffi_fixture "ffi-traits"      "ffi_traits"
build_ffi_fixture "ffi-features"    "ffi_features"

echo "==> Installing JS dependencies..."
(cd "$REPO_ROOT/binding_tests" && pnpm install --frozen-lockfile)

echo "Done. Run 'pnpm test' in binding_tests/ to execute the smoke tests."
