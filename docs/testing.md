# Testing

## Quick Reference

```bash
just check            # Full CI: fmt + clippy + test
just typecheck-golden # tsc --noEmit on all golden files
just regen-golden     # Regenerate UDL-mode golden files
just test-ffi         # Build FFI wasm fixtures + run golden tests
```

## Required Gates

- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --workspace`
- `just typecheck-golden`

## Golden Test Pattern

Fixtures live in `fixtures/{name}/` with UDL input at `src/{name}.udl` and expected output at `expected/{name}.ts`. The test harness generates bindings and compares byte-for-byte against expected files. When changing generator output, run `just regen-golden` and update FFI fixtures with `UPDATE_GOLDEN=1 cargo test -p uniffi-bindgen-js golden_ffi_ -- --include-ignored`.

Additionally, `scripts/typecheck_golden.sh` runs `tsc --noEmit` on all golden files with auto-generated `.d.ts` stubs, catching type errors that byte-for-byte comparison alone would miss.

## Policy

Use TDD: add failing tests before implementation.
