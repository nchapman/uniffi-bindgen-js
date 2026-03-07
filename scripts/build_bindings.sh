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
  cargo run -p uniffi-bindgen-js -- generate \
    --out-dir "$GENERATED_DIR" \
    "$wasm_crate/target/wasm32-unknown-unknown/release/${namespace}.wasm"
}

build_fixture_with_config() {
  local fixture="$1"
  local namespace="$2"
  local config="$3"
  local wasm_crate="$REPO_ROOT/fixtures/${fixture}/wasm"

  echo "==> [${fixture}] Compiling WASM..."
  (cd "$wasm_crate" && RUSTFLAGS="-C link-arg=--export-table -C link-arg=--growable-table" \
    cargo build --target wasm32-unknown-unknown --release)

  echo "==> [${fixture}] Generating TypeScript bindings..."
  cargo run -p uniffi-bindgen-js -- generate \
    --config "$config" \
    --out-dir "$GENERATED_DIR" \
    "$wasm_crate/target/wasm32-unknown-unknown/release/${namespace}.wasm"
}

build_fixture "ffi-basic"       "ffi_basic"
build_fixture "ffi-compound"    "ffi_compound"
build_fixture "ffi-errors"      "ffi_errors"
build_fixture "ffi-callbacks"   "ffi_callbacks"
build_fixture "ffi-async"       "ffi_async"
build_fixture "ffi-traits"      "ffi_traits"
build_fixture "ffi-features"    "ffi_features"
build_fixture_with_config "ffi-custom-types"   "ffi_custom_types" "$REPO_ROOT/fixtures/ffi-custom-types/wasm/uniffi.toml"
build_fixture_with_config "ffi-rename-exclude"  "ffi_rename_exclude" "$REPO_ROOT/fixtures/ffi-rename-exclude/wasm/uniffi.toml"

echo "==> Installing JS dependencies..."
(cd "$REPO_ROOT/binding_tests" && pnpm install --frozen-lockfile)

echo "Done. Run 'pnpm test' in binding_tests/ to execute the smoke tests."
