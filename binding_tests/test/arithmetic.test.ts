/**
 * Smoke tests for the arithmetic fixture — exercises [Throws] + flat [Error] enum.
 *
 * The WASM binary and JS glue (arithmetic_bg.js, arithmetic_bg_bg.wasm) must be
 * present in binding_tests/generated/ before running (built by build_bindings.sh).
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, MathError, Arithmetic } from '../generated/arithmetic.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  const wasmPath = resolve(__dirname, '../generated/arithmetic_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

describe('Arithmetic.divide', () => {
  it('divides two positive numbers', () => {
    expect(Arithmetic.divide(10, 2)).toBe(5);
  });

  it('divides returning a float', () => {
    expect(Arithmetic.divide(1, 4)).toBe(0.25);
  });

  it('throws MathError.DivisionByZero when dividing by zero', () => {
    expect(() => Arithmetic.divide(5, 0)).toThrow(MathError);
    try {
      Arithmetic.divide(5, 0);
    } catch (e) {
      expect(e).toBeInstanceOf(MathError);
      expect((e as MathError).tag).toBe('DivisionByZero');
    }
  });
});

describe('Arithmetic.sqrt', () => {
  it('returns the square root of a positive number', () => {
    expect(Arithmetic.sqrt(9)).toBe(3);
    expect(Arithmetic.sqrt(2)).toBeCloseTo(Math.SQRT2);
  });

  it('returns 0 for sqrt(0)', () => {
    expect(Arithmetic.sqrt(0)).toBe(0);
  });

  it('throws MathError.NegativeSquareRoot for negative input', () => {
    expect(() => Arithmetic.sqrt(-1)).toThrow(MathError);
    try {
      Arithmetic.sqrt(-1);
    } catch (e) {
      expect(e).toBeInstanceOf(MathError);
      expect((e as MathError).tag).toBe('NegativeSquareRoot');
    }
  });
});
