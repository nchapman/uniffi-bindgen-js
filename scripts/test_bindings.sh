#!/usr/bin/env bash
# Run all tests: Rust (golden + unit) and JS smoke tests.
#
# Usage: ./scripts/test_bindings.sh
#
# Requires: cargo, pnpm, wasm32-unknown-unknown target
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"

echo "==> Building bindings..."
"$REPO_ROOT/scripts/build_bindings.sh"

echo "==> Running Rust tests..."
cargo test --workspace

echo "==> Typechecking golden files..."
"$REPO_ROOT/scripts/typecheck_golden.sh"

echo "==> Running JS smoke tests..."
(cd "$REPO_ROOT/binding_tests" && pnpm test)
