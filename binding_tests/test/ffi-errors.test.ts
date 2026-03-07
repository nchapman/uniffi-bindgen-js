/**
 * FFI-mode error handling tests: flat errors, rich errors,
 * throwing from functions/constructors/methods.
 */

import { describe, it, expect } from 'vitest';
import {
  FfiErrors,
  MathError,
  NetworkError,
  ParseError,
  Parser,
} from '../generated/ffi_errors.js';

// ---------------------------------------------------------------------------
// Flat error (MathError)
// ---------------------------------------------------------------------------

describe('FfiErrors.safeDivide (flat error)', () => {
  it('returns result on success', () => {
    expect(FfiErrors.safeDivide(10, 2)).toBe('5');
  });

  it('throws DivisionByZero', () => {
    try {
      FfiErrors.safeDivide(1, 0);
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(MathError);
      expect((e as MathError).variant.tag).toBe('DivisionByZero');
    }
  });
});

// ---------------------------------------------------------------------------
// Rich error (NetworkError)
// ---------------------------------------------------------------------------

describe('FfiErrors.fetchData (rich error)', () => {
  it('returns data for valid URL', () => {
    expect(FfiErrors.fetchData('good')).toBe('data for good');
  });

  it('throws NotFound', () => {
    try {
      FfiErrors.fetchData('404');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('NotFound');
      if (err.variant.tag === 'NotFound') {
        expect(err.variant.url).toBe('404');
      }
    }
  });

  it('throws Timeout', () => {
    try {
      FfiErrors.fetchData('timeout');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('Timeout');
      if (err.variant.tag === 'Timeout') {
        expect(err.variant.afterMs).toBe(5000);
      }
    }
  });

  it('throws ServerError with fields', () => {
    try {
      FfiErrors.fetchData('500');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('ServerError');
      if (err.variant.tag === 'ServerError') {
        expect(err.variant.code).toBe(500);
        expect(err.variant.message).toBe('Internal Server Error');
      }
    }
  });
});

// ---------------------------------------------------------------------------
// Object with throwing constructor
// ---------------------------------------------------------------------------

describe('Parser (object with throws)', () => {
  it('constructor succeeds', () => {
    const p = Parser.create('hello world');
    expect(p.result()).toBe('hello world');
    p.free();
  });

  it('constructor throws InvalidInput on empty string', () => {
    try {
      Parser.create('');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(ParseError);
      expect((e as ParseError).variant.tag).toBe('InvalidInput');
    }
  });

  it('method succeeds', () => {
    const p = Parser.create('section:data');
    expect(p.parseSection('section')).toBe('section');
    p.free();
  });

  it('method throws MissingSection', () => {
    const p = Parser.create('hello world');
    try {
      p.parseSection('nonexistent');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(ParseError);
      expect((e as ParseError).variant.tag).toBe('MissingSection');
    }
    p.free();
  });

  it('free is idempotent', () => {
    const p = Parser.create('test');
    p.free();
    p.free();
  });
});
