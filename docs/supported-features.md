# Supported Features

This document tracks implemented UniFFI feature parity for JavaScript/TypeScript.

## Status Snapshot
Legend:
- `Implemented`: available in current prototype with tests.
- `Partial`: some paths implemented, parity still incomplete.
- `Planned`: not implemented yet.

| Area | Status | Notes |
|---|---|---|
| Top-level functions | Implemented | sync and async; primitives, temporal, bytes, records/enums, typed throws envelopes, and metadata-backed default-argument rendering in generated TypeScript wrapper signatures |
| Objects/interfaces | Implemented | constructors (sync + async + named), methods, `free()` lifecycle with `_freed` guard and `_assertLive()` safety, `Symbol.dispose` for resource management; double-free safe |
| Trait interfaces (`[Trait]`) | Implemented | `_fromInner` object return lifting; `Optional<Object>` and `Sequence<Object>` handled; trait vtable FFI glue is N/A for WASM target |
| Records | Implemented | TypeScript `interface` types with field defaults (`?:` syntax); serde-wasm-bindgen pass-through for runtime marshalling |
| Enums | Implemented | flat enums as string literal unions; data-carrying enums as discriminated unions with `tag` field; enum methods via companion namespace; enum discriminant values in JSDoc |
| Errors (`[Error]` + `[Throws]`) | Implemented | flat error classes with `tag` property; rich error classes with discriminated `variant` property and static factory methods; error lifting via `_liftErrorName()` JSON deserialization; non-exhaustive error support |
| Optionals/sequences/maps | Implemented | `T \| null` for optionals, `T[]` for sequences, `Map<K, V>` for maps |
| Builtins | Implemented | int/float/bool/string/bytes (`Uint8Array`)/timestamp (`Date`)/duration (`number` ms) |
| Async futures | Implemented | `[Async]` maps to `Promise<T>` APIs; async functions, methods, and constructors (`static async`) all supported |
| Callback interfaces | Partial | `export interface` declarations with camelCase methods and `[Async]` → `Promise<T>` support; callback/trait vtable FFI glue (JS objects as Rust trait impls) is N/A for WASM target |
| Custom types | Implemented | `[Custom] typedef` → `export type Alias = builtin`; rename-aware |
| External/remote types | Implemented | `[External="crate"]` types detected by module_path; named imports from `external_packages` config; error on missing config entries; deduplication across usages; `Optional<ExternalType>` and `Sequence<ExternalType>` coverage |
| Rename/exclude/docstrings | Implemented | `rename`/`exclude` config keys for per-function, per-type, and per-method API names with dedicated `rename-exclude` golden coverage; JSDoc emission on all generated symbols with flat-enum variant docs folded into parent type |
| Library-mode metadata input | Implemented | `generate --library <cdylib>` parses UniFFI metadata from library artifacts with optional crate selection via `--crate` |
| Record field defaults | Implemented | `Field::default_value()` parsed; `?:` on fields with defaults |
| Async constructors | Implemented | `Constructor::is_async()` → `static async` returning `Promise<ClassName>` |
| Object lifecycle safety | Implemented | `_freed` flag + `_assertLive()` guard on all methods; `Error('{ClassName} object has been freed')` on destroyed access |

## Notes
- Current fixture coverage includes 14 golden tests and 9 JS smoke test files across all major feature domains.
- Strict hygiene gate includes `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and full `./scripts/test_bindings.sh`.
- WASM/wasm-pack architecture means some UniFFI features that require native FFI scaffolding (trait vtable glue, object equality/hashing via Rust traits) are not applicable.
