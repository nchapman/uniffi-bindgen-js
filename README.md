# uniffi-bindgen-js

JavaScript/TypeScript backend scaffold for UniFFI bindings generation.

## Current Status
- Scaffold only: CLI, config shape, deterministic golden test harness, and project policy docs.
- Production feature implementation is tracked in `PLAN.md`.

## Quick Start
```bash
cargo run --bin uniffi-bindgen-js -- generate fixtures/simple/src/simple.udl --out-dir /tmp/uniffi-js
cargo test --workspace
./scripts/test_bindings.sh
```
