/**
 * Smoke tests for the custom-types fixture — exercises [Custom] typedef
 * support (Url = string, Handle = bigint).
 *
 * Build generated output with scripts/build_bindings.sh before running.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, type Url, type Handle, CustomTypes } from '../generated/custom_types.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  const wasmPath = resolve(__dirname, '../generated/custom_types_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

describe('CustomTypes.normalizeUrl', () => {
  it('lowercases a URL', () => {
    const result: Url = CustomTypes.normalizeUrl('HTTPS://Example.COM/path/');
    expect(result).toBe('https://example.com/path');
  });

  it('removes trailing slash', () => {
    expect(CustomTypes.normalizeUrl('https://example.com/')).toBe('https://example.com');
  });

  it('is a no-op on an already-normalised URL', () => {
    expect(CustomTypes.normalizeUrl('https://example.com')).toBe('https://example.com');
  });
});

describe('CustomTypes.makeHandle', () => {
  it('returns a deterministic handle from a seed', () => {
    const h1: Handle = CustomTypes.makeHandle(1n);
    const h2: Handle = CustomTypes.makeHandle(1n);
    expect(h1).toBe(h2);
  });

  it('returns different handles for different seeds', () => {
    expect(CustomTypes.makeHandle(1n)).not.toBe(CustomTypes.makeHandle(2n));
  });
});

describe('CustomTypes.handlesEqual', () => {
  it('returns true for equal handles', () => {
    const h = CustomTypes.makeHandle(42n);
    expect(CustomTypes.handlesEqual(h, h)).toBe(true);
  });

  it('returns false for different handles', () => {
    const h1 = CustomTypes.makeHandle(1n);
    const h2 = CustomTypes.makeHandle(2n);
    expect(CustomTypes.handlesEqual(h1, h2)).toBe(false);
  });
});
