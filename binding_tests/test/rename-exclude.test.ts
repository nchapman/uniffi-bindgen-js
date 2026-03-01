/**
 * Smoke tests for the rename-exclude fixture — verifies that:
 * - `hello` (renamed to `greet` in config) calls through to __bg.hello correctly
 * - `farewell` (excluded in config) is not exported
 * - `version` (unchanged) works normally
 *
 * Build generated output with scripts/build_bindings.sh before running.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, RenameExclude } from '../generated/rename_exclude.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  const wasmPath = resolve(__dirname, '../generated/rename_exclude_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

describe('RenameExclude.greet (renamed from hello)', () => {
  it('calls through to the underlying hello wasm function', () => {
    expect(RenameExclude.greet('world')).toBe('hello, world');
  });

  it('handles empty string', () => {
    expect(RenameExclude.greet('')).toBe('hello, ');
  });
});

describe('RenameExclude.version', () => {
  it('returns the version string', () => {
    expect(RenameExclude.version()).toBe('1.0.0');
  });
});

describe('excluded exports', () => {
  it('farewell is not exported', () => {
    expect((RenameExclude as Record<string, unknown>)['farewell']).toBeUndefined();
  });
});
