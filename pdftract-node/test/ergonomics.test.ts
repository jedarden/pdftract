/**
 * Unit tests for ergonomics.ts
 */

import { describe, it, expect } from 'vitest';
import {
  normalizeOptions,
  coerceSource,
  validateOptions,
  mergeOptions,
  lazyOptions,
  isUrl,
  isBuffer,
} from '../src/ergonomics.js';
import type { ExtractOptions, SearchOptions, BaseOptions } from '../src/codegen/types.js';

describe('ergonomics', () => {
  describe('normalizeOptions', () => {
    it('should return empty array for undefined options', () => {
      const args = normalizeOptions(undefined);
      expect(args).toEqual([]);
    });

    it('should return empty array for null options', () => {
      const args = normalizeOptions(null as any);
      expect(args).toEqual([]);
    });

    it('should return empty array for empty object', () => {
      const args = normalizeOptions({});
      expect(args).toEqual([]);
    });

    describe('ExtractOptions', () => {
      it('should normalize ocrLanguage', () => {
        const args = normalizeOptions({ ocrLanguage: 'eng' });
        expect(args).toEqual(['--ocr-language', 'eng']);
      });

      it('should normalize ocrThreshold', () => {
        const args = normalizeOptions({ ocrThreshold: 80 });
        expect(args).toEqual(['--ocr-threshold', '80']);
      });

      it('should normalize preserveLayout flag', () => {
        const args = normalizeOptions({ preserveLayout: true });
        expect(args).toEqual(['--preserve-layout']);
      });

      it('should normalize extractImages flag', () => {
        const args = normalizeOptions({ extractImages: true });
        expect(args).toEqual(['--extract-images']);
      });

      it('should normalize imageFormat', () => {
        const args = normalizeOptions({ imageFormat: 'png' });
        expect(args).toEqual(['--image-format', 'png']);
      });

      it('should normalize minImageSize', () => {
        const args = normalizeOptions({ minImageSize: 100 });
        expect(args).toEqual(['--min-image-size', '100']);
      });

      it('should normalize password', () => {
        const args = normalizeOptions({ password: 'secret123' });
        expect(args).toEqual(['--password', 'secret123']);
      });

      it('should normalize multiple options', () => {
        const args = normalizeOptions({
          ocrLanguage: 'eng',
          ocrThreshold: 75,
          preserveLayout: true,
          extractImages: true,
          imageFormat: 'jpeg',
          minImageSize: 50,
          password: 'test',
        });
        expect(args).toEqual([
          '--ocr-language',
          'eng',
          '--ocr-threshold',
          '75',
          '--preserve-layout',
          '--extract-images',
          '--image-format',
          'jpeg',
          '--min-image-size',
          '50',
          '--password',
          'test',
        ]);
      });

      it('should skip undefined options', () => {
        const args = normalizeOptions({
          ocrLanguage: 'eng',
          ocrThreshold: undefined,
          preserveLayout: true,
          extractImages: undefined,
        });
        expect(args).toEqual(['--ocr-language', 'eng', '--preserve-layout']);
      });
    });

    describe('SearchOptions', () => {
      it('should normalize caseInsensitive flag', () => {
        const args = normalizeOptions({ caseInsensitive: true });
        expect(args).toEqual(['--case-insensitive']);
      });

      it('should normalize regex flag', () => {
        const args = normalizeOptions({ regex: true });
        expect(args).toEqual(['--regex']);
      });

      it('should normalize wholeWord flag', () => {
        const args = normalizeOptions({ wholeWord: true });
        expect(args).toEqual(['--whole-word']);
      });

      it('should normalize maxResults', () => {
        const args = normalizeOptions({ maxResults: 100 });
        expect(args).toEqual(['--max-results', '100']);
      });

      it('should normalize multiple search options', () => {
        const args = normalizeOptions({
          caseInsensitive: true,
          regex: true,
          wholeWord: true,
          maxResults: 50,
        });
        expect(args).toEqual([
          '--case-insensitive',
          '--regex',
          '--whole-word',
          '--max-results',
          '50',
        ]);
      });
    });

    describe('BaseOptions', () => {
      it('should normalize timeout', () => {
        const args = normalizeOptions({ timeout: 30000 });
        expect(args).toEqual(['--timeout', '30000']);
      });
    });
  });

  describe('coerceSource', () => {
    it('should return URL object as-is', () => {
      const url = new URL('https://example.com/doc.pdf');
      const result = coerceSource(url);
      expect(result).toBe(url);
    });

    it('should handle string URLs', () => {
      const result = coerceSource('https://example.com/doc.pdf');
      expect(result).toBe('https://example.com/doc.pdf');
    });

    it('should handle file path strings', () => {
      const result = coerceSource('/path/to/doc.pdf');
      expect(result).toBe('/path/to/doc.pdf');
    });

    it('should handle Buffers', () => {
      const buffer = Buffer.from('test pdf content');
      const result = coerceSource(buffer);
      expect(result).toBe(buffer);
    });

    it('should handle Source objects with toArgs method', () => {
      const source = {
        toArgs: () => ['path/to/doc.pdf'],
      };
      const result = coerceSource(source);
      expect(result).toBe(source);
    });

    it('should throw TypeError for invalid source type', () => {
      expect(() => coerceSource(12345)).toThrow(TypeError);
      expect(() => coerceSource(null)).toThrow(TypeError);
      expect(() => coerceSource({})).toThrow(TypeError);
    });
  });

  describe('validateOptions', () => {
    it('should accept valid options', () => {
      expect(() => {
        validateOptions({ ocrLanguage: 'eng', ocrThreshold: 50 });
      }).not.toThrow();
    });

    it('should throw for non-object options', () => {
      expect(() => validateOptions(null as any)).toThrow(TypeError);
      expect(() => validateOptions('string' as any)).toThrow(TypeError);
      expect(() => validateOptions(123 as any)).toThrow(TypeError);
    });

    describe('numeric validation', () => {
      it('should validate ocrThreshold range', () => {
        expect(() => validateOptions({ ocrThreshold: -1 })).toThrow(TypeError);
        expect(() => validateOptions({ ocrThreshold: 101 })).toThrow(TypeError);
        expect(() => validateOptions({ ocrThreshold: 50 })).not.toThrow();
        expect(() => validateOptions({ ocrThreshold: 0 })).not.toThrow();
        expect(() => validateOptions({ ocrThreshold: 100 })).not.toThrow();
      });

      it('should validate minImageSize is non-negative', () => {
        expect(() => validateOptions({ minImageSize: -1 })).toThrow(TypeError);
        expect(() => validateOptions({ minImageSize: 0 })).not.toThrow();
        expect(() => validateOptions({ minImageSize: 100 })).not.toThrow();
      });

      it('should validate maxResults is non-negative', () => {
        expect(() => validateOptions({ maxResults: -1 })).toThrow(TypeError);
        expect(() => validateOptions({ maxResults: 0 })).not.toThrow();
        expect(() => validateOptions({ maxResults: 1000 })).not.toThrow();
      });

      it('should validate timeout is non-negative', () => {
        expect(() => validateOptions({ timeout: -1 })).toThrow(TypeError);
        expect(() => validateOptions({ timeout: 0 })).not.toThrow();
        expect(() => validateOptions({ timeout: 60000 })).not.toThrow();
      });

      it('should reject non-numeric values for numeric options', () => {
        expect(() => validateOptions({ ocrThreshold: '50' as any })).toThrow(TypeError);
        expect(() => validateOptions({ minImageSize: '100' as any })).toThrow(TypeError);
        expect(() => validateOptions({ maxResults: '10' as any })).toThrow(TypeError);
        expect(() => validateOptions({ timeout: '30' as any })).toThrow(TypeError);
      });
    });

    describe('string validation', () => {
      it('should validate ocrLanguage is non-empty string', () => {
        expect(() => validateOptions({ ocrLanguage: '' })).toThrow(TypeError);
        expect(() => validateOptions({ ocrLanguage: 'eng' })).not.toThrow();
        expect(() => validateOptions({ ocrLanguage: 'eng+fra' })).not.toThrow();
      });

      it('should reject non-string ocrLanguage', () => {
        expect(() => validateOptions({ ocrLanguage: 123 as any })).toThrow(TypeError);
        expect(() => validateOptions({ ocrLanguage: null as any })).toThrow(TypeError);
      });

      it('should validate imageFormat is allowed value', () => {
        expect(() => validateOptions({ imageFormat: 'png' })).not.toThrow();
        expect(() => validateOptions({ imageFormat: 'jpeg' })).not.toThrow();
        expect(() => validateOptions({ imageFormat: 'webp' })).not.toThrow();
        expect(() => validateOptions({ imageFormat: 'gif' })).toThrow(TypeError);
        expect(() => validateOptions({ imageFormat: '' })).toThrow(TypeError);
        expect(() => validateOptions({ imageFormat: 'PNG' as any })).toThrow(TypeError);
      });

      it('should reject non-string imageFormat', () => {
        expect(() => validateOptions({ imageFormat: 123 as any })).toThrow(TypeError);
      });
    });
  });

  describe('mergeOptions', () => {
    it('should merge empty options', () => {
      const defaults: ExtractOptions = {};
      const overrides: Partial<ExtractOptions> = {};
      const result = mergeOptions(defaults, overrides);
      expect(result).toEqual({});
    });

    it('should return defaults when no overrides', () => {
      const defaults: ExtractOptions = { ocrLanguage: 'eng', ocrThreshold: 50 };
      const overrides: Partial<ExtractOptions> = {};
      const result = mergeOptions(defaults, overrides);
      expect(result).toEqual({ ocrLanguage: 'eng', ocrThreshold: 50 });
    });

    it('should override defaults with provided values', () => {
      const defaults: ExtractOptions = { ocrLanguage: 'eng', ocrThreshold: 50 };
      const overrides: Partial<ExtractOptions> = { ocrThreshold: 75 };
      const result = mergeOptions(defaults, overrides);
      expect(result).toEqual({ ocrLanguage: 'eng', ocrThreshold: 75 });
    });

    it('should add new options from overrides', () => {
      const defaults: ExtractOptions = { ocrLanguage: 'eng' };
      const overrides: Partial<ExtractOptions> = { preserveLayout: true };
      const result = mergeOptions(defaults, overrides);
      expect(result).toEqual({ ocrLanguage: 'eng', preserveLayout: true });
    });

    it('should not mutate original objects', () => {
      const defaults: ExtractOptions = { ocrLanguage: 'eng', ocrThreshold: 50 };
      const defaultsCopy = { ...defaults };
      const overrides: Partial<ExtractOptions> = { ocrThreshold: 75 };
      const overridesCopy = { ...overrides };

      mergeOptions(defaults, overrides);

      expect(defaults).toEqual(defaultsCopy);
      expect(overrides).toEqual(overridesCopy);
    });
  });

  describe('lazyOptions', () => {
    it('should call factory function immediately', () => {
      let called = false;
      const options = lazyOptions(() => {
        called = true;
        return { ocrLanguage: 'eng' };
      });

      expect(called).toBe(true);
      expect(options).toEqual({ ocrLanguage: 'eng' });
    });

    it('should return factory result', () => {
      const options = lazyOptions(() => ({
        ocrThreshold: 75,
        preserveLayout: true,
      }));

      expect(options).toEqual({
        ocrThreshold: 75,
        preserveLayout: true,
      });
    });
  });

  describe('isUrl', () => {
    it('should identify http URLs', () => {
      expect(isUrl('http://example.com')).toBe(true);
      expect(isUrl('http://example.com/doc.pdf')).toBe(true);
    });

    it('should identify https URLs', () => {
      expect(isUrl('https://example.com')).toBe(true);
      expect(isUrl('https://example.com/path?query=1')).toBe(true);
    });

    it('should reject non-URL strings', () => {
      expect(isUrl('/path/to/file.pdf')).toBe(false);
      expect(isUrl('file.pdf')).toBe(false);
      expect(isUrl('ftp://example.com')).toBe(false);
      expect(isUrl('')).toBe(false);
    });
  });

  describe('isBuffer', () => {
    it('should identify Buffers', () => {
      const buffer = Buffer.from('test');
      expect(isBuffer(buffer)).toBe(true);
    });

    it('should reject non-Buffers', () => {
      expect(isBuffer('string')).toBe(false);
      expect(isBuffer(123)).toBe(false);
      expect(isBuffer({})).toBe(false);
      expect(isBuffer(null)).toBe(false);
      expect(isBuffer(undefined)).toBe(false);
    });
  });

  describe('real-world option combinations', () => {
    it('should handle typical OCR options', () => {
      const options: ExtractOptions = {
        ocrLanguage: 'eng+fra',
        ocrThreshold: 70,
        preserveLayout: true,
      };

      const args = normalizeOptions(options);
      expect(args).toEqual([
        '--ocr-language',
        'eng+fra',
        '--ocr-threshold',
        '70',
        '--preserve-layout',
      ]);
    });

    it('should handle image extraction options', () => {
      const options: ExtractOptions = {
        extractImages: true,
        imageFormat: 'jpeg',
        minImageSize: 200,
      };

      const args = normalizeOptions(options);
      expect(args).toEqual([
        '--extract-images',
        '--image-format',
        'jpeg',
        '--min-image-size',
        '200',
      ]);
    });

    it('should handle search with regex', () => {
      const options: SearchOptions = {
        caseInsensitive: true,
        regex: true,
        maxResults: 1000,
      };

      const args = normalizeOptions(options);
      expect(args).toEqual(['--case-insensitive', '--regex', '--max-results', '1000']);
    });

    it('should handle timeout with other options', () => {
      const options: ExtractOptions & BaseOptions = {
        ocrLanguage: 'deu',
        timeout: 60000,
      };

      const args = normalizeOptions(options);
      expect(args).toEqual(['--ocr-language', 'deu', '--timeout', '60000']);
    });
  });
});
