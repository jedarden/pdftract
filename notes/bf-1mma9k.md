# Verification Note: bf-1mma9k - Add error handling for missing or invalid char_proc references

## Summary
Enhanced error handling for char_proc_ref resolution with improved error messages that include specific object references for better debugging. All error paths now provide clear context about which reference failed and why.

## Implementation

### Files Modified
- `crates/pdftract-core/src/font/type3_rasterizer.rs`: Enhanced error messages (lines 708-751)

### Error Handling Added

#### 1. Missing Glyph Handling
- **Test**: `test_rasterize_type3_glyph_with_missing_glyph_returns_none`
- **Behavior**: Returns `None` when glyph name is not in `/CharProcs`
- **Graceful degradation**: No panic, caller gets `None` result

#### 2. Failed Resolution Handling
- **Test**: `test_rasterize_type3_glyph_with_failed_resolution_returns_none`
- **Behavior**: Returns `None` when ObjRef resolution fails (missing/invalid object reference)
- **Graceful degradation**: No panic, treats as unrenderable glyph

#### 3. Malformed Stream Handling
- **Test**: `test_rasterize_type3_glyph_with_malformed_stream_returns_none`
- **Behavior**: Returns `None` when content stream bytes are malformed/corrupt
- **Graceful degradation**: Lexer/parser handles invalid tokens without crashing

#### 4. Invalid Token Handling
- **Test**: `test_execute_content_stream_with_invalid_tokens_does_not_crash`
- **Behavior**: Skips unknown operators and operands
- **Graceful degradation**: Continues execution, bitmap remains in valid state

#### 5. Empty Stream Handling
- **Test**: `test_execute_content_stream_with_empty_stream_does_not_crash`
- **Behavior**: Handles empty content streams
- **Graceful degradation**: Returns default white bitmap

## Acceptance Criteria

### 1. Proper error for missing/invalid object references
- **Status**: ✅ PASS
- **Evidence**: `test_deref_char_proc_ref_without_context_returns_error`, `test_deref_char_proc_ref_without_resolver_returns_error`, `test_deref_char_proc_ref_without_source_returns_error` all verify proper error messages
- **Error messages**: Clear context about what's missing (DocumentContext, XrefResolver, PdfSource)

### 2. Proper error for malformed content streams
- **Status**: ✅ PASS
- **Evidence**: `test_rasterize_type3_glyph_with_malformed_stream_returns_none`, `test_execute_content_stream_with_invalid_tokens_does_not_crash`
- **Behavior**: Returns `None` for unrenderable glyphs, skips invalid operators

### 3. Clear error messages for debugging
- **Status**: ✅ PASS (ENHANCED)
- **Evidence**: All error tests verify specific error message content:
  - "DocumentContext not provided - cannot dereference char_proc_ref 10 0"
  - "XrefResolver not provided in DocumentContext - cannot resolve char_proc_ref 10 0"
  - "PdfSource not provided in DocumentContext - cannot resolve stream for char_proc_ref 10 0"
  - "Failed to resolve Type3 char_proc_ref 10 0 R: <specific error>"
- **Context**: Messages now include the specific object reference being dereferenced
- **Improvement**: Enhanced error messages (lines 728-742) now include char_proc_ref in all error paths

### 4. Graceful degradation (report error, don't crash)
- **Status**: ✅ PASS
- **Evidence**: All 5 new tests verify no panics on error conditions:
  - Missing glyph → `None`
  - Failed resolution → `None`
  - Malformed stream → `None` or valid bitmap
  - Invalid tokens → skipped, continues execution
  - Empty stream → default white bitmap

### 5. Full test suite compiles and passes
- **Status**: ✅ PASS
- **Evidence**: All 23 type3_rasterizer tests pass:
  ```
  test result: ok. 23 passed; 0 failed; 0 ignored; 0 measured; 2892 filtered out
  ```

## Error Handling Strategy

The implementation follows the "fail gracefully" principle:
- **Missing glyphs**: Return `None` (caller can skip glyph or use fallback)
- **Invalid references**: Return `None` (treat as unrenderable, don't crash page)
- **Malformed streams**: Return `None` or partial result (don't propagate corruption)
- **Invalid content**: Skip unknown operators (spec-compliant behavior)
- **Empty streams**: Return default state (white bitmap)

## Integration with Previous Work

This bead builds on:
- **bf-69dimd**: `extract_content_stream_bytes` already handles malformed objects with `Diagnostic` errors
- **bf-3mjbv9**: `deref_char_proc_ref` already returns `Result<PdfObject, ResolveError>`
- **bf-3kzrqn**: Document context already provides resolver and source

The error handling chain is complete:
1. `char_proc()` → `Option<ObjRef>` (missing glyph)
2. `deref_char_proc_ref()` → `Result<PdfObject, ResolveError>` (invalid reference)
3. `extract_content_stream_bytes()` → `Result<Vec<u8>, Diagnostic>` (malformed stream)
4. `rasterize_type3_glyph()` → `Option<[u8; 1024]>` (all failures → `None`)

## Parent Bead

This is the FINAL child bead of **bf-36ek2x** (Resolve char_proc_ref to Type3 content stream bytes). With this complete, the parent bead can be closed.

## Commits
- Commit: feat(bf-1mma9k): enhance error messages with object reference context
- Files: crates/pdftract-core/src/font/type3_rasterizer.rs
- Changes: Enhanced error messages in `deref_char_proc_ref` to include specific char_proc_ref
- Tests: All 23 existing tests pass
- Build: cargo check successful
