/**
 * Stress tests for JS ↔ WASM memory management.
 *
 * These tests push the FFI boundary to its limits: mass object allocation,
 * large data transfers that trigger WASM memory growth, concurrent async
 * operations, and rapid create/free cycles. They verify that:
 *
 * - Handle cloning and freeing stays consistent under load
 * - The scratch bump allocator resets correctly across many calls
 * - DataView invalidation works when WASM memory grows
 * - Concurrent async calls don't corrupt shared state
 * - Large sequences/strings serialize correctly
 */

import { describe, it, expect } from 'vitest';
import { FfiBasic, Counter } from '../generated/ffi_basic.js';
import { FfiCompound, type Point } from '../generated/ffi_compound.js';
import { FfiAsync, AsyncCounter } from '../generated/ffi_async.js';

// ---------------------------------------------------------------------------
// Object handle lifecycle stress
// ---------------------------------------------------------------------------

describe('stress: object handles', () => {
  it('create and free 10,000 objects', () => {
    for (let i = 0; i < 10_000; i++) {
      const c = Counter.create(BigInt(i));
      expect(c.getValue()).toBe(BigInt(i));
      c.free();
    }
  });

  it('create 1,000 objects, use all, then free all', () => {
    const counters: Counter[] = [];
    for (let i = 0; i < 1_000; i++) {
      counters.push(Counter.create(BigInt(i)));
    }
    // Use every counter while all are alive
    for (let i = 0; i < counters.length; i++) {
      expect(counters[i].getValue()).toBe(BigInt(i));
    }
    // Free in reverse order
    for (let i = counters.length - 1; i >= 0; i--) {
      counters[i].free();
    }
  });

  it('interleaved create/free maintains correct handles', () => {
    // Create A, create B, free A, use B, create C, free B, use C, free C
    const a = Counter.create(10n);
    const b = Counter.create(20n);
    a.free();
    expect(b.getValue()).toBe(20n);
    const c = Counter.create(30n);
    b.free();
    expect(c.getValue()).toBe(30n);
    c.free();
  });

  it('object passed as argument 1,000 times without cloning issues', () => {
    const c = Counter.create(42n);
    for (let i = 0; i < 1_000; i++) {
      expect(FfiBasic.getCounterValue(c)).toBe(42n);
    }
    c.free();
  });

  it('clone and use 1,000 objects from cloneCounter', () => {
    const original = Counter.create(99n);
    const clones: Counter[] = [];
    for (let i = 0; i < 1_000; i++) {
      clones.push(FfiBasic.cloneCounter(original));
    }
    // Verify all clones are independent
    original.increment();
    for (const clone of clones) {
      expect(clone.getValue()).toBe(99n);
      clone.free();
    }
    expect(original.getValue()).toBe(100n);
    original.free();
  });

  it('free is safe to call many times', () => {
    const c = Counter.create(0n);
    for (let i = 0; i < 100; i++) {
      c.free(); // should never throw
    }
  });
});

// ---------------------------------------------------------------------------
// Scratch allocator stress
// ---------------------------------------------------------------------------

describe('stress: scratch allocator', () => {
  it('10,000 rapid function calls', () => {
    for (let i = 0; i < 10_000; i++) {
      expect(FfiBasic.add(i, 1)).toBe(i + 1);
    }
  });

  it('10,000 rapid string roundtrips', () => {
    for (let i = 0; i < 10_000; i++) {
      expect(FfiCompound.identityString(`test_${i}`)).toBe(`test_${i}`);
    }
  });

  it('alternating small and large allocations', () => {
    for (let i = 0; i < 1_000; i++) {
      // Small: single int
      expect(FfiBasic.add(1, 1)).toBe(2);
      // Medium: string
      expect(FfiCompound.identityString('hello world')).toBe('hello world');
      // Larger: record
      const p = FfiCompound.makePoint(i, i * 2);
      expect(p.x).toBe(i);
      expect(p.y).toBe(i * 2);
    }
  });
});

