# Testing

## Required Gates
- `cargo fmt --check`
- `cargo clippy --all-targets -- -D warnings`
- `cargo test --workspace`
- `./scripts/test_bindings.sh`

## Policy
Use TDD: add failing runtime + generator tests before implementation.
