# Supported Features

This document tracks implemented UniFFI feature parity for JavaScript/TypeScript.

## Status Snapshot
Legend:
- `Implemented`: available with tests.
- `Partial`: some paths implemented, parity still incomplete.
- `Planned`: not implemented yet.

| Area | Status | Notes |
|---|---|---|
| Top-level functions | Implemented | sync and async; primitives, temporal, bytes, records/enums, typed throws envelopes, and default-argument rendering; optional parameters with defaults supported |
| Objects/interfaces | Implemented | constructors (sync + async + named), methods, `free()` lifecycle with `_freed` guard and `_assertLive()` safety, `Symbol.dispose` for resource management (guarded for pre-ES2025); FinalizationRegistry safety net for leaked handles; double-free safe; forward declarations (mutual references) supported |
| Trait interfaces (`[Trait]`) | Implemented | `_fromHandle` object return lifting; `Optional<Object>` and `Sequence<Object>` handled |
| Records | Implemented | TypeScript `interface` types with field defaults (`?:` syntax) |
| Enums | Implemented | flat enums as string literal unions; data-carrying enums as discriminated unions with `tag` field; enum methods via companion namespace; enum discriminant values via companion `Values` const |
| Errors (`[Error]` + `[Throws]`) | Implemented | flat error classes with `tag` property; rich error classes with discriminated `variant` property and static factory methods; object-as-error (`[Throws=Interface]`) supported; non-exhaustive error support |
| Optionals/sequences/maps | Implemented | `T \| null` for optionals, `T[]` for sequences, `Map<K, V>` for maps |
| Builtins | Implemented | int/float/bool/string/bytes (`Uint8Array`)/timestamp (`Date`)/duration (`number` ms) |
| Async futures | Implemented | `[Async]` maps to `Promise<T>` APIs; async functions, methods, and constructors (`static async`) all supported; `[Async, Throws=X]` fully tested |
| Callback interfaces | Implemented | VTable-based FFI glue using `WebAssembly.Function` trampolines; JS objects passed as Rust trait implementations |
| Custom types | Implemented | `[Custom] typedef` → `export type Alias = builtin`; rename-aware |
| External/remote types | Implemented | `[External="crate"]` types detected by module_path; named imports from `external_packages` config; error on missing config entries; deduplication across usages |
| Rename/exclude/docstrings | Implemented | `rename`/`exclude` config keys for per-function, per-type, and per-method API names; JSDoc emission on all generated symbols with flat-enum variant docs folded into parent type |
| Library-mode metadata input | Implemented | `generate --library <cdylib>` parses UniFFI metadata from library artifacts with optional crate selection via `--crate` |
| Record field defaults | Implemented | `Field::default_value()` parsed; `?:` on fields with defaults |
| Async constructors | Implemented | `Constructor::is_async()` → `static async` returning `Promise<ClassName>` |
| Object lifecycle safety | Implemented | `_freed` flag + `_assertLive()` guard on all methods; FinalizationRegistry as safety net; `Error('{ClassName} object has been freed')` on destroyed access |
| Non-exhaustive enums | Implemented | `[NonExhaustive]` flat enums include `string` catchall type; data-carrying enums include unknown variant |
| ABI integrity checks | Implemented | contract version and per-function checksum verification at WASM module init; mismatches throw clear diagnostic errors |

## Known Limitations
- **Callback/trait interfaces require `WebAssembly.Function`**: The Type Reflection proposal is needed for typed WASM trampolines. Supported in V8 (Chrome, Node.js 22+), SpiderMonkey (Firefox), and Safari 18.2+.
- **Traits on records/enums**: `[Traits=(Display, Eq, Hash)]` on dictionary/enum types is parsed without error but has no TypeScript equivalent (structural equality and string unions provide this natively).

## Notes
- Current fixture coverage includes 28 golden tests across all major feature domains, anchored by `coverall-demo` (comprehensive feature combinations).
- Strict hygiene gate includes `cargo clippy --all-targets -- -D warnings`, `cargo fmt --check`, and full test suite via `just check`.
