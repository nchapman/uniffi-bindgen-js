# uniffi-bindgen-js

[![CI](https://github.com/aspect-build/uniffi-bindgen-js/actions/workflows/ci.yml/badge.svg)](https://github.com/aspect-build/uniffi-bindgen-js/actions/workflows/ci.yml)
[![License: MPL-2.0](https://img.shields.io/badge/License-MPL_2.0-blue.svg)](https://opensource.org/licenses/MPL-2.0)

Generate idiomatic [TypeScript](https://www.typescriptlang.org/) bindings for [UniFFI](https://github.com/mozilla/uniffi-rs) components, targeting WebAssembly via [wasm-pack](https://rustwasm.github.io/wasm-bindgen/).

`uniffi-bindgen-js` is a third-party bindings generator that produces
production-grade TypeScript wrappers from UniFFI interface definitions. It
targets `uniffi-rs` version **0.31.0** and works in any JavaScript runtime
that supports WebAssembly (browsers, Node.js, Deno, Bun).

## Features

- All UniFFI primitives, strings, bytes, timestamps (`Date`), and durations
- Records as TypeScript `interface` types with optional field defaults
- Flat enums (string literal unions) and data-carrying enums (discriminated unions)
- Objects with constructors, methods, `free()` lifecycle, and `Symbol.dispose` support
- Flat and rich error classes via `[Error]` and `[Throws]`
- Async functions and methods mapped to `Promise<T>`
- Callback interfaces as TypeScript `interface` declarations
- Trait interfaces with `_fromInner` lifting
- Custom type aliases and external/remote type imports
- Rename, exclude, and docstring (JSDoc) support
- Enum methods and enum discriminant annotations

## Install

Requires Rust 1.75 or later.

```bash
cargo install --git https://github.com/aspect-build/uniffi-bindgen-js
```

Or build from source:

```bash
git clone https://github.com/aspect-build/uniffi-bindgen-js
cd uniffi-bindgen-js
cargo build --release
```

## Usage

Generate bindings from a UDL file:

```bash
uniffi-bindgen-js generate path/to/definitions.udl --out-dir out/
```

Generate from a compiled library (library mode):

```bash
uniffi-bindgen-js generate path/to/libmycrate.so --library --out-dir out/
```

### CLI flags

| Flag | Description |
|---|---|
| `--out-dir <dir>` | Output directory for generated TypeScript files |
| `--library` | Treat source as a compiled cdylib (library mode) |
| `--config <file>` | Path to `uniffi.toml` configuration |
| `--crate <name>` | In library mode, generate bindings for this crate only |

### How it works

Generated TypeScript wraps **wasm-pack** output. Your Rust crate is compiled
to WebAssembly with `wasm-pack build --target web`, and the generated `.ts`
file imports and re-exports the WASM module:

```typescript
import __init, * as __bg from './my_crate_bg.js';
export { __init as init };

export namespace MyCrate {
  export function greet(name: string): string {
    return __bg.greet(name);
  }
}
```

Consumers call `await init()` once before using the namespace. This works
universally across browsers, Node.js, Deno, and Bun.

## Configuration

Place a `[bindings.js]` section in your `uniffi.toml`:

```toml
[bindings.js]
module_name = "MyBindings"
rename = { add_numbers = "sumValues", "Counter.currentValue" = "getValue" }
exclude = ["internal_helper"]
external_packages = { other_crate = "./other_bindings.js" }
```

See [docs/configuration.md](docs/configuration.md) for the full reference.

## Compatibility

| uniffi-bindgen-js | uniffi-rs |
|---|---|
| 0.1.x | 0.31.0 |

## Feature status

See [docs/supported-features.md](docs/supported-features.md) for a detailed
feature parity matrix.

## Development

```bash
# Run all Rust tests (unit + golden)
cargo test --workspace

# Build WASM fixtures, generate bindings, and run JS runtime tests
./scripts/test_bindings.sh
```

See [docs/testing.md](docs/testing.md) for the full test workflow.

## Documentation

- [Supported features](docs/supported-features.md)
- [Configuration reference](docs/configuration.md)
- [Testing guide](docs/testing.md)
- [Release process](docs/release.md)

## License

MPL-2.0
