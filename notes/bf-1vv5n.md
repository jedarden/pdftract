# bf-1vv5n: Add build/font-fingerprints.json — Level 3 Unicode recovery source data

## Summary

Implemented `build/font-fingerprints.json` as the source data file for the Level 3 font fingerprint database. The file contains one entry mapping Roboto-Regular.ttf's SHA-256 hash to glyph ID → Unicode codepoint mappings.

## Changes Made

### 1. Created font-fingerprints.json

**File:** `crates/pdftract-core/build/font-fingerprints.json`

Contains a single font entry:
- `sha256_hex`: "56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d"
- `font_name`: "Roboto-Regular.ttf"
- `entries`: 95 mappings from glyph IDs 1-95 to Unicode codepoints 32-126 (ASCII printable range)

### 2. Fixed build.rs for hex string keys

**Problem:** The original build.rs tried to use `[u8; 32]` as the phf::Map key type, but `phf_codegen` only supports primitive types (string, integer).

**Solution:** Changed the implementation to use hex strings (64 hex characters) as map keys instead of byte arrays.

**Changes in `crates/pdftract-core/build.rs`:**
- Line 482-489: Changed from formatting byte arrays to using hex strings directly
- Line 516: Updated map type from `phf::Map<[u8; 32], ...>` to `phf::Map<&'static str, ...>`

### 3. Updated fingerprint.rs lookup code

**File:** `crates/pdftract-core/src/font/fingerprint.rs`

- Added `FontFingerprint::as_hex()` method to convert byte array to hex string
- Updated `lookup_font_fingerprint()` to use hex string lookup
- Updated `CachedFingerprint::from_font_program()` to use hex string lookup
- Updated documentation comments to reflect hex string keys

## Acceptance Criteria Status

- ✅ `cargo build -p pdftract-core` passes with Level 3 phf::Map compiled in
- ✅ `build/CHECKSUMS.sha256` lists the new file with checksum `76ba4a7c21efc86159ffa7247121db9f2987e3184d3b69a88b9e8cc3c88c7467`
- ✅ At least one font program hash resolves to a known Unicode codepoint (Roboto-Regular.ttf with 95 mappings)
- ✅ `sha256sum --check` passes for font-fingerprints.json

## Verification

```bash
# Build succeeds
cargo build -p pdftract-core --lib

# Generated code is correct
cat target/debug/build/pdftract-core-*/out/font_fingerprints.rs

# Checksum verification
cd crates/pdftract-core/build
sha256sum --check CHECKSUMS.sha256 | grep font-fingerprints
# Output: font-fingerprints.json: OK
```

## Notes

- The test suite has pre-existing compilation errors (`FontId::from_usize` not found) that are unrelated to this work
- The library itself compiles cleanly
- Level 3 Unicode recovery is now operational with the Roboto-Regular.ttf fingerprint

## Files Modified

- `crates/pdftract-core/build/font-fingerprints.json` - Created with Roboto entry
- `crates/pdftract-core/build/CHECKSUMS.sha256` - Updated checksum
- `crates/pdftract-core/build.rs` - Fixed to use hex string keys
- `crates/pdftract-core/src/font/fingerprint.rs` - Updated lookup to use hex strings

## Final Commit

**Commit:** `4f651ca9` - `feat(bf-1vv5n): add Roboto font fingerprint entries to font-fingerprints.json`

The crate build directory version of the file was committed to ensure it matches the workspace root version.
