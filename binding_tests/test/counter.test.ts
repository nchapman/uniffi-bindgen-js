/**
 * Smoke tests for the counter fixture — exercises object/interface support.
 *
 * Build generated output with scripts/build_bindings.sh before running.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, Counter } from '../generated/counter.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  const wasmPath = resolve(__dirname, '../generated/counter_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

describe('Counter', () => {
  it('starts at the given value', () => {
    const c = Counter.new(0n);
    expect(c.get()).toBe(0n);
    c.free();
  });

  it('increments correctly', () => {
    const c = Counter.new(10n);
    c.increment();
    c.increment();
    expect(c.get()).toBe(12n);
    c.free();
  });

  it('decrements correctly', () => {
    const c = Counter.new(5n);
    c.decrement();
    expect(c.get()).toBe(4n);
    c.free();
  });

  it('handles negative values', () => {
    const c = Counter.new(-1n);
    c.decrement();
    expect(c.get()).toBe(-2n);
    c.free();
  });

  it('resets to a given value (exercises multi-word method camelCase)', () => {
    const c = Counter.new(0n);
    c.resetTo(42n);
    expect(c.get()).toBe(42n);
    c.free();
  });

  it('double-free does not throw', () => {
    const c = Counter.new(0n);
    c.free();
    expect(() => c.free()).not.toThrow();
  });

  it('use after free throws', () => {
    const c = Counter.new(0n);
    c.free();
    expect(() => c.get()).toThrow(/has been freed/);
  });
});
