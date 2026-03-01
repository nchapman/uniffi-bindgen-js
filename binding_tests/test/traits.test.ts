/**
 * Smoke tests for the traits fixture — exercises [Trait] interfaces and the
 * Object._fromInner() lifting path for functions that return object types.
 *
 * Build generated output with scripts/build_bindings.sh before running.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, Drawable, Traits } from '../generated/traits.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  const wasmPath = resolve(__dirname, '../generated/traits_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

describe('Traits.makeCircle', () => {
  it('returns a Drawable instance', () => {
    const d = Traits.makeCircle(1);
    expect(d).toBeInstanceOf(Drawable);
    d.free();
  });

  it('describe returns a circle description', () => {
    const d = Traits.makeCircle(5);
    expect(d.describe()).toBe('circle(r=5)');
    d.free();
  });

  it('area returns π * r²', () => {
    const d = Traits.makeCircle(1);
    expect(d.area()).toBeCloseTo(Math.PI);
    d.free();
  });
});

describe('Traits.makeRect', () => {
  it('returns a Drawable instance', () => {
    const d = Traits.makeRect(3, 4);
    expect(d).toBeInstanceOf(Drawable);
    d.free();
  });

  it('describe returns a rect description', () => {
    const d = Traits.makeRect(3, 4);
    expect(d.describe()).toBe('rect(3x4)');
    d.free();
  });

  it('area returns width * height', () => {
    const d = Traits.makeRect(3, 4);
    expect(d.area()).toBe(12);
    d.free();
  });
});
