#!/usr/bin/env bash
# Build generated bindings for the smoke-test fixture (simple-fns).
#
# Usage: ./scripts/build_bindings.sh
#
# Requires: cargo, wasm-pack, pnpm
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
WASM_CRATE="$REPO_ROOT/fixtures/simple-fns/wasm"
GENERATED_DIR="$REPO_ROOT/binding_tests/generated"
NAMESPACE="simple_fns"

echo "==> Compiling WASM fixture..."
(cd "$WASM_CRATE" && wasm-pack build --target web --out-name "${NAMESPACE}_bg" --out-dir "$GENERATED_DIR")

echo "==> Generating TypeScript bindings..."
cargo run --bin uniffi-bindgen-js -- generate \
  --out-dir "$GENERATED_DIR" \
  "$REPO_ROOT/fixtures/simple-fns/src/${NAMESPACE}.udl"

echo "==> Installing JS dependencies..."
(cd "$REPO_ROOT/binding_tests" && pnpm install)

echo "Done. Run 'pnpm test' in binding_tests/ to execute the smoke tests."
