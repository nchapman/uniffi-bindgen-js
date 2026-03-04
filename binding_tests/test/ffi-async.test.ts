/**
 * FFI-mode smoke test: async functions via RustFuture polling.
 *
 * Tests that async Rust functions can be called from JS through the
 * RustFuture polling protocol (poll → complete → free).
 *
 * Prerequisites: run `./scripts/build_bindings.sh` to populate
 * binding_tests/generated/ with ffi_async.ts, ffi_async.wasm, and uniffi_runtime.ts.
 */

import { describe, it, expect } from 'vitest';
import { FfiAsync } from '../generated/ffi_async.js';

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
