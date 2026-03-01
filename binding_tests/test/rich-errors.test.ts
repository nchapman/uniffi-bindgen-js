/**
 * Smoke tests for the rich-errors fixture — exercises data-carrying [Error]
 * interface (discriminated union variant with fields).
 *
 * Build generated output with scripts/build_bindings.sh before running.
 */

import { describe, it, expect, beforeAll } from 'vitest';
import { readFile } from 'fs/promises';
import { fileURLToPath } from 'url';
import { dirname, resolve } from 'path';
import { init, NetworkError, RichErrors } from '../generated/rich_errors.js';

const __dirname = dirname(fileURLToPath(import.meta.url));

beforeAll(async () => {
  const wasmPath = resolve(__dirname, '../generated/rich_errors_bg_bg.wasm');
  const bytes = await readFile(wasmPath);
  await init({ module_or_path: bytes });
});

describe('RichErrors.fetchData', () => {
  it('returns data for a valid url', () => {
    expect(RichErrors.fetchData('example.com')).toBe('data for example.com');
  });

  it('throws NetworkError.NotFound for a 404 url', () => {
    try {
      RichErrors.fetchData('404');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('NotFound');
      if (err.variant.tag === 'NotFound') {
        expect(err.variant.url).toBe('404');
      }
    }
  });

  it('throws NetworkError.Timeout for a timeout url', () => {
    try {
      RichErrors.fetchData('timeout');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('Timeout');
      if (err.variant.tag === 'Timeout') {
        expect(err.variant.url).toBe('timeout');
        expect(err.variant.elapsedMs).toBe(5000);
      }
    }
  });

  it('throws NetworkError.ServerError for a 500 url', () => {
    try {
      RichErrors.fetchData('500');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('ServerError');
      if (err.variant.tag === 'ServerError') {
        expect(err.variant.statusCode).toBe(500);
      }
    }
  });

  it('throws NetworkError.Unknown for an unknown url', () => {
    try {
      RichErrors.fetchData('unknown');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('Unknown');
    }
  });
});

describe('RichErrors.fetchWithTimeout', () => {
  it('returns data for a valid url', () => {
    expect(RichErrors.fetchWithTimeout('example.com', 1000)).toBe('data for example.com');
  });

  it('propagates errors through fetchWithTimeout', () => {
    expect(() => RichErrors.fetchWithTimeout('404', 1000)).toThrow(NetworkError);
  });
});

describe('RichErrors.fetchDataAsync (async + throws)', () => {
  it('returns data for a valid url', async () => {
    const result = await RichErrors.fetchDataAsync('example.com');
    expect(result).toBe('data for example.com');
  });

  it('throws NetworkError.NotFound for a 404 url', async () => {
    try {
      await RichErrors.fetchDataAsync('404');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('NotFound');
      if (err.variant.tag === 'NotFound') {
        expect(err.variant.url).toBe('404');
      }
    }
  });

  it('throws NetworkError.Timeout for a timeout url', async () => {
    try {
      await RichErrors.fetchDataAsync('timeout');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('Timeout');
      if (err.variant.tag === 'Timeout') {
        expect(err.variant.url).toBe('timeout');
        expect(err.variant.elapsedMs).toBe(5000);
      }
    }
  });

  it('throws NetworkError.ServerError for a 500 url', async () => {
    try {
      await RichErrors.fetchDataAsync('500');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('ServerError');
      if (err.variant.tag === 'ServerError') {
        expect(err.variant.statusCode).toBe(500);
      }
    }
  });

  it('throws NetworkError.Unknown for an unknown url', async () => {
    try {
      await RichErrors.fetchDataAsync('unknown');
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('Unknown');
    }
  });
});

describe('RichErrors.fetchWithRetryAsync (async + throws, multi-arg)', () => {
  it('returns data for a valid url', async () => {
    const result = await RichErrors.fetchWithRetryAsync('example.com', 3);
    expect(result).toBe('data for example.com');
  });

  it('throws NetworkError.NotFound for a 404 url', async () => {
    try {
      await RichErrors.fetchWithRetryAsync('404', 3);
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('NotFound');
      if (err.variant.tag === 'NotFound') {
        expect(err.variant.url).toBe('404');
      }
    }
  });

  it('throws NetworkError.Timeout for a timeout url', async () => {
    try {
      await RichErrors.fetchWithRetryAsync('timeout', 3);
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('Timeout');
      if (err.variant.tag === 'Timeout') {
        expect(err.variant.url).toBe('timeout');
        expect(err.variant.elapsedMs).toBe(5000);
      }
    }
  });

  it('throws NetworkError.ServerError for a 500 url', async () => {
    try {
      await RichErrors.fetchWithRetryAsync('500', 3);
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('ServerError');
      if (err.variant.tag === 'ServerError') {
        expect(err.variant.statusCode).toBe(500);
      }
    }
  });

  it('throws NetworkError.Unknown for an unknown url', async () => {
    try {
      await RichErrors.fetchWithRetryAsync('unknown', 3);
      expect.fail('should have thrown');
    } catch (e) {
      expect(e).toBeInstanceOf(NetworkError);
      const err = e as NetworkError;
      expect(err.variant.tag).toBe('Unknown');
    }
  });
});
