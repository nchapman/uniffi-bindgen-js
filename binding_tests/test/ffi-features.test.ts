/**
 * FFI-mode smoke test: feature coverage.
 *
 * Tests default arguments, named constructors, docstrings (via generated
 * output shape), reserved-word escaping, records with defaults, flat enums,
 * and error handling on named constructors.
 */

import { describe, it, expect } from 'vitest';
import {
  FfiFeatures,
  Widget,
  BuildError,
  type Config,
  type ReturnValue,
  type Status,
} from '../generated/ffi_features.js';

describe('Default arguments', () => {
  it('greet with no argument uses default', () => {
    expect(FfiFeatures.greet()).toBe('Hello, world!');
  });

  it('greet with explicit name', () => {
    expect(FfiFeatures.greet('Alice')).toBe('Hello, Alice!');
  });

  it('greet with explicit null', () => {
    expect(FfiFeatures.greet(null)).toBe('Hello, world!');
  });

  it('add_maybe with no second argument', () => {
    expect(FfiFeatures.addMaybe(5)).toBe(5);
  });

  it('add_maybe with explicit second argument', () => {
    expect(FfiFeatures.addMaybe(5, 3)).toBe(8);
  });

  it('add_maybe with explicit null', () => {
    expect(FfiFeatures.addMaybe(5, null)).toBe(5);
  });
});

describe('Reserved-word function names', () => {
  it('class_ function', () => {
    expect(FfiFeatures.class_(10)).toBe(11);
  });

  it('return_ function', () => {
    expect(FfiFeatures.return_('hello')).toBe('returned: hello');
  });

  it('delete_ function', () => {
    expect(FfiFeatures.delete_(true)).toBe(false);
    expect(FfiFeatures.delete_(false)).toBe(true);
  });
});

describe('Reserved-word record fields', () => {
  it('describe_keywords round-trips record', () => {
    const rv: ReturnValue = { class_: 'foo', return_: 42, typeof_: true };
    expect(FfiFeatures.describeKeywords(rv)).toBe('class=foo, return=42, typeof=true');
  });
});

describe('Flat enum', () => {
  it('status_name Active', () => {
    expect(FfiFeatures.statusName('Active')).toBe('Active');
  });

  it('status_name Inactive', () => {
    expect(FfiFeatures.statusName('Inactive')).toBe('Inactive');
  });
});

describe('Named constructors', () => {
  it('default constructor', () => {
    const w = Widget.create('test');
    expect(w.getLabel()).toBe('test');
    w.free();
  });

  it('named constructor success', () => {
    const w = Widget.fromPositive(42);
    expect(w.getLabel()).toBe('widget-42');
    w.free();
  });

  it('named constructor throws on negative', () => {
    expect(() => Widget.fromPositive(-1)).toThrow(BuildError);
    try {
      Widget.fromPositive(-1);
    } catch (e) {
      expect((e as BuildError).tag).toBe('InvalidInput');
    }
  });

  it('named constructor throws on overflow', () => {
    expect(() => Widget.fromPositive(2_000_000)).toThrow(BuildError);
    try {
      Widget.fromPositive(2_000_000);
    } catch (e) {
      expect((e as BuildError).tag).toBe('Overflow');
    }
  });
});

describe('Reserved-word method names', () => {
  it('class_ method on Widget', () => {
    const w = Widget.fromPositive(99);
    expect(w.class_()).toBe(99);
    w.free();
  });
});

describe('Method with default argument', () => {
  it('format with no prefix', () => {
    const w = Widget.create('hello');
    expect(w.format()).toBe('hello');
    w.free();
  });

  it('format with explicit prefix', () => {
    const w = Widget.create('hello');
    expect(w.format('prefix')).toBe('prefix: hello');
    w.free();
  });
});

describe('Record with field defaults', () => {
  it('get_config returns record with all fields', () => {
    const w = Widget.create('mywidget');
    const cfg = w.getConfig();
    expect(cfg.host).toBe('localhost');
    expect(cfg.port).toBe(8080);
    expect(cfg.verbose).toBe(false);
    expect(cfg.label).toBe('mywidget');
    w.free();
  });
});
