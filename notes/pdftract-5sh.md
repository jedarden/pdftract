# pdftract-5sh: CIDToGIDMap resolver (Identity and stream forms)

## Summary

Implemented the CIDToGIDMap resolver for CIDFontType2 descendant fonts with:
- `/Identity` name detection (zero-allocation short-circuit)
- Stream form parsing into `Box<[u16]>` array (2-byte big-endian GID values)
- `CIDTOGIDMAP_TRUNCATED` diagnostic for odd-byte-count input
- Out-of-range CID returns GID 0 (notdef glyph)

## Changes Made

### 1. Added new diagnostic code (`diagnostics.rs`)

- `DiagCode::FontCidtogidmapTruncated` - emitted when CIDToGIDMap stream has odd byte count
- Added to category, name, severity (Warning), and catalog entries

### 2. Updated `CIDToGIDMap` enum (`type0.rs`)

Changed from `Custom(Vec<u8>)` to `Array(Box<[u16]>)`:
- Pre-parsed u16 array instead of raw bytes
- Single heap allocation, not per-lookup
- `get()` method now uses `arr.get(cid as usize).copied().or(Some(0))`

### 3. Updated `load_cid_to_gid_map()` function

- Now parses decoded bytes into `Box<[u16]>` array
- Emits `CIDTOGIDMAP_TRUNCATED` diagnostic on odd-length input
- Truncates trailing byte instead of failing
- Takes `diagnostics: &mut Vec<Diagnostic>` parameter

### 4. Updated tests

- `test_cid_to_gid_map_array` - tests Array variant with [0, 1, 2, 3]
- `test_cid_to_gid_map_array_big_endian` - tests big-endian parsing
- `test_cid_to_gid_map_out_of_range` - tests GID 0 return for out-of-range CID
- `test_cid_to_gid_map_from_stream` - tests stream loading with [0, 5, 10] per acceptance criteria
- `test_cid_to_gid_map_truncated` - tests odd-byte-count diagnostic emission

## Acceptance Criteria - PASS

- [PASS] Identity form: lookup of any CID returns same value as u16
- [PASS] Stream form: synthetic 3-CID array decodes correctly [0, 5, 10]
- [PASS] Out-of-range CID returns GID 0 with no panic
- [PASS] Diagnostic `CIDTOGIDMAP_TRUNCATED` emitted on odd-byte-count input

## Test Results

```
test font::type0::tests::test_cid_to_gid_map_array ... ok
test font::type0::tests::test_cid_to_gid_map_array_big_endian ... ok
test font::type0::tests::test_cid_to_gid_map_identity ... ok
test font::type0::tests::test_cid_to_gid_map_out_of_range ... ok
test font::type0::tests::test_cid_to_gid_map_truncated ... ok
test font::type0::tests::test_cid_to_gid_map_from_stream ... ok
test result: ok. 6 passed; 0 failed; 0 ignored
```

All 25 type0 tests pass.

## Files Modified

- `crates/pdftract-core/src/diagnostics.rs` - added FontCidtogidmapTruncated diagnostic
- `crates/pdftract-core/src/font/type0.rs` - updated CIDToGIDMap enum and implementation
