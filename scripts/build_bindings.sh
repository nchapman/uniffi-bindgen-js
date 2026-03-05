#!/usr/bin/env bash
# Build generated bindings for all smoke-test fixtures.
#
# Usage: ./scripts/build_bindings.sh
#
# Requires: cargo, pnpm, wasm32-unknown-unknown target
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
GENERATED_DIR="$REPO_ROOT/binding_tests/generated"

build_fixture() {
  local fixture="$1"
  local namespace="$2"
  local wasm_crate="$REPO_ROOT/fixtures/${fixture}/wasm"

  echo "==> [${fixture}] Compiling WASM..."
  (cd "$wasm_crate" && RUSTFLAGS="-C link-arg=--export-table -C link-arg=--growable-table" \
    cargo build --target wasm32-unknown-unknown --release)

  echo "==> [${fixture}] Generating TypeScript bindings..."
  cargo run --bin uniffi-bindgen-js -- generate \
    --out-dir "$GENERATED_DIR" \
    "$wasm_crate/target/wasm32-unknown-unknown/release/${namespace}.wasm"
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
build_fixture "ffi-basic"       "ffi_basic"
build_fixture "ffi-compound"    "ffi_compound"
build_fixture "ffi-errors"      "ffi_errors"
build_fixture "ffi-callbacks"   "ffi_callbacks"
build_fixture "ffi-async"       "ffi_async"
build_fixture "ffi-traits"      "ffi_traits"
build_fixture "ffi-features"    "ffi_features"

echo "==> Installing JS dependencies..."
(cd "$REPO_ROOT/binding_tests" && pnpm install --frozen-lockfile)

echo "Done. Run 'pnpm test' in binding_tests/ to execute the smoke tests."
