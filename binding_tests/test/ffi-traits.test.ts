/**
 * FFI-mode smoke test: trait interfaces.
 *
 * Tests trait objects returned from Rust, trait methods,
 * passing trait objects back to Rust, and Sequence<Trait>.
 */

import { describe, it, expect } from 'vitest';
import { FfiTraits, Drawable, Circle, Rect } from '../generated/ffi_traits.js';

describe('Trait: make_shapes returns trait objects', () => {
  it('returns a list of Drawable', () => {
    const shapes = FfiTraits.makeShapes();
    expect(shapes).toHaveLength(2);
    shapes.forEach(s => s.free());
  });

  it('trait objects have working methods', () => {
    const shapes = FfiTraits.makeShapes();
    // First is Circle(radius=5)
    expect(shapes[0].describe()).toBe('Circle(radius=5)');
    expect(shapes[0].area()).toBeCloseTo(Math.PI * 25, 5);
    // Second is Rect(3x4)
    expect(shapes[1].describe()).toBe('Rect(3x4)');
    expect(shapes[1].area()).toBeCloseTo(12.0, 5);
    shapes.forEach(s => s.free());
  });
});

describe('Trait: pass trait objects back to Rust', () => {
  it('describe_all', () => {
    const shapes = FfiTraits.makeShapes();
    const desc = FfiTraits.describeAll(shapes);
    expect(desc).toBe('Circle(radius=5), Rect(3x4)');
    // shapes should still be usable after passing
    expect(shapes[0].describe()).toBe('Circle(radius=5)');
    shapes.forEach(s => s.free());
  });

  it('total_area', () => {
    const shapes = FfiTraits.makeShapes();
    const area = FfiTraits.totalArea(shapes);
    expect(area).toBeCloseTo(Math.PI * 25 + 12.0, 5);
    // shapes still usable after passing (clone correctness)
    expect(shapes[0].describe()).toBe('Circle(radius=5)');
    shapes.forEach(s => s.free());
  });

  it('passes same shape in sequence multiple times', () => {
    const shapes = FfiTraits.makeShapes();
    const area = FfiTraits.totalArea([shapes[0], shapes[0], shapes[0]]);
    expect(area).toBeCloseTo(Math.PI * 25 * 3, 5);
    shapes.forEach(s => s.free());
  });
});

describe('Trait: concrete types still work', () => {
  it('Circle has constructor and methods', () => {
    const c = Circle.create(10.0);
    expect(c.radius()).toBe(10.0);
    c.free();
  });

  it('Rect has constructor and methods', () => {
    const r = Rect.create(6.0, 8.0);
    expect(r.width()).toBe(6.0);
    expect(r.height()).toBe(8.0);
    r.free();
  });
});

describe('Trait: Drawable has no public constructor', () => {
  it('Drawable class exists but has no create method', () => {
    expect(Drawable).toBeDefined();
    expect((Drawable as any).create).toBeUndefined();
  });
});
