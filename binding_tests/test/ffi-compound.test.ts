/**
 * FFI-mode compound type tests: exercise records, enums, optionals,
 * sequences, maps, duration, timestamp, bytes, and all primitive types
 * through actual WASM execution.
 */

import { describe, it, expect } from 'vitest';
import { FfiCompound, type Point, type Color, type Shape } from '../generated/ffi_compound.js';

// ---------------------------------------------------------------------------
// Primitives
// ---------------------------------------------------------------------------

describe('primitives', () => {
  it('bool roundtrips', () => {
    expect(FfiCompound.identityBool(true)).toBe(true);
    expect(FfiCompound.identityBool(false)).toBe(false);
  });

  it('i8 roundtrips', () => {
    expect(FfiCompound.identityI8(0)).toBe(0);
    expect(FfiCompound.identityI8(127)).toBe(127);
    expect(FfiCompound.identityI8(-128)).toBe(-128);
  });

  it('u8 roundtrips', () => {
    expect(FfiCompound.identityU8(0)).toBe(0);
    expect(FfiCompound.identityU8(255)).toBe(255);
  });

  it('i16 roundtrips', () => {
    expect(FfiCompound.identityI16(0)).toBe(0);
    expect(FfiCompound.identityI16(32767)).toBe(32767);
    expect(FfiCompound.identityI16(-32768)).toBe(-32768);
  });

  it('u16 roundtrips', () => {
    expect(FfiCompound.identityU16(0)).toBe(0);
    expect(FfiCompound.identityU16(65535)).toBe(65535);
  });

  it('i32 roundtrips', () => {
    expect(FfiCompound.identityI32(0)).toBe(0);
    expect(FfiCompound.identityI32(2147483647)).toBe(2147483647);
    expect(FfiCompound.identityI32(-2147483648)).toBe(-2147483648);
  });

  it('u32 roundtrips', () => {
    expect(FfiCompound.identityU32(0)).toBe(0);
    expect(FfiCompound.identityU32(4294967295)).toBe(4294967295);
  });

  it('i64 roundtrips', () => {
    expect(FfiCompound.identityI64(0n)).toBe(0n);
    expect(FfiCompound.identityI64(9223372036854775807n)).toBe(9223372036854775807n);
    expect(FfiCompound.identityI64(-9223372036854775808n)).toBe(-9223372036854775808n);
  });

  it('u64 roundtrips', () => {
    expect(FfiCompound.identityU64(0n)).toBe(0n);
    expect(FfiCompound.identityU64(18446744073709551615n)).toBe(18446744073709551615n);
  });

  it('f32 roundtrips', () => {
    expect(FfiCompound.identityF32(0)).toBe(0);
    const result = FfiCompound.identityF32(3.14);
    expect(result).toBeCloseTo(3.14, 5);
  });

  it('f64 roundtrips', () => {
    expect(FfiCompound.identityF64(0)).toBe(0);
    expect(FfiCompound.identityF64(3.141592653589793)).toBe(3.141592653589793);
    expect(FfiCompound.identityF64(-1e100)).toBe(-1e100);
  });

  it('string roundtrips', () => {
    expect(FfiCompound.identityString('')).toBe('');
    expect(FfiCompound.identityString('hello')).toBe('hello');
    expect(FfiCompound.identityString('🎉🌍')).toBe('🎉🌍');
  });

  it('bytes roundtrips', () => {
    const empty = FfiCompound.identityBytes(new Uint8Array([]));
    expect(empty.length).toBe(0);

    const data = new Uint8Array([1, 2, 3, 255, 0]);
    const result = FfiCompound.identityBytes(data);
    expect(result).toEqual(data);
  });
});

// ---------------------------------------------------------------------------
// Optional
// ---------------------------------------------------------------------------

describe('optional', () => {
  it('string: some', () => {
    expect(FfiCompound.identityOptionalString('hello')).toBe('hello');
  });

  it('string: null', () => {
    expect(FfiCompound.identityOptionalString(null)).toBeNull();
  });

  it('i32: some', () => {
    expect(FfiCompound.identityOptionalI32(42)).toBe(42);
  });

  it('i32: null', () => {
    expect(FfiCompound.identityOptionalI32(null)).toBeNull();
  });
});

// ---------------------------------------------------------------------------
// Sequence
// ---------------------------------------------------------------------------

