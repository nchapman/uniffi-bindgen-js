/**
 * FFI-mode smoke test: verify the generated FFI bindings call directly
 * into the WASM module via the UniFFI FFI buffer convention.
 *
 * Tests primitives, strings, objects (handle lifecycle), object arguments
 * to functions and methods, and object return types.
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

describe('Object arguments', () => {
  it('passes object to free function', () => {
    const c = Counter.create(42n);
    expect(FfiBasic.getCounterValue(c)).toBe(42n);
    // counter should still be usable after being passed as arg
    expect(c.getValue()).toBe(42n);
    c.free();
  });

  it('passes object to method (add_from)', () => {
    const c1 = Counter.create(10n);
    const c2 = Counter.create(20n);
    c1.addFrom(c2);
    expect(c1.getValue()).toBe(30n);
    // c2 should still be usable
    expect(c2.getValue()).toBe(20n);
    c1.free();
    c2.free();
  });

  it('passes same object multiple times', () => {
    const c = Counter.create(5n);
    expect(FfiBasic.getCounterValue(c)).toBe(5n);
    expect(FfiBasic.getCounterValue(c)).toBe(5n);
    expect(FfiBasic.getCounterValue(c)).toBe(5n);
    c.free();
  });
});

describe('Object return types', () => {
  it('returns object from free function', () => {
    const original = Counter.create(100n);
    const cloned = FfiBasic.cloneCounter(original);
    expect(cloned.getValue()).toBe(100n);
    // Modify original, clone should be independent
    original.increment();
    expect(original.getValue()).toBe(101n);
    expect(cloned.getValue()).toBe(100n);
    original.free();
    cloned.free();
  });

  it('original still works after cloneCounter', () => {
    const c = Counter.create(0n);
    const c2 = FfiBasic.cloneCounter(c);
    c.increment();
    expect(c.getValue()).toBe(1n);
    expect(c2.getValue()).toBe(0n);
    c.free();
    c2.free();
  });
});
