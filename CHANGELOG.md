# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.2.0] - 2026-03-07

### Added

- **FFI-direct code generation** — generated TypeScript now calls UniFFI FFI buffer functions directly via `(argPtr, retPtr)` convention. No wasm-pack or wasm-bindgen required.
- **WASM-as-source mode** — pass a `.wasm` file directly to extract metadata and copy it to the output directory
- **Library mode** — pass a `.dylib`/`.so`/`.dll` to extract metadata from a compiled cdylib
- Callback interface support with VTable FFI glue and typed WASM trampolines
- Async function/method support via RustFuture polling
- Trait interface support with object return lifting
- Object handle cloning for method calls and compound type serialization
- Complete primitive type support: bool, integers, floats, string, bytes, void
- Record, flat enum, and data-carrying enum types with serialization
- Duration and Timestamp type conversions
- Non-exhaustive enum support with catch-all variants
- Enum constructor and method bindings with companion namespaces
- External type imports from cross-package UDL references
- Custom type aliases with configurable lift/lower templates
- Docstring emission as JSDoc comments from Rust `///` doc comments
- JS/TS reserved-word escaping in generated identifiers
- Default value rendering for function parameters and record fields
- Rename and exclude config for generated public API surface
- Rich error classes with human-readable `.message` built from variant fields, `Error.cause` for error chain compatibility, and structured `.variant` for programmatic matching
- Golden test harness with 28 fixture suites for deterministic output
- TypeScript typechecking of all golden files via `tsc --noEmit`
- Runtime integration tests via native-lib builds
- CI pipeline with fmt, clippy, golden tests, FFI fixture builds, library-mode tests, and runtime validation

### Changed

- **Breaking:** Removed legacy wasm-pack wrapper mode — FFI-direct is now the only output format
- Modularized `js/mod.rs` into 8 focused sub-modules (ffi, render_types, render_helpers, types, config, parsing, wasm_metadata, runtime_ts)
- Namespace auto-derivation uses PascalCase of the crate/UDL namespace name

### Fixed

- Custom type serialization inside compound types (Optional, Sequence, Map) now correctly applies lift/lower config
- Scratch allocator exhaustion for large data payloads
- Scratch memory leak on error paths (try/finally)
- Rich error field deserialization order
- Rename escaping and type-limits codec wiring
- Buffer safety and double-parse issues in generator
- `Sequence<u64>` type annotation (was `number`, now `bigint`)
- `instantiateStreaming` fallback: clone Response before consuming

[Unreleased]: https://github.com/aspect-build/uniffi-bindgen-js/compare/v0.2.0...HEAD
[0.2.0]: https://github.com/aspect-build/uniffi-bindgen-js/compare/v0.1.2...v0.2.0
