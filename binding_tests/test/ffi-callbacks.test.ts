/**
 * FFI-mode smoke test: callback interfaces via VTable registration.
 *
 * Tests that JS objects implementing a callback interface can be passed
 * to Rust and called back through the VTable mechanism.
 *
 * Prerequisites: run `./scripts/build_bindings.sh` to populate
 * binding_tests/generated/ with ffi_callbacks.ts, ffi_callbacks.wasm, and uniffi_runtime.ts.
 */

import { describe, it, expect } from 'vitest';
import { Processor, type Formatter } from '../generated/ffi_callbacks.js';

describe('Callback interface: Formatter', () => {
  it('basic format callback', () => {
    const formatter: Formatter = {
      format: (input: string) => `formatted: ${input}`,
    };
    const proc = Processor.create(formatter);
    expect(proc.process('hello')).toBe('formatted: hello');
    proc.free();
  });

  it('handles empty string', () => {
    const formatter: Formatter = {
      format: (input: string) => `[${input}]`,
    };
    const proc = Processor.create(formatter);
    expect(proc.process('')).toBe('[]');
    proc.free();
  });

  it('handles unicode', () => {
    const formatter: Formatter = {
      format: (input: string) => input.toUpperCase(),
    };
    const proc = Processor.create(formatter);
    expect(proc.process('café')).toBe('CAFÉ');
    proc.free();
  });

  it('multiple calls through same callback', () => {
    let callCount = 0;
    const formatter: Formatter = {
      format: (input: string) => {
        callCount++;
        return `${callCount}: ${input}`;
      },
    };
    const proc = Processor.create(formatter);
    expect(proc.process('a')).toBe('1: a');
    expect(proc.process('b')).toBe('2: b');
    expect(proc.process('c')).toBe('3: c');
    expect(callCount).toBe(3);
    proc.free();
  });

  it('different formatters produce different results', () => {
    const upper: Formatter = { format: (s) => s.toUpperCase() };
    const lower: Formatter = { format: (s) => s.toLowerCase() };

    const procUpper = Processor.create(upper);
    const procLower = Processor.create(lower);

    expect(procUpper.process('Hello')).toBe('HELLO');
    expect(procLower.process('Hello')).toBe('hello');

    procUpper.free();
    procLower.free();
  });
});
