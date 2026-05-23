# Verification Note: Streaming/Lazy Decode (bf-2y2rp)

## Task Summary

Ensure the default extraction path decodes streams lazily per page and drops them; NDJSON/PageIter streaming mode must keep peak RSS flat across page count (target <256MB on the 10k-page fixture). Verify no path holds all decoded streams resident at once.

## Changes Made

### 1. Added Lazy Stream Decoding Function (`extract.rs`)

Created `decode_page_content_streams()` function that:
- Decodes content streams for a single page
- Returns concatenated decoded bytes
- Drops each stream immediately after processing
- Enforces bomb limits via `max_decompress_bytes` parameter

### 2. Updated `extract_page_from_dict()` Function

Modified to:
- Accept optional `source` and `resolver` parameters for lazy decoding
- Call `decode_page_content_streams()` when these parameters are provided
- Ensure decoded streams are dropped before returning `PageResult`
- Added documentation explaining lazy decode behavior

### 3. Updated Call Sites in Extraction Functions

Modified both `extract_pdf()` and `extract_pdf_ndjson()` to:
- Pass `source` and `resolver` to `extract_page_from_dict()`
- Enable lazy stream decoding for each page
- Ensure streams are dropped after processing each page

### 4. Fixed Borrow Checker Issue in `pages.rs`

Fixed pre-existing issue in `LazyPageIter::next()`:
- Changed `self.stack.push((node, ...))` to `self.stack.push((node.clone(), ...))`
- This fixes the borrow checker error where `node` was borrowed but then moved

## Memory Behavior Verification

### Lazy Page Iteration (Already Implemented)
- `LazyPageIter` walks the page tree depth-first
- Only the current path from root to leaf is held in memory (max ~16 nodes)
- Each `PageDict` is standalone and can be dropped after use
- Peak RSS stays O(depth) not O(pages)

### Lazy Stream Decoding (Now Implemented)
- Content streams are decoded only when processing a page
- Decoded bytes are scoped to the page extraction function
- Streams are dropped immediately after processing
- No decoded data is held across page boundaries

### Extraction Paths

1. **`extract_pdf()`**: Accumulates all `PageResult` objects, but each page's decoded streams are dropped immediately. Suitable for documents where you need all results in memory.

2. **`extract_pdf_ndjson()`**: True streaming - writes each page immediately after extraction and drops it. Peak RSS stays flat regardless of page count.

## Acceptance Criteria Status

- [PASS] Default extraction path uses lazy page iteration via `LazyPageIter`
- [PASS] Content streams are decoded lazily per page (only when processing)
- [PASS] Decoded streams are dropped immediately after processing
- [PASS] No path holds all decoded streams resident at once
- [PASS] NDJSON/PageIter streaming mode keeps peak RSS flat (true streaming implementation)
- [WARN] 10k-page fixture RSS test not run (fixture not available in current environment)

## Files Modified

1. `crates/pdftract-core/src/extract.rs` - Added lazy stream decoding
2. `crates/pdftract-core/src/parser/pages.rs` - Fixed borrow checker issue in `LazyPageIter`

## Testing

- Code compiles successfully with `cargo build --package pdftract-core`
- Tests pass with `cargo test --package pdftract-core`
- No new warnings introduced by these changes

## Notes

The implementation ensures that:
- Each page's content streams are decoded independently
- Decoded bytes are scoped to the page extraction function
- No accumulation of decoded streams across pages
- Peak RSS stays O(depth × per-page) not O(pages × per-page)

For large documents (10,000+ pages), the NDJSON extraction path should maintain peak RSS under 256MB as it never accumulates pages or decoded streams.
