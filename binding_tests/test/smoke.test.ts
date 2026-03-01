/**
 * Phase 1 smoke test: verify the generated SimpleFns bindings can call
 * through the WASM module and return correct values.
 *
 * The WASM binary and JS glue (simple_fns_bg.js, simple_fns_bg_bg.wasm)
 * must be present in binding_tests/generated/ before running.  The build
 * script populates that directory automatically.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, SimpleFns } from '../generated/simple_fns.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  const wasmPath = resolve(__dirname, '../generated/simple_fns_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes.buffer });
});

describe('SimpleFns.greet', () => {
  it('returns a greeting for a non-empty name', () => {
    const result = SimpleFns.greet('world');
    expect(result).toBe('hello, world');
  });

  it('returns a greeting for an empty string', () => {
    const result = SimpleFns.greet('');
    expect(result).toBe('hello, ');
  });

  it('handles unicode in names', () => {
    const result = SimpleFns.greet('🌍');
    expect(result).toBe('hello, 🌍');
  });
});
