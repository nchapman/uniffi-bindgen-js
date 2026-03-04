/**
 * FFI-mode smoke test: async functions via RustFuture polling.
 *
 * Tests async functions, async constructors, async methods,
 * async + throws, and object arguments to async functions.
 */

import { describe, it, expect } from 'vitest';
import { FfiAsync, AsyncCounter, AsyncError } from '../generated/ffi_async.js';

describe('Async: basic functions', () => {
  it('async_add returns sum', async () => {
    expect(await FfiAsync.asyncAdd(1, 2)).toBe(3);
  });

  it('async_add with zero', async () => {
    expect(await FfiAsync.asyncAdd(0, 0)).toBe(0);
  });

  it('async_add with large values', async () => {
    expect(await FfiAsync.asyncAdd(1_000_000, 2_000_000)).toBe(3_000_000);
  });

  it('async_greet returns formatted string', async () => {
    expect(await FfiAsync.asyncGreet('world')).toBe('Hello, world!');
  });

  it('async_greet with empty string', async () => {
    expect(await FfiAsync.asyncGreet('')).toBe('Hello, !');
  });

  it('async_greet with unicode', async () => {
    expect(await FfiAsync.asyncGreet('café')).toBe('Hello, café!');
  });

  it('async_noop completes without error', async () => {
    await FfiAsync.asyncNoop();
  });

  it('multiple async calls in sequence', async () => {
    const r1 = await FfiAsync.asyncAdd(1, 1);
    const r2 = await FfiAsync.asyncAdd(r1, 1);
    const r3 = await FfiAsync.asyncAdd(r2, 1);
    expect(r3).toBe(4);
  });

  it('concurrent async calls', async () => {
    const [a, b, c] = await Promise.all([
      FfiAsync.asyncAdd(1, 2),
      FfiAsync.asyncGreet('Alice'),
      FfiAsync.asyncNoop(),
    ]);
    expect(a).toBe(3);
    expect(b).toBe('Hello, Alice!');
    expect(c).toBeUndefined();
  });
});

describe('Async: errors (Throws)', () => {
  it('async_divide success', async () => {
    expect(await FfiAsync.asyncDivide(10, 2)).toBe('5');
  });

  it('async_divide throws DivisionByZero', async () => {
    await expect(FfiAsync.asyncDivide(1, 0)).rejects.toThrow(AsyncError);
    try {
      await FfiAsync.asyncDivide(1, 0);
    } catch (e) {
      expect(e).toBeInstanceOf(AsyncError);
      expect((e as AsyncError).tag).toBe('DivisionByZero');
    }
  });
});

describe('Async: object with async constructor and methods', () => {
  it('async constructor creates counter', async () => {
    const counter = await AsyncCounter.create(42n);
    expect(await counter.getValue()).toBe(42n);
    counter.free();
  });

  it('async increment', async () => {
    const counter = await AsyncCounter.create(0n);
    await counter.increment();
    await counter.increment();
    await counter.increment();
    expect(await counter.getValue()).toBe(3n);
    counter.free();
  });

  it('async method with throws — success', async () => {
    const counter = await AsyncCounter.create(100n);
    await counter.validate(); // should not throw
    counter.free();
  });

  it('async method with throws — error', async () => {
    const counter = await AsyncCounter.create(2_000_000n);
    await expect(counter.validate()).rejects.toThrow(AsyncError);
    try {
      await counter.validate();
    } catch (e) {
      expect((e as AsyncError).tag).toBe('InvalidInput');
    }
    counter.free();
  });

  it('object argument to async function', async () => {
    const counter = await AsyncCounter.create(99n);
    const val = await FfiAsync.asyncGetCounterValue(counter);
    expect(val).toBe(99n);
    // counter should still be usable after being passed as arg
    expect(await counter.getValue()).toBe(99n);
    counter.free();
  });

  it('concurrent async method calls', async () => {
    const c1 = await AsyncCounter.create(10n);
    const c2 = await AsyncCounter.create(20n);
    const [v1, v2] = await Promise.all([
      c1.getValue(),
      c2.getValue(),
    ]);
    expect(v1).toBe(10n);
    expect(v2).toBe(20n);
    c1.free();
    c2.free();
  });
});
