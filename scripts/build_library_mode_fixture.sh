#!/usr/bin/env bash
set -euo pipefail

# Build the library-mode native fixture and print the path to the compiled cdylib.

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
FIXTURE_DIR="$REPO_ROOT/fixtures/library-mode/native-lib"
TARGET_DIR="$REPO_ROOT/target/library-mode-native"

cargo build --manifest-path "$FIXTURE_DIR/Cargo.toml" --release --target-dir "$TARGET_DIR" >&2

# Detect platform-specific library extension
case "$(uname -s)" in
  Darwin) EXT="dylib" ;;
  Linux)  EXT="so" ;;
  MINGW*|MSYS*|CYGWIN*) EXT="dll" ;;
  *) echo "Unsupported platform: $(uname -s)" >&2; exit 1 ;;
esac

LIB_PATH="$TARGET_DIR/release/liblibrary_mode_fixture.${EXT}"
if [ ! -f "$LIB_PATH" ]; then
  echo "ERROR: Expected library not found at $LIB_PATH" >&2
  exit 1
fi

echo "$LIB_PATH"
