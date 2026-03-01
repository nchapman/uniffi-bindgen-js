/**
 * Smoke tests for the callbacks fixture — exercises callback interface
 * support where a JS object implementing an interface is passed into WASM.
 *
 * Build generated output with scripts/build_bindings.sh before running.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, Formatter, Callbacks } from '../generated/callbacks.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

/** A simple Formatter implementation that wraps input in brackets. */
const bracketFormatter: Formatter = {
  format(input: string) {
    return `[${input}]`;
  },
  formatWithPrefix(prefix: string, input: string) {
    return `${prefix}: ${input}`;
  },
};

beforeAll(async () => {
  const wasmPath = resolve(__dirname, '../generated/callbacks_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

describe('Callbacks.applyFormatter', () => {
  it('calls the format method on the provided callback', () => {
    expect(Callbacks.applyFormatter(bracketFormatter, 'hello')).toBe('[hello]');
  });

  it('works with a different formatter implementation', () => {
    const upper: Formatter = {
      format: (s) => s.toUpperCase(),
      formatWithPrefix: (p, s) => `${p.toUpperCase()}: ${s.toUpperCase()}`,
    };
    expect(Callbacks.applyFormatter(upper, 'world')).toBe('WORLD');
  });
});

describe('Callbacks.formatGreeting', () => {
  it('calls formatWithPrefix with "Hello" as the prefix', () => {
    expect(Callbacks.formatGreeting(bracketFormatter, 'Alice')).toBe('Hello: Alice');
  });
});