describe('sequence', () => {
  it('i32 sequence', () => {
    expect(FfiCompound.identitySeqI32([])).toEqual([]);
    expect(FfiCompound.identitySeqI32([1, 2, 3])).toEqual([1, 2, 3]);
  });

  it('string sequence', () => {
    expect(FfiCompound.identitySeqString([])).toEqual([]);
    expect(FfiCompound.identitySeqString(['a', 'b', 'c'])).toEqual(['a', 'b', 'c']);
  });
});

// ---------------------------------------------------------------------------
// Map
// ---------------------------------------------------------------------------

describe('map', () => {
  it('string→i32 map', () => {
    const empty = FfiCompound.identityMapStringI32(new Map());
    expect(empty.size).toBe(0);

    const m = new Map([['a', 1], ['b', 2]]);
    const result = FfiCompound.identityMapStringI32(m);
    expect(result.get('a')).toBe(1);
    expect(result.get('b')).toBe(2);
  });
});

// ---------------------------------------------------------------------------
// Duration & Timestamp
// ---------------------------------------------------------------------------

describe('duration', () => {
  it('roundtrips a duration', () => {
    const d = FfiCompound.identityDuration(1.5);
    expect(d).toBeCloseTo(1.5, 6);
  });

  it('handles zero', () => {
    expect(FfiCompound.identityDuration(0)).toBe(0);
  });
});

describe('timestamp', () => {
  it('roundtrips a Date', () => {
    const now = new Date();
    // Truncate to milliseconds (JS Date precision)
    const truncated = new Date(now.getTime());
    const result = FfiCompound.identityTimestamp(truncated);
    expect(result.getTime()).toBe(truncated.getTime());
  });

  it('handles epoch', () => {
    const epoch = new Date(0);
    const result = FfiCompound.identityTimestamp(epoch);
    expect(result.getTime()).toBe(0);
  });
});

// ---------------------------------------------------------------------------
// Record
// ---------------------------------------------------------------------------

describe('record', () => {
  it('make_point', () => {
    const p = FfiCompound.makePoint(3, 4);
    expect(p.x).toBe(3);
    expect(p.y).toBe(4);
  });

  it('point_distance', () => {
    const a: Point = { x: 0, y: 0 };
    const b: Point = { x: 3, y: 4 };
    expect(FfiCompound.pointDistance(a, b)).toBeCloseTo(5, 10);
  });
});

// ---------------------------------------------------------------------------
// Flat enum
// ---------------------------------------------------------------------------

describe('flat enum', () => {
  it('Red', () => {
    expect(FfiCompound.colorName('Red')).toBe('red');
  });

  it('Green', () => {
    expect(FfiCompound.colorName('Green')).toBe('green');
  });

  it('Blue', () => {
    expect(FfiCompound.colorName('Blue')).toBe('blue');
  });
});

// ---------------------------------------------------------------------------
// Data enum
// ---------------------------------------------------------------------------

describe('data enum', () => {
  it('Circle area', () => {
    const area = FfiCompound.shapeArea({ tag: 'Circle', radius: 1 });
    expect(area).toBeCloseTo(Math.PI, 10);
  });

  it('Rect area', () => {
    const area = FfiCompound.shapeArea({ tag: 'Rect', width: 3, height: 4 });
    expect(area).toBe(12);
  });
});

// ---------------------------------------------------------------------------
// Nested compound types
// ---------------------------------------------------------------------------

describe('nested compound', () => {
  it('sequence of records', () => {
    const points: Point[] = [{ x: 1, y: 2 }, { x: 3, y: 4 }];
    const result = FfiCompound.identitySeqPoint(points);
    expect(result).toEqual(points);
  });

  it('map of records', () => {
    const m = new Map<string, Point>([
      ['origin', { x: 0, y: 0 }],
      ['unit', { x: 1, y: 1 }],
    ]);
    const result = FfiCompound.identityMapPoint(m);
    expect(result.get('origin')).toEqual({ x: 0, y: 0 });
    expect(result.get('unit')).toEqual({ x: 1, y: 1 });
  });

  it('optional record: some', () => {
    const p: Point = { x: 5, y: 6 };
    const result = FfiCompound.identityOptionalPoint(p);
    expect(result).toEqual(p);
  });

  it('optional record: null', () => {
    expect(FfiCompound.identityOptionalPoint(null)).toBeNull();
  });
});
