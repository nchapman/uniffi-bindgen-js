# uniffi-bindgen-js

Call Rust code from JavaScript and TypeScript.

[![Crates.io](https://img.shields.io/crates/v/uniffi-bindgen-js)](https://crates.io/crates/uniffi-bindgen-js)
[![CI](https://github.com/nchapman/uniffi-bindgen-js/actions/workflows/ci.yml/badge.svg)](https://github.com/nchapman/uniffi-bindgen-js/actions/workflows/ci.yml)
[![License: MIT](https://img.shields.io/badge/License-MIT-blue.svg)](https://opensource.org/licenses/MIT)

`uniffi-bindgen-js` generates idiomatic TypeScript bindings from [UniFFI](https://mozilla.github.io/uniffi-rs/) interface definitions. Define your API once in Rust, compile to WebAssembly, and get typed, documented TypeScript that works in browsers, Node.js, Deno, and Bun.

## Quickstart

**1. Define your interface** in a UDL file (`src/math.udl`):

```webidl
namespace math {
  u32 add(u32 left, u32 right);
  string greet(string name);
};
```

**2. Implement it in Rust** (`src/lib.rs`):

```rust
pub fn add(left: u32, right: u32) -> u32 {
    left + right
}

pub fn greet(name: String) -> String {
    format!("Hello, {name}!")
}
```

Configure for UniFFI + WASM:

```toml
# Cargo.toml
[lib]
crate-type = ["cdylib"]

[dependencies]
uniffi = { version = "0.31", features = ["scaffolding-ffi-buffer-fns", "wasm-unstable-single-threaded"] }
```

**3. Build** the WASM module:

```bash
cargo build --target wasm32-unknown-unknown --release
```

**4. Generate TypeScript bindings:**

```bash
uniffi-bindgen-js generate target/wasm32-unknown-unknown/release/math.wasm --out-dir pkg/

# With configuration (custom namespace, rename/exclude, etc.):
uniffi-bindgen-js generate target/wasm32-unknown-unknown/release/math.wasm --out-dir pkg/ --config uniffi.toml
```

**5. Use it:**

```typescript
import { Math } from './pkg/math.js';

console.log(Math.add(2, 3));        // 5
console.log(Math.greet('World'));    // "Hello, World!"
```

The generator reads your compiled WASM binary (or UDL file) and emits TypeScript that calls UniFFI FFI functions directly — no wasm-pack or wasm-bindgen required. The `.wasm` file is loaded automatically from the same directory using `import.meta.url`.

## Install

Requires [Rust](https://rustup.rs/).

```bash
cargo install uniffi-bindgen-js
```

Or build from source:

```bash
git clone https://github.com/nchapman/uniffi-bindgen-js
cd uniffi-bindgen-js
cargo build --release
```

## What it generates

Generated TypeScript is designed to look like something you would write by hand. Exported names use camelCase; internal FFI calls retain the original Rust snake_case names.

### Top-level functions

UDL:

```webidl
namespace math {
  u32 add(u32 left, u32 right);
  string greet(string name);
};
```

Generated TypeScript:

```typescript
export namespace Math {
  export function add(left: number, right: number): number { /* FFI call */ }
  export function greet(name: string): string { /* FFI call */ }
}
```

Top-level functions are grouped into a namespace named after the UDL file (PascalCase).

### Objects

UDL:

```webidl
interface Counter {
  constructor(i64 start);
  void increment();
  i64 get();
};
```

Generated TypeScript:

```typescript
export class Counter {
  private _freed = false;
  private _assertLive(): void {
    if (this._freed) throw new Error('Counter object has been freed');
  }
  static create(start: bigint): Counter { /* FFI call */ }
  increment(): void { this._assertLive(); /* FFI call */ }
  get(): bigint { this._assertLive(); /* FFI call */ }
  /** Releases the underlying WASM resource. Safe to call more than once. */
  free(): void {
    if (this._freed) return;
    this._freed = true;
    _rt.unregisterPointer(this);
    _rt.callFree('uniffi_counter_fn_free_counter', this._handle);
  }
}
if (Symbol.dispose) (Counter as any).prototype[Symbol.dispose] = Counter.prototype.free;
```

Objects are wrapped in lifecycle-safe classes with `FinalizationRegistry` support, `free()` for deterministic cleanup, `Symbol.dispose` for `using` declarations, and guards against use-after-free.

### Records

UDL:

```webidl
dictionary Point {
  f64 x;
  f64 y;
};
```

Generated TypeScript:

```typescript
export interface Point {
  x: number;
  y: number;
}
```

### Enums

UDL:

```webidl
enum Direction { "North", "South", "East", "West" };

[Enum]
interface Shape {
  Circle(f64 radius);
  Rectangle(f64 width, f64 height);
  Point();
};
```

Generated TypeScript:

```typescript
export type Direction = 'North' | 'South' | 'East' | 'West';

export type Shape =
  | { tag: 'Circle', radius: number }
  | { tag: 'Rectangle', width: number, height: number }
  | { tag: 'Point' };
```

Flat enums map to string literal unions; data-carrying enums map to discriminated unions with exhaustive pattern matching.

### Errors

Rust:

```rust
#[derive(Debug, thiserror::Error, uniffi::Error)]
pub enum NetworkError {
    #[error("Resource not found: {url}")]
    NotFound { url: String },
    #[error("Request timed out after {timeout_ms}ms")]
    Timeout { timeout_ms: u32 },
}
```

Generated TypeScript:

```typescript
export class NetworkError extends Error {
  override readonly name = 'NetworkError' as const;
  constructor(public readonly variant: NetworkErrorVariant) { /* ... */ }
  static NotFound(url: string): NetworkError { /* ... */ }
  static Timeout(timeoutMs: number): NetworkError { /* ... */ }
}
```

Catching errors:

```typescript
try {
  MyApi.fetchData(url);
} catch (e) {
  if (e instanceof NetworkError) {
    console.error(e.message);      // "NotFound: url=https://example.com"
    console.error(e.variant.tag);   // "NotFound"
    if (e.variant.tag === 'NotFound') {
      console.error(e.variant.url); // "https://example.com" (typed access)
    }
  }
}
```

Rich errors have human-readable `.message` strings built from the variant fields, structured `.variant` data for programmatic matching, and standard `.cause` for error chain tooling. Flat errors (no fields) produce a `.message` equal to the variant tag.

## Usage

### Generate command

```bash
uniffi-bindgen-js generate <SOURCE> --out-dir <DIR> [OPTIONS]
```

The tool auto-detects the mode from the file extension:

- **WASM mode** (`.wasm`) — reads metadata from a compiled WASM binary. Copies the `.wasm` to the output directory. This is the recommended approach.
- **Library mode** (`.dylib` / `.so` / `.dll`) — reads metadata from a compiled UniFFI cdylib.
- **UDL mode** (`.udl`) — reads a UDL file directly. Useful during development; the `.wasm` file must be placed alongside the output manually.

| Flag | Description |
|---|---|
| `--out-dir <dir>` | Output directory for generated TypeScript files |
| `--config <file>` | Path to `uniffi.toml` configuration |
| `--crate <name>` | Generate bindings for this crate only (library mode) |

### Configuration

Place a `[bindings.js]` section in your `uniffi.toml` and pass it with `--config`:

```toml
[bindings.js]
module_name = "MyBindings"
rename = { add_numbers = "sumValues", "Counter.currentValue" = "getValue" }
exclude = ["internal_helper"]
external_packages = { other_crate = "./other_bindings.js" }
```

See [docs/configuration.md](docs/configuration.md) for the full reference.

### Naming: separate UniFFI crates

If your UniFFI crate is a thin wrapper around a library (e.g., `html2markdown-uniffi` wrapping `html2markdown`), the auto-derived namespace will include the suffix — `Html2markdownUniffi`. Use `module_name` to choose a clean name:

```toml
# uniffi.toml
[bindings.js]
module_name = "Html2Markdown"
```

```typescript
// Before: Html2markdownUniffi.convert(html)
// After:  Html2Markdown.convert(html)
```

This is especially common when you keep UniFFI scaffolding in a separate crate to avoid feature-flag conflicts or to support multiple binding targets.

### External types

External types declared with `[External="crate_name"]` in UDL require a corresponding entry in `external_packages`:

```toml
[bindings.js]
external_packages = { other_crate = "./other_bindings.js" }
```

The generator emits named imports from the configured path.

## Features

- All UniFFI primitives, strings, bytes, timestamps (`Date`), and durations
- Records as TypeScript `interface` types with optional field defaults
- Flat enums (string literal unions) and data-carrying enums (discriminated unions)
- Objects with constructors, methods, `free()` lifecycle, and `Symbol.dispose`
- Flat and rich error classes via `[Error]` and `[Throws]`
- Async functions and methods mapped to `Promise<T>`
- Callback interfaces with VTable FFI glue
- Trait interfaces with object return lifting
- Custom type aliases and external type imports
- Rename, exclude, and docstring (JSDoc) support
- Enum methods, constructors, and discriminant annotations
- Non-exhaustive enums and errors with catch-all variants
- Default argument values and optional parameters

## Platform Requirements

Generated bindings require:

- **ES2022 modules** — top-level `await` is used to load the WASM module.
- **`FinalizationRegistry`** — used as a safety net for preventing leaked object handles (supported in all modern engines; a no-op polyfill is included for older environments).
- **`WebAssembly.Function`** (Type Reflection proposal) — required only when using **callback interfaces** or **async functions**. These features need typed WASM trampolines via `__indirect_function_table`. Supported in V8 (Chrome, Node.js 22+) and SpiderMonkey (Firefox). Safari 18.2+ added support; older Safari versions may not work.

### Rust crate setup

The Rust crate must enable two UniFFI feature flags:

```toml
uniffi = { version = "0.31", features = ["scaffolding-ffi-buffer-fns", "wasm-unstable-single-threaded"] }
```

- **`scaffolding-ffi-buffer-fns`** generates an alternate FFI layer where every function uses a uniform `(argPtr, retPtr)` calling convention instead of per-function signatures. This is what the generated TypeScript calls into. Also used by Mozilla's gecko-js bindings.
- **`wasm-unstable-single-threaded`** opts out of `Send + Sync` requirements on UniFFI objects when targeting `wasm32`, since WASM is single-threaded. The "unstable" label reflects the evolving state of WASM threading support; the feature itself has been stable since uniffi 0.27.

For callback interfaces and async, also set:

```
RUSTFLAGS="-C link-arg=--export-table -C link-arg=--growable-table"
```

## Compatibility

| uniffi-bindgen-js | uniffi-rs |
|---|---|
| 0.1.x | 0.31.0 |

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.

## License

MIT
