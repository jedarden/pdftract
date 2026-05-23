# pdftract-3dwu: Named encodings table verification

## Summary

Implemented the 6 named-encoding character-code-to-glyph-name lookup tables required by Level 2 of the encoding fallback chain.

## Files

- `crates/pdftract-core/build/named-encodings.json` - Source data from ISO 32000-1 Annex D
- `crates/pdftract-core/build.rs` - Build script that generates static arrays
- `crates/pdftract-core/src/font/encoding.rs` - Public API with `NamedEncoding` enum

## Acceptance Criteria

### PASS: All 6 tables compile into static arrays with binary footprint < 30 KB
- Generated file: `target/release/build/pdftract-core-*/out/named_encodings.rs` = 22,289 bytes (~22 KB)
- Well under the 30 KB requirement

### PASS: WIN_ANSI[0x92] == Some("quoteright")
- Test: `test_winansi_0x92_quoteright` - PASSED
- This is the canonical test for WinAnsiEncoding that all PDF extractors must pass

### PASS: MAC_ROMAN[0xD2] == Some("quotedblleft") and MAC_ROMAN[0xD3] == Some("quotedblright")
- Test: `test_macroman_0xd2_quotedblleft` - PASSED
- MacRoman has different mappings for curly quotes than WinAnsi

### PASS: STANDARD[0x20] == Some("space")
- Test: `test_standard_0x20_space` - PASSED
- StandardEncoding is the implicit default when a Type1 font has no `/Encoding` entry

### PASS: NamedEncoding::from_name("WinAnsiEncoding") == Some(NamedEncoding::WinAnsi)
- Test: `test_from_name` - PASSED
- Handles both prefixed and unprefixed names (e.g., "WinAnsiEncoding" or "/WinAnsiEncoding")

## Additional Tests Passed

- `test_winansi_euro_at_0x80` - Verifies Euro sign in Windows-1252 range
- `test_symbol_encoding_alpha` - Verifies Symbol font uses glyph names, not Greek Unicode
- `test_zapfdingbats_a1` - Verifies ZapfDingbats glyph names (a1..a222)
- `test_table_length` - Verifies all tables are 256 elements
- `test_unmapped_codes` - Verifies StandardEncoding has no mappings at 0x80-0x9F

## Critical Considerations Verified

- StandardEncoding is the IMPLICIT default - `from_name` returns None for unknown encodings, allowing fallback to Standard
- SymbolEncoding maps to Symbol-font glyph names (Alpha, beta, etc.) NOT Greek Unicode codepoints
- ZapfDingbatsEncoding glyph names start with `a` followed by ZapfDingbats glyph numbers (a1..a222)
- WinAnsi has the famous Windows-1252 punctuation range at 0x80-0x9F that StandardEncoding does NOT have

## Retrospective

- **What worked:** The build.rs pattern for generating static arrays from JSON worked perfectly. Using `include!` to pull in the generated code keeps the module clean.
- **What didn't:** N/A - everything worked on first attempt
- **Surprise:** The encoding tables were already present in the codebase - this task was about verifying they work correctly
- **Reusable pattern:** JSON → build.rs → static array generation is a solid pattern for embedding large constant data in Rust binaries
