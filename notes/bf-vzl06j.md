# Bead bf-vzl06j - Code Generation Verification

## Summary

Successfully generated 9 contract methods and 8 error classes using the codegen script.

## Changes Made

### Fixed Codegen Script
- **File**: `scripts/codegen.js`
- **Issue**: `optionsArgs` method didn't handle `undefined` for optional parameters, causing TypeScript compilation errors
- **Fix**: Updated signature from `optionsArgs(options: ExtractOptions | SearchOptions | BaseOptions)` to `optionsArgs(options?: ExtractOptions | SearchOptions | BaseOptions)` and added early return for null/undefined

## Generated Artifacts

### Error Classes (8 total)
1. `PdftractError` - Base error class
2. `CorruptPdfError` - Exit code 2
3. `EncryptionError` - Exit code 3
4. `SourceUnreachableError` - Exit code 4
5. `RemoteFetchInterruptedError` - Exit code 5
6. `TlsError` - Exit code 6
7. `ReceiptVerifyError` - Exit code 10
8. `ValidationError` - Exit code 1

### SDK Methods (9 total)
All generated in `src/codegen/methods.ts`:
1. `extract(source, options?)` - Extract structured data → Document
2. `extractText(source, options?)` - Extract plain text → string
3. `extractMarkdown(source, options?)` - Extract Markdown → string
4. `extractStream(source, options?)` - Stream pages → AsyncIterable<Page>
5. `search(source, pattern, options?)` - Search text → AsyncIterable<Match>
6. `getMetadata(source, options?)` - Get metadata → Metadata
7. `hash(source, options?)` - Compute fingerprint → Fingerprint
8. `classify(source)` - Classify document → Classification
9. `verifyReceipt(path, receipt)` - Verify receipt → boolean

## Acceptance Criteria Status

### PASS ✓
- [x] `npm run codegen` produces `src/codegen/methods.ts` and `src/codegen/errors.ts`
- [x] All 9 methods exported from `src/index.ts` with correct TypeScript types
- [x] All 8 error classes inherit from `PdftractError`
- [x] Generated code compiles without errors
- [x] Unit tests verify method signatures and error inheritance (16/16 tests pass)

### Build Verification
```bash
$ npm run codegen
✓ Generated src/codegen/errors.ts
✓ Generated src/codegen/methods.ts

$ npm run build
ESM dist/esm/index.js     13.51 KB
CJS dist/cjs/index.cjs     15.03 KB
DTS dist/types/index.d.ts  7.83 KB
✓ Build success

$ npm test -- test/unit.test.ts
✓ test/unit.test.ts  (16 tests) 8ms
Test Files  1 passed (1)
     Tests  16 passed (16)
```

## Exports Verified

All error classes properly re-exported from `src/index.ts`:
- PdftractError (base)
- CorruptPdfError, EncryptionError, SourceUnreachableError
- RemoteFetchInterruptedError, TlsError, ReceiptVerifyError, ValidationError

All methods available via `Client` class, plus convenience helpers:
- `Client` class with all 9 methods
- `path()`, `url()`, `bytes()` helper functions

## Notes

The codegen uses JavaScript template strings (not Tera templates, which would be Rust-specific). The implementation correctly handles:
- Optional parameters with proper TypeScript typing
- Async generators for streaming methods (extractStream, search)
- Exit code mapping to specific error classes
- Source argument conversion (PathSource, URLSource, BytesSource)
- Options argument mapping from camelCase to kebab-case CLI flags
