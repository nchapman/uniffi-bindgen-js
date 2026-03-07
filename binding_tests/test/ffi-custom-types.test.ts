/**
 * FFI-mode smoke test: custom type aliases.
 *
 * Tests that custom types (Url → URL, Handle → bigint) are correctly
 * lowered and lifted through the FFI boundary with user-defined conversions.
 */

import { describe, it, expect } from 'vitest';
import { FfiCustomTypes, type Url, type Handle } from '../generated/ffi_custom_types.js';

describe('Custom type: Url (string → URL)', () => {
  it('lowercases a URL', () => {
    const result: Url = FfiCustomTypes.normalizeUrl(new URL('HTTP://EXAMPLE.COM/Path'));
    expect(result).toBeInstanceOf(URL);
    expect(result.href).toBe('http://example.com/path');
  });

  it('preserves origin-only URL', () => {
    const result = FfiCustomTypes.normalizeUrl(new URL('http://example.com/'));
    expect(result).toBeInstanceOf(URL);
    expect(result.origin).toBe('http://example.com');
  });

  it('is a no-op on an already-normalized URL', () => {
    const result = FfiCustomTypes.normalizeUrl(new URL('http://example.com/path'));
    expect(result.href).toBe('http://example.com/path');
  });
});

describe('Custom type: Handle (i64 → bigint)', () => {
  it('returns a deterministic handle from a seed', () => {
    const h: Handle = FfiCustomTypes.makeHandle(1n);
    expect(h).toBe(42n);
  });

  it('returns different handles for different seeds', () => {
    const h1 = FfiCustomTypes.makeHandle(1n);
    const h2 = FfiCustomTypes.makeHandle(2n);
    expect(h1).not.toBe(h2);
  });

  it('handles_equal returns true for equal handles', () => {
    const h1 = FfiCustomTypes.makeHandle(5n);
    const h2 = FfiCustomTypes.makeHandle(5n);
    expect(FfiCustomTypes.handlesEqual(h1, h2)).toBe(true);
  });

  it('handles_equal returns false for different handles', () => {
    const h1 = FfiCustomTypes.makeHandle(1n);
    const h2 = FfiCustomTypes.makeHandle(2n);
    expect(FfiCustomTypes.handlesEqual(h1, h2)).toBe(false);
  });
});