// ---------------------------------------------------------------------------
// Large data (triggers WASM memory growth → DataView invalidation)
// ---------------------------------------------------------------------------

describe('stress: large data', () => {
  it('roundtrip a 1MB string', () => {
    const big = 'x'.repeat(1_000_000);
    const result = FfiCompound.identityString(big);
    expect(result.length).toBe(1_000_000);
    expect(result[0]).toBe('x');
    expect(result[result.length - 1]).toBe('x');
  });

  it('roundtrip a 1MB byte array', () => {
    const big = new Uint8Array(1_000_000);
    big.fill(0xAB);
    const result = FfiCompound.identityBytes(big);
    expect(result.length).toBe(1_000_000);
    expect(result[0]).toBe(0xAB);
    expect(result[result.length - 1]).toBe(0xAB);
  });

  it('multiple large strings in sequence (exercises memory growth)', () => {
    for (let i = 0; i < 10; i++) {
      const big = String.fromCharCode(65 + i).repeat(500_000);
      const result = FfiCompound.identityString(big);
      expect(result.length).toBe(500_000);
      expect(result[0]).toBe(String.fromCharCode(65 + i));
    }
  });

  it('large byte array followed by small operations', () => {
    // This specifically tests DataView invalidation: the large allocation
    // may trigger memory.grow(), and subsequent small reads must still work.
    const big = new Uint8Array(2_000_000);
    big.fill(0xFF);
    const result = FfiCompound.identityBytes(big);
    expect(result.length).toBe(2_000_000);

    // Small operations after potential memory growth
    expect(FfiBasic.add(1, 2)).toBe(3);
    expect(FfiCompound.identityString('ok')).toBe('ok');
    expect(FfiCompound.identityI32(42)).toBe(42);
  });

  it('large unicode string (multi-byte chars)', () => {
    // Emoji are 4 bytes each in UTF-8, so this is ~4MB of encoded data
    const emoji = '🎉'.repeat(100_000);
    const result = FfiCompound.identityString(emoji);
    expect(result.length).toBe(200_000); // 100k emoji × 2 UTF-16 code units each
    expect(result.slice(0, 2)).toBe('🎉');
  });
});

// ---------------------------------------------------------------------------
// Large compound types
// ---------------------------------------------------------------------------

describe('stress: large compound types', () => {
  it('sequence of 10,000 integers', () => {
    const arr = Array.from({ length: 10_000 }, (_, i) => i);
    const result = FfiCompound.identitySeqI32(arr);
    expect(result.length).toBe(10_000);
    expect(result[0]).toBe(0);
    expect(result[9_999]).toBe(9_999);
  });

  it('sequence of 10,000 strings', () => {
    const arr = Array.from({ length: 10_000 }, (_, i) => `item_${i}`);
    const result = FfiCompound.identitySeqString(arr);
    expect(result.length).toBe(10_000);
    expect(result[0]).toBe('item_0');
    expect(result[9_999]).toBe('item_9999');
  });

  it('sequence of 5,000 records', () => {
    const points: Point[] = Array.from({ length: 5_000 }, (_, i) => ({
      x: i,
      y: i * 2,
    }));
    const result = FfiCompound.identitySeqPoint(points);
    expect(result.length).toBe(5_000);
    expect(result[0]).toEqual({ x: 0, y: 0 });
    expect(result[4_999]).toEqual({ x: 4_999, y: 9_998 });
  });

  it('map with 1,000 entries', () => {
    const m = new Map<string, number>();
    for (let i = 0; i < 1_000; i++) {
      m.set(`key_${i}`, i);
    }
    const result = FfiCompound.identityMapStringI32(m);
    expect(result.size).toBe(1_000);
    expect(result.get('key_0')).toBe(0);
    expect(result.get('key_999')).toBe(999);
  });

  it('sequence of 1,000 objects (handle cloning at scale)', () => {
    const counters: Counter[] = [];
    for (let i = 0; i < 1_000; i++) {
      counters.push(Counter.create(BigInt(i)));
    }
    const values = FfiBasic.getCounterValues(counters);
    expect(Array.from(values).length).toBe(1_000);
    expect(values[0]).toBe(0n);
    expect(values[999]).toBe(999n);
    // All counters should still be usable after being passed in a sequence
    for (let i = 0; i < counters.length; i++) {
      expect(counters[i].getValue()).toBe(BigInt(i));
    }
    for (const c of counters) {
      c.free();
    }
  });
});

