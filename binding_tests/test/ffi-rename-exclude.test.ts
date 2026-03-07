/**
 * FFI-mode smoke test: rename and exclude config.
 *
 * Tests that [bindings.js] config correctly renames and excludes
 * functions and types from the generated output.
 */

import { describe, it, expect } from 'vitest';
import { FfiRenameExclude } from '../generated/ffi_rename_exclude.js';
import * as mod from '../generated/ffi_rename_exclude.js';

describe('Rename', () => {
  it('hello is renamed to greet', () => {
    expect(FfiRenameExclude.greet('world')).toBe('hello, world');
  });

  it('greet handles empty string', () => {
    expect(FfiRenameExclude.greet('')).toBe('hello, ');
  });
});

describe('Exclude', () => {
  it('farewell is not exported', () => {
    expect('farewell' in FfiRenameExclude).toBe(false);
  });

  it('InternalState is not exported', () => {
    expect('InternalState' in mod).toBe(false);
  });
});

describe('Non-excluded functions', () => {
  it('version is still exported', () => {
    expect(FfiRenameExclude.version()).toBe('1.0.0');
  });
});
