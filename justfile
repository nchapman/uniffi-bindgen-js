set shell := ["bash", "-cu"]

# Show all available recipes.
default:
  @just --list

# ---------- Quick checks ----------

# Run full CI check (format, lint, test).
check: fmt-check lint test

# Check formatting without changing files.
fmt-check:
  cargo fmt --check

# Run clippy across all targets.
lint:
  cargo clippy --all-targets -- -D warnings

# Run workspace tests (unit + golden).
test *args:
  cargo test --workspace {{ args }}

# ---------- Formatting ----------

# Format all Rust code.
fmt:
  cargo fmt --all

# Fix everything auto-fixable, then check what's left.
fix: fmt lint

# ---------- Golden file analysis ----------

# Typecheck golden files with tsc.
typecheck-golden:
  ./scripts/typecheck_golden.sh

# Regenerate all UDL-mode golden files from current generator.
regen-golden:
  #!/usr/bin/env bash
  set -euo pipefail
  for udl in fixtures/*/src/*.udl; do
    dir="$(dirname "$(dirname "$udl")")"
    name="$(basename "$udl" .udl)"
    ns="$(echo "$name" | tr '-' '_')"
    cargo run -- generate "$udl" --out-dir /tmp/regen_golden 2>/dev/null
    [ -f "/tmp/regen_golden/${ns}.ts" ] && cp "/tmp/regen_golden/${ns}.ts" "$dir/expected/${ns}.ts"
  done
  echo "Regenerated all UDL-mode golden files."

# ---------- Library-mode ----------

# Build the library-mode native fixture cdylib.
build-library-fixture:
  ./scripts/build_library_mode_fixture.sh

# Build the library-mode fixture and run its golden test.
test-library:
  #!/usr/bin/env bash
  set -euo pipefail
  LIB_PATH="$(./scripts/build_library_mode_fixture.sh)"
  UBJS_LIBRARY_MODE_LIB="$LIB_PATH" cargo test -p uniffi-bindgen-js golden_library_mode -- --include-ignored

# ---------- FFI-mode (WASM) ----------

# Build all FFI wasm fixtures and run their golden tests.
test-ffi:
  #!/usr/bin/env bash
  set -euo pipefail
  for d in fixtures/ffi-*/wasm; do
    echo "Building $(basename "$(dirname "$d")") wasm..."
    (cd "$d" && cargo build --target wasm32-unknown-unknown --release)
  done
  cargo test -p uniffi-bindgen-js golden_ffi_

# ---------- Full integration ----------

# Build WASM fixtures, generate bindings, and run JS runtime tests.
test-all:
  ./scripts/test_bindings.sh

# Build the workspace.
build:
  cargo build --workspace

# ---------- Code generation ----------

# Generate JS/TS bindings from a UDL file or compiled library.
generate *args:
  cargo run -- generate {{ args }}
