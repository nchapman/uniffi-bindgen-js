# Contributing to uniffi-bindgen-js

Thank you for your interest in contributing! This document covers the essentials.

## Prerequisites

- **Rust** (stable toolchain) with `clippy`, `rustfmt`, and `wasm32-unknown-unknown` target
- **wasm-pack** (`curl https://rustwasm.github.io/wasm-pack/installer/init.sh -sSf | sh`)
- **Node.js 22+** and **pnpm 9+** (for running JS smoke tests)

## Development Workflow

```bash
# Clone and build
git clone https://github.com/aspect-build/uniffi-bindgen-js.git
cd uniffi-bindgen-js
cargo build --workspace

# Run the full CI gate (format + lint + tests)
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace

# Build WASM fixtures + generate TS bindings + run JS smoke tests
./scripts/test_bindings.sh
```

## Project Structure

| Directory | Purpose |
|-----------|---------|
| `crates/ubjs_bindgen/` | Code generator (CLI + TypeScript output) |
| `crates/ubjs_runtime/` | Runtime support library |
| `crates/ubjs_testing/` | Shared test helpers |
| `fixtures/` | UDL fixtures with expected golden output |
| `binding_tests/` | JS/TS runtime smoke tests (vitest) |
| `scripts/` | Build and test automation |

## Adding a New UDL Feature

1. Create a fixture in `fixtures/{name}/src/{name}.udl` (snake_case filename)
2. Generate the golden output:
   ```bash
   cargo run --bin uniffi-bindgen-js -- generate fixtures/{name}/src/{name}.udl --out-dir /tmp/gen
   cp /tmp/gen/{namespace}.ts fixtures/{name}/expected/{namespace}.ts
   ```
3. Add a golden test to `crates/ubjs_bindgen/tests/golden_generated.rs`
4. Run the full test suite to verify

## Golden Test Pattern

Golden tests compare generated TypeScript output byte-for-byte against expected files. When modifying the generator:

1. Make your code changes in `crates/ubjs_bindgen/src/js/mod.rs`
2. Regenerate affected golden files (see step 2 above)
3. Review the diff to ensure changes are intentional
4. Commit both the code change and updated golden files together

To auto-update all golden files at once:
```bash
UPDATE_GOLDEN=1 cargo test -p ubjs_bindgen --test golden_generated
```

## Code Style

- Run `cargo fmt` before committing
- All code must pass `cargo clippy --all-targets -- -D warnings`
- Follow existing patterns in the codebase
- Commit messages use imperative mood ("Add feature" not "Added feature")

## Reporting Issues

Open an issue at [github.com/aspect-build/uniffi-bindgen-js/issues](https://github.com/aspect-build/uniffi-bindgen-js/issues) with:
- UniFFI version and Rust toolchain version
- Minimal UDL that reproduces the problem
- Expected vs actual generated output
