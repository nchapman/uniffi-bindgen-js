/**
 * Smoke tests for the type-zoo fixture — exercises all builtin types:
 * numeric primitives, boolean, string, bytes, duration, timestamp,
 * optionals, sequences, and maps.
 *
 * Build generated output with scripts/build_bindings.sh before running.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, TypeZoo } from '../generated/type_zoo.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  // wasm-pack appends _bg to our --out-name, resulting in the double _bg suffix
  const wasmPath = resolve(__dirname, '../generated/type_zoo_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

// --- Integer primitives ---

describe('TypeZoo integer echo', () => {
  it('echoes i8', () => {
    expect(TypeZoo.echoI8(42)).toBe(42);
    expect(TypeZoo.echoI8(-128)).toBe(-128);
    expect(TypeZoo.echoI8(127)).toBe(127);
  });

  it('echoes i16', () => {
    expect(TypeZoo.echoI16(1000)).toBe(1000);
    expect(TypeZoo.echoI16(-32768)).toBe(-32768);
  });

  it('echoes i32', () => {
    expect(TypeZoo.echoI32(0)).toBe(0);
    expect(TypeZoo.echoI32(-2147483648)).toBe(-2147483648);
    expect(TypeZoo.echoI32(2147483647)).toBe(2147483647);
  });

  it('echoes u8', () => {
    expect(TypeZoo.echoU8(0)).toBe(0);
    expect(TypeZoo.echoU8(255)).toBe(255);
  });

  it('echoes u16', () => {
    expect(TypeZoo.echoU16(65535)).toBe(65535);
  });

  it('echoes u32', () => {
    expect(TypeZoo.echoU32(4294967295)).toBe(4294967295);
  });
});

// --- Bigint types (i64, u64) ---

describe('TypeZoo bigint echo', () => {
  it('echoes i64', () => {
    expect(TypeZoo.echoI64(0n)).toBe(0n);
    expect(TypeZoo.echoI64(-9007199254740991n)).toBe(-9007199254740991n);
  });

  it('echoes u64', () => {
    expect(TypeZoo.echoU64(0n)).toBe(0n);
    expect(TypeZoo.echoU64(9007199254740991n)).toBe(9007199254740991n);
    expect(TypeZoo.echoU64(18446744073709551615n)).toBe(18446744073709551615n);
  });
});

// --- Floating-point ---

describe('TypeZoo float echo', () => {
  it('echoes f32', () => {
    const result = TypeZoo.echoF32(3.14);
    expect(result).toBeCloseTo(3.14, 2);
  });

  it('echoes f32 negative', () => {
    const result = TypeZoo.echoF32(-0.001);
    expect(result).toBeCloseTo(-0.001, 3);
  });

  it('echoes f64', () => {
    expect(TypeZoo.echoF64(3.141592653589793)).toBe(3.141592653589793);
  });

  it('echoes f64 zero', () => {
    expect(TypeZoo.echoF64(0.0)).toBe(0.0);
  });
});

// --- Boolean ---

describe('TypeZoo boolean echo', () => {
  it('echoes true', () => {
    expect(TypeZoo.echoBool(true)).toBe(true);
  });

  it('echoes false', () => {
    expect(TypeZoo.echoBool(false)).toBe(false);
  });
});

// --- String ---

describe('TypeZoo string echo', () => {
  it('echoes a plain string', () => {
    expect(TypeZoo.echoString('hello')).toBe('hello');
  });

  it('echoes an empty string', () => {
    expect(TypeZoo.echoString('')).toBe('');
  });

  it('echoes unicode', () => {
    expect(TypeZoo.echoString('cafe\u0301')).toBe('cafe\u0301');
  });
});

// --- Bytes ---

describe('TypeZoo bytes echo', () => {
  it('echoes a byte array', () => {
    const input = new Uint8Array([1, 2, 3, 255]);
    const result = TypeZoo.echoBytes(input);
    expect(result).toBeInstanceOf(Uint8Array);
    expect(Array.from(result)).toEqual([1, 2, 3, 255]);
  });

  it('echoes empty bytes', () => {
    const result = TypeZoo.echoBytes(new Uint8Array([]));
    expect(result.length).toBe(0);
  });
});

// --- Duration ---

describe('TypeZoo duration echo', () => {
  it('round-trips a duration value', () => {
    expect(TypeZoo.echoDuration(5.5)).toBeCloseTo(5.5);
  });

  it('round-trips zero duration', () => {
    expect(TypeZoo.echoDuration(0)).toBe(0);
  });
});

// --- Timestamp ---

describe('TypeZoo timestamp echo', () => {
  it('round-trips a Date', () => {
    const now = new Date();
    const result = TypeZoo.echoTimestamp(now);
    expect(result).toBeInstanceOf(Date);
    // Allow 1ms tolerance for floating-point Date round-trip
    expect(Math.abs(result.getTime() - now.getTime())).toBeLessThanOrEqual(1);
  });

  it('round-trips epoch', () => {
    const epoch = new Date(0);
    const result = TypeZoo.echoTimestamp(epoch);
    expect(result.getTime()).toBe(0);
  });
});

// --- Optional string ---

describe('TypeZoo optional string', () => {
  it('returns the string when present', () => {
    expect(TypeZoo.maybeString('hi')).toBe('hi');
  });

  it('returns null when input is null', () => {
    expect(TypeZoo.maybeString(null)).toBeNull();
  });
});

// --- Sequences ---

describe('TypeZoo sequence echo', () => {
  it('echoes a string array', () => {
    expect(TypeZoo.echoStrings(['a', 'b', 'c'])).toEqual(['a', 'b', 'c']);
  });

  it('echoes an empty array', () => {
    expect(TypeZoo.echoStrings([])).toEqual([]);
  });

  it('echoes a bigint array', () => {
    const input = [1n, 2n, 3n];
    const result = TypeZoo.echoBigints(input);
    expect(result).toEqual([1n, 2n, 3n]);
  });
});

// --- Maps ---

describe('TypeZoo map echo', () => {
  it('echoes a string-to-i32 map', () => {
    const input = new Map<string, number>([['a', 1], ['b', 2]]);
    const result = TypeZoo.echoMap(input);
    expect(result).toBeInstanceOf(Map);
    expect(result.get('a')).toBe(1);
    expect(result.get('b')).toBe(2);
  });

  it('echoes a string-to-boolean map', () => {
    const input = new Map<string, boolean>([['x', true], ['y', false]]);
    const result = TypeZoo.echoBoolMap(input);
    expect(result).toBeInstanceOf(Map);
    expect(result.get('x')).toBe(true);
    expect(result.get('y')).toBe(false);
  });
});
