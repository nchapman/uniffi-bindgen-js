# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

### Added

- Full UDL-to-TypeScript code generation targeting WASM via `wasm-pack`
- Complete primitive type support: bool, integers, floats, string, bytes, void
- Record, flat enum, and data-carrying enum types with serialization
- Typed error mapping for UDL `[Throws]` declarations
- Object lifecycle bindings with `Symbol.dispose` for automatic cleanup
- Async support via Rust futures
- Callback interface support
- Trait interface support with object return lifting
- External type imports from cross-package UDL references
- Custom type aliases and rename/exclude config
- Docstring emission as JSDoc comments from UDL `///` comments
- JS/TS reserved-word escaping in generated identifiers
- Default value rendering for function and method parameters
- Duration and Timestamp type conversions
- Non-exhaustive enum support and recursive lifting
- Library-mode metadata parsing
- Enum constructor and method bindings
- Golden test harness with fixture suites for deterministic output
- Runtime integration tests via native-lib builds
- CI pipeline with build, golden tests, and runtime validation

### Changed

- Modularized `js/mod.rs` into 8 focused sub-modules

### Fixed

- Rich error field deserialization
- Rename escaping and type-limits codec wiring
- Buffer safety and double-parse issues in generator

[Unreleased]: https://github.com/aspect-build/uniffi-bindgen-js/compare/v0.1.0...HEAD
