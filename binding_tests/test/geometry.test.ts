/**
 * Smoke tests for the geometry fixture — exercises records (Point), flat enums
 * (Direction), and data-carrying enums (Shape).
 *
 * Build generated output with scripts/build_bindings.sh before running.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, type Point, type Shape, Geometry } from '../generated/geometry.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  const wasmPath = resolve(__dirname, '../generated/geometry_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

describe('Geometry.translate', () => {
  it('translates a point', () => {
    const p: Point = { x: 1, y: 2 };
    const result = Geometry.translate(p, 3, 4);
    expect(result).toEqual({ x: 4, y: 6 });
  });

  it('translates by zero', () => {
    const p: Point = { x: 5, y: -3 };
    const result = Geometry.translate(p, 0, 0);
    expect(result).toEqual({ x: 5, y: -3 });
  });

  it('translates by negative deltas', () => {
    const p: Point = { x: 10, y: 10 };
    const result = Geometry.translate(p, -3, -7);
    expect(result).toEqual({ x: 7, y: 3 });
  });
});

describe('Geometry.step', () => {
  it('steps North increases y', () => {
    expect(Geometry.step({ x: 0, y: 0 }, 'North')).toEqual({ x: 0, y: 1 });
  });

  it('steps South decreases y', () => {
    expect(Geometry.step({ x: 0, y: 5 }, 'South')).toEqual({ x: 0, y: 4 });
  });

  it('steps East increases x', () => {
    expect(Geometry.step({ x: 3, y: 0 }, 'East')).toEqual({ x: 4, y: 0 });
  });

  it('steps West decreases x', () => {
    expect(Geometry.step({ x: 3, y: 0 }, 'West')).toEqual({ x: 2, y: 0 });
  });
});

describe('Geometry.area', () => {
  it('computes circle area', () => {
    const shape: Shape = { tag: 'Circle', radius: 1 };
    expect(Geometry.area(shape)).toBeCloseTo(Math.PI);
  });

  it('computes rectangle area', () => {
    const shape: Shape = { tag: 'Rectangle', width: 4, height: 5 };
    expect(Geometry.area(shape)).toBe(20);
  });

  it('returns 0 for a point shape', () => {
    const shape: Shape = { tag: 'Point' };
    expect(Geometry.area(shape)).toBe(0);
  });
});
