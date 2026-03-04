#!/usr/bin/env bash
# Build generated bindings for all smoke-test fixtures.
#
# Usage: ./scripts/build_bindings.sh
#
# Requires: cargo, wasm-pack, pnpm
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GENERATED_DIR="$REPO_ROOT/binding_tests/generated"

build_fixture() {
  local fixture="$1"
  local namespace="$2"
  local wasm_crate="$REPO_ROOT/fixtures/${fixture}/wasm"

  echo "==> [${fixture}] Compiling WASM..."
  # NOTE: wasm-pack appends its own _bg suffix to --out-name, so the wasm
  # binary will be ${namespace}_bg_bg.wasm (double _bg). This is expected.
  (cd "$wasm_crate" && wasm-pack build --target web --out-name "${namespace}_bg" --out-dir "$GENERATED_DIR")

  echo "==> [${fixture}] Generating TypeScript bindings..."
  cargo run --bin uniffi-bindgen-js -- generate \
    --out-dir "$GENERATED_DIR" \
    "$REPO_ROOT/fixtures/${fixture}/src/${namespace}.udl"
}

build_ffi_fixture() {
  local fixture="$1"
  local namespace="$2"
  local wasm_crate="$REPO_ROOT/fixtures/${fixture}/wasm"

  echo "==> [${fixture}] Compiling WASM (FFI mode)..."
  (cd "$wasm_crate" && RUSTFLAGS="-C link-arg=--export-table -C link-arg=--growable-table" \
    cargo build --target wasm32-unknown-unknown --release)

  echo "==> [${fixture}] Generating TypeScript bindings (FFI mode)..."
  cargo run --bin uniffi-bindgen-js -- generate \
    --wasm "$wasm_crate/target/wasm32-unknown-unknown/release/${namespace}.wasm" \
    --out-dir "$GENERATED_DIR" \
    "$REPO_ROOT/fixtures/${fixture}/src/${namespace}.udl"
}

build_fixture "simple-fns"      "simple_fns"
build_fixture "arithmetic"      "arithmetic"
build_fixture "geometry"        "geometry"
build_fixture "counter"         "counter"
build_fixture "rich-errors"     "rich_errors"
build_fixture "custom-types"    "custom_types"
build_fixture "rename-exclude"  "rename_exclude"
build_fixture "traits"          "traits"
build_fixture "callbacks"       "callbacks"
build_fixture "type-zoo"        "type_zoo"
build_fixture "keywords-demo"   "keywords_demo"

# FFI-mode fixtures (direct WASM, no wasm-pack)
build_ffi_fixture "ffi-basic"      "ffi_basic"
build_ffi_fixture "ffi-compound"   "ffi_compound"
build_ffi_fixture "ffi-errors"     "ffi_errors"
build_ffi_fixture "ffi-callbacks"  "ffi_callbacks"
build_ffi_fixture "ffi-async"      "ffi_async"

echo "==> Installing JS dependencies..."
(cd "$REPO_ROOT/binding_tests" && pnpm install --frozen-lockfile)

echo "Done. Run 'pnpm test' in binding_tests/ to execute the smoke tests."
