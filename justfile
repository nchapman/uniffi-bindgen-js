set shell := ["bash", "-cu"]

# Show all available recipes.
default:
  @just --list

# ---------- Quick checks ----------

# Run full CI check (format, lint, test).
# If wasm32-unknown-unknown is installed, also runs FFI golden tests.
check: fmt-check lint test _check-ffi

# Check formatting without changing files.
fmt-check:
  cargo fmt --check

# Run clippy across all targets.
lint:
  cargo clippy --all-targets -- -D warnings

# Run workspace tests (unit + golden).
test *args:
  cargo test --workspace {{ args }}

# Build FFI wasm fixtures and run golden tests if wasm32 target is installed.
_check-ffi:
  #!/usr/bin/env bash
  if rustup target list --installed 2>/dev/null | grep -q wasm32-unknown-unknown; then
    echo "wasm32-unknown-unknown target found — running FFI golden tests..."
    just test-ffi
  else
    echo "NOTE: wasm32-unknown-unknown not installed — skipping FFI golden tests."
    echo "      Install with: rustup target add wasm32-unknown-unknown"
  fi

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
    config_flag=""
    if [ -f "$dir/src/uniffi.toml" ]; then
      config_flag="--config $dir/src/uniffi.toml"
    fi
    cargo run -- generate $config_flag "$udl" --out-dir /tmp/regen_golden 2>/dev/null
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
