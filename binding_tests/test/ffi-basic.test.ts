/**
 * FFI-mode smoke test: verify the generated FFI bindings call directly
 * into the WASM module via the UniFFI FFI buffer convention.
 *
 * Unlike the wasm-pack tests, there is no init() ceremony — the generated
 * module uses top-level await to auto-load the co-located .wasm file.
 *
 * Prerequisites: run `./scripts/build_ffi_fixture.sh` to populate
 * binding_tests/generated/ with ffi_basic.ts, ffi_basic.wasm, and uniffi_runtime.ts.
 */

import { describe, it, expect } from 'vitest';
import { FfiBasic, Counter } from '../generated/ffi_basic.js';

describe('FfiBasic.add', () => {
  it('adds two unsigned integers', () => {
    expect(FfiBasic.add(1, 2)).toBe(3);
  });

  it('handles zero', () => {
    expect(FfiBasic.add(0, 0)).toBe(0);
  });

  it('handles large values', () => {
    expect(FfiBasic.add(2_000_000_000, 1_000_000_000)).toBe(3_000_000_000);
  });
});

describe('FfiBasic.greet', () => {
  it('returns a greeting', () => {
    expect(FfiBasic.greet('world')).toBe('Hello, world!');
  });

  it('handles empty string', () => {
    expect(FfiBasic.greet('')).toBe('Hello, !');
  });

  it('handles unicode', () => {
    expect(FfiBasic.greet('🌍')).toBe('Hello, 🌍!');
  });
});

describe('Counter', () => {
  it('creates with initial value', () => {
    const c = Counter.create(42n);
    expect(c.getValue()).toBe(42n);
    c.free();
  });

  it('increments', () => {
    const c = Counter.create(0n);
    c.increment();
    c.increment();
    c.increment();
    expect(c.getValue()).toBe(3n);
    c.free();
  });

  it('free is idempotent', () => {
    const c = Counter.create(1n);
    c.free();
    c.free(); // should not throw
  });

  it('throws after free', () => {
    const c = Counter.create(1n);
    c.free();
    expect(() => c.getValue()).toThrow('Counter object has been freed');
  });
});
