import { describe, expect, it } from 'vitest';
import type { Diagnostic, Document } from '../src/codegen/types.js';

describe('structured diagnostic types', () => {
  it('preserves the canonical fields while retaining legacy metadata strings', () => {
    const document = JSON.parse(JSON.stringify({
      schema_version: '1.0',
      metadata: {
        page_count: 1,
        diagnostics: ['zlib stream truncated mid-inflation'],
        diagnostics_detailed: [{
          code: 'STREAM_DECODE_ERROR',
          message: 'zlib stream truncated mid-inflation',
          severity: 'warning',
          page_index: 3,
          location: { object_number: 42, generation_number: 7 },
          hint: 'Inspect the source PDF for corrupt stream data',
        }],
      },
      pages: [],
      errors: [{
        code: 'STREAM_DECODE_ERROR',
        message: 'zlib stream truncated mid-inflation',
        severity: 'warning',
        page_index: 3,
        location: { object_number: 42, generation_number: 7 },
        hint: 'Inspect the source PDF for corrupt stream data',
      }],
    })) as Document;

    const diagnostic: Diagnostic = document.errors[0];
    expect(diagnostic.code).toBe('STREAM_DECODE_ERROR');
    expect(diagnostic.severity).toBe('warning');
    expect(diagnostic.page_index).toBe(3);
    expect(diagnostic.location?.object_number).toBe(42);
    expect(diagnostic.hint).toContain('Inspect');
    expect(document.metadata.diagnostics?.[0]).toBe(diagnostic.message);
  });
});