// ---------------------------------------------------------------------------
// Concurrent async stress
// ---------------------------------------------------------------------------

describe('stress: concurrent async', () => {
  it('100 concurrent async calls', async () => {
    const promises = Array.from({ length: 100 }, (_, i) =>
      FfiAsync.asyncAdd(i, 1),
    );
    const results = await Promise.all(promises);
    for (let i = 0; i < 100; i++) {
      expect(results[i]).toBe(i + 1);
    }
  });

  it('100 concurrent async string calls', async () => {
    const promises = Array.from({ length: 100 }, (_, i) =>
      FfiAsync.asyncGreet(`user_${i}`),
    );
    const results = await Promise.all(promises);
    for (let i = 0; i < 100; i++) {
      expect(results[i]).toBe(`Hello, user_${i}!`);
    }
  });

  it('50 concurrent async object methods', async () => {
    const counters: AsyncCounter[] = [];
    for (let i = 0; i < 50; i++) {
      counters.push(await AsyncCounter.create(BigInt(i)));
    }
    // Concurrently read all values
    const values = await Promise.all(
      counters.map((c) => c.getValue()),
    );
    for (let i = 0; i < 50; i++) {
      expect(values[i]).toBe(BigInt(i));
    }
    for (const c of counters) {
      c.free();
    }
  });

  it('concurrent create, increment, read, free cycle', async () => {
    const run = async (id: number) => {
      const c = await AsyncCounter.create(BigInt(id));
      await c.increment();
      await c.increment();
      await c.increment();
      const val = await c.getValue();
      c.free();
      return val;
    };
    const results = await Promise.all(
      Array.from({ length: 50 }, (_, i) => run(i)),
    );
    for (let i = 0; i < 50; i++) {
      expect(results[i]).toBe(BigInt(i + 3));
    }
  });

  it('mixed sync and async calls', async () => {
    // Sync calls interleaved with async shouldn't corrupt scratch state
    const asyncResult = FfiAsync.asyncAdd(10, 20);
    const syncResult = FfiBasic.add(1, 2);
    expect(syncResult).toBe(3);
    expect(await asyncResult).toBe(30);
  });
});

// ---------------------------------------------------------------------------
// Edge cases
// ---------------------------------------------------------------------------

describe('stress: edge cases', () => {
  it('empty string roundtrip under load', () => {
    for (let i = 0; i < 1_000; i++) {
      expect(FfiCompound.identityString('')).toBe('');
    }
  });

  it('empty bytes roundtrip under load', () => {
    for (let i = 0; i < 1_000; i++) {
      const result = FfiCompound.identityBytes(new Uint8Array(0));
      expect(result.length).toBe(0);
    }
  });

  it('empty sequence roundtrip under load', () => {
    for (let i = 0; i < 1_000; i++) {
      expect(FfiCompound.identitySeqI32([])).toEqual([]);
    }
  });

  it('null optional roundtrip under load', () => {
    for (let i = 0; i < 1_000; i++) {
      expect(FfiCompound.identityOptionalString(null)).toBeNull();
    }
  });

  it('boundary integer values under load', () => {
    for (let i = 0; i < 1_000; i++) {
      expect(FfiCompound.identityI32(2_147_483_647)).toBe(2_147_483_647);
      expect(FfiCompound.identityI32(-2_147_483_648)).toBe(-2_147_483_648);
      expect(FfiCompound.identityU64(18_446_744_073_709_551_615n)).toBe(18_446_744_073_709_551_615n);
    }
  });
});
