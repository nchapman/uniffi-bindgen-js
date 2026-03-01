# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## Project Overview

uniffi-bindgen-js is a JavaScript/TypeScript backend for UniFFI — it generates idiomatic JS/TS bindings from UDL definitions. The project is in early development (Phase 0 complete, scaffold in place). Built in Rust, it outputs TypeScript code.

## Build & Test Commands

```bash
# Build
cargo build --workspace

# Run all tests (3 currently: unit, golden, harness)
cargo test --workspace

# Full CI check (run before committing)
cargo fmt --check && cargo clippy --all-targets -- -D warnings && cargo test --workspace

# Generate bindings from a UDL file
cargo run --bin uniffi-bindgen-js -- generate fixtures/simple/src/simple.udl --out-dir /tmp/out

# Run a single test
cargo test -p ubjs_bindgen golden_simple_fixture
cargo test -p ubjs_bindgen pascal_case
```

## Architecture

### Crate Layout

- **`crates/ubjs_bindgen`** — Main code generator. CLI entrypoint, config parsing, JS/TS output.
- **`crates/ubjs_runtime`** — Runtime support library (scaffold, not yet implemented).
- **`crates/ubjs_testing`** — Test utilities (scaffold, not yet implemented).

### Generator Pipeline

```
main.rs → cli.rs (Clap) → lib.rs::run() → js/mod.rs::generate_bindings()
```

`js/mod.rs` is the core generation module. It resolves config from an optional TOML file (`[bindings.js]` section), derives the module name (PascalCase from UDL filename), and renders TypeScript output.

### Golden Test Pattern

Fixtures live in `fixtures/{name}/` with UDL input at `src/{name}.udl` and expected output at `expected/{name}.ts`. The test harness (`crates/ubjs_bindgen/tests/golden_generated.rs`) generates bindings and compares byte-for-byte against expected files. When changing generator output, update the corresponding expected files.

### Naming Conventions

- Generated module names use PascalCase (e.g., `simple` → `Simple`)
- Output files are lowercase `.ts` (e.g., `simple.ts`)
- Crate names prefixed with `ubjs_`

## Development Methodology

- **TDD-first**: Write failing tests before implementation for each UDL feature.
- **Idiomatic output**: Generated code must look like hand-written JS/TS, not Rust transliterations. It should pass Prettier and the TypeScript compiler.
- **Deterministic generation**: Golden tests enforce reproducible output.
- **Linear history**: Keep commits as coherent units with descriptive messages in imperative mood.

## Reference Repositories

These external repos are used for correctness verification:

- **uniffi-rs** (`/Users/nchapman/Drive/Code/lessisbetter/refs/uniffi-rs`) — Canonical UniFFI semantics and upstream behavior.
- **uniffi-bindgen-dart** (`/Users/nchapman/Drive/Code/uniffi-bindgen-dart`) — Template for generator structure, testing workflow, and implementation sequencing.

## Key Files

| Purpose | Path |
|---------|------|
| JS generation logic | `crates/ubjs_bindgen/src/js/mod.rs` |
| CLI definition | `crates/ubjs_bindgen/src/cli.rs` |
| Golden test harness | `crates/ubjs_bindgen/tests/golden_generated.rs` |
| Test fixture (simple) | `fixtures/simple/` |
| Implementation plan | `PLAN.md` |
| Phase template | `PLAN_TEMPLATE.md` |
