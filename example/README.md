# Example: Rust to TypeScript via UniFFI

This example shows the full workflow for generating TypeScript bindings from a Rust crate.

## What's here

```
example/
  rust/           # Minimal Rust crate with UDL interface
    Cargo.toml
    src/lib.rs
    src/example.udl
  generated/      # Pre-generated TypeScript output (for reference)
    example.ts
```

## Workflow

### 1. Build the Rust crate

The Rust side uses `#[uniffi::export]` proc macros to define the FFI interface. For WASM targets, a companion `wasm-bindgen` crate wraps the UniFFI exports (see the fixtures in `fixtures/*/wasm/` for the full pattern). For this example, the Rust crate compiles as a cdylib:

```bash
cd example/rust
cargo build --release
# Produces a cdylib: target/release/libuniffi_example.dylib (macOS) / .so (Linux)
```

For a WASM build, you would create a separate wasm-bindgen wrapper crate and use `wasm-pack build`.

### 2. Generate TypeScript bindings

```bash
# From the repository root:
cargo run -- generate example/rust/src/example.udl --out-dir example/generated/
```

This reads the UDL interface definition and produces `example.ts`.

### 3. Use in TypeScript

```typescript
import { init, Example } from './generated/example.ts';
import type { TodoEntry } from './generated/example.ts';

// Initialize the WASM module
await init();

// Call Rust functions directly
console.log(Example.greet('World')); // "Hello, World!"

// Use generated record types
const todo: TodoEntry = { title: 'Learn UniFFI', done: false };
console.log(`${todo.title}: ${todo.done}`);
```

## UDL interface

The interface is defined in [`rust/src/example.udl`](rust/src/example.udl):

```
namespace example {
  string greet(string name);
};

dictionary TodoEntry {
  string title;
  boolean done;
};
```

The matching Rust implementation is in [`rust/src/lib.rs`](rust/src/lib.rs). The `#[uniffi::export]` and `#[derive(uniffi::Record)]` macros wire the Rust code to the UDL definitions. The `.udl` file is read by `uniffi-bindgen-js generate` — it is not used by the Rust compiler (the proc macros handle that side).
