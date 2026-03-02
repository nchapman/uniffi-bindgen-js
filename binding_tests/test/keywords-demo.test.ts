/**
 * Smoke tests for the keywords-demo fixture — exercises reserved-word
 * escaping for namespace functions, object methods, record fields, and enum
 * variants.
 *
 * Build generated output with scripts/build_bindings.sh before running.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, KeywordsDemo, SuperWidget, ThrowKind } from '../generated/keywords_demo.js';
import type { ReturnValue, AsyncKind } from '../generated/keywords_demo.js';
import * as bg from '../generated/keywords_demo_bg.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  // wasm-pack appends _bg to our --out-name, resulting in the double _bg suffix
  const wasmPath = resolve(__dirname, '../generated/keywords_demo_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

// --- Namespace functions with reserved-word names ---

describe('KeywordsDemo namespace functions', () => {
  it('class_ increments by 1', () => {
    expect(KeywordsDemo.class_(10)).toBe(11);
    expect(KeywordsDemo.class_(0)).toBe(1);
  });

  it('return_ wraps a string', () => {
    expect(KeywordsDemo.return_('hello')).toBe('returned: hello');
    expect(KeywordsDemo.return_('')).toBe('returned: ');
  });

  it('delete_ negates a boolean', () => {
    expect(KeywordsDemo.delete_(true)).toBe(false);
    expect(KeywordsDemo.delete_(false)).toBe(true);
  });
});

// --- Object with reserved-word method names ---

describe('SuperWidget', () => {
  it('constructs and tracks state', () => {
    const w = SuperWidget.new();
    expect(w.return_()).toBe(0);
    w.free();
  });

  it('class_ increments counter and formats output', () => {
    const w = SuperWidget.new();
    expect(w.class_('x')).toBe('widget(1):x');
    expect(w.class_('y')).toBe('widget(2):y');
    expect(w.return_()).toBe(2);
    w.free();
  });

  it('throws after free', () => {
    const w = SuperWidget.new();
    w.free();
    expect(() => w.class_('z')).toThrow();
  });

  it('double free is safe', () => {
    const w = SuperWidget.new();
    w.free();
    expect(() => w.free()).not.toThrow();
  });
});

// --- ReturnValue record with reserved-word field names ---

describe('ReturnValue record', () => {
  it('creates a record with reserved-word fields', () => {
    const rv: ReturnValue = bg.make_return_value('hello', 42, true);
    expect(rv.class_).toBe('hello');
    expect(rv.return_).toBe(42);
    expect(rv.typeof_).toBe(true);
  });

  it('handles empty string and zero values', () => {
    const rv: ReturnValue = bg.make_return_value('', 0, false);
    expect(rv.class_).toBe('');
    expect(rv.return_).toBe(0);
    expect(rv.typeof_).toBe(false);
  });
});

// --- AsyncKind flat enum with reserved-word variants ---

describe('AsyncKind enum', () => {
  it('echoes void variant', () => {
    const result: AsyncKind = bg.echo_async_kind('void');
    expect(result).toBe('void');
  });

  it('echoes yield variant', () => {
    const result: AsyncKind = bg.echo_async_kind('yield');
    expect(result).toBe('yield');
  });

  it('echoes await variant', () => {
    const result: AsyncKind = bg.echo_async_kind('await');
    expect(result).toBe('await');
  });
});

// --- ThrowKind error enum with reserved-word variants ---

describe('ThrowKind error', () => {
  it('throws catch variant', () => {
    expect(() => bg.throw_kind('catch')).toThrow();
  });

  it('throws finally variant', () => {
    expect(() => bg.throw_kind('finally')).toThrow();
  });

  it('ThrowKind factory methods produce correct tags', () => {
    const c = ThrowKind.catch_();
    expect(c).toBeInstanceOf(ThrowKind);
    expect(c.tag).toBe('catch');

    const f = ThrowKind.finally_();
    expect(f).toBeInstanceOf(ThrowKind);
    expect(f.tag).toBe('finally');
  });
});
