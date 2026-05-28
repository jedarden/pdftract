# pdftract-3g6ne: CMap Codespace Range Parser

## Bead Summary

Implemented the CMap codespace range parser for extracting byte-width boundaries from `begincodespacerange` / `endcodespacerange` PostScript blocks.

## Implementation Location

- Module: `crates/pdftract-core/src/font/codespace.rs`
- Exported from: `crates/pdftract-core/src/font/mod.rs`

## Structures Implemented

### CodespaceRange
```rust
pub struct CodespaceRange {
    pub lo: [u8; 4],   // Low bound (big-endian, 4-byte storage)
    pub hi: [u8; 4],   // High bound (big-endian, 4-byte storage)
    pub width: u8,     // Byte width (1-4)
}
```

### CodespaceRanges
```rust
pub struct CodespaceRanges {
    pub ranges: SmallVec<[CodespaceRange; 8]>,
}
```

### CodespaceParser
PostScript-style tokenizer that:
- Recognizes `begincodespacerange` / `endcodespacerange` keywords
- Parses hex string pairs `<lo> <hi>`
- Validates width matching (lo.len() == hi.len())
- Emits diagnostics on malformed entries
- Continues parsing after errors (recovery)

## Acceptance Criteria Status

| Criterion | Status | Test |
|-----------|--------|------|
| Parse <00> <7F> → 1 range, width=1 | PASS | `test_parse_single_range_one_byte` |
| Parse <00> <7F> <8000> <FFFF> in one block → 2 ranges | PASS | `test_parse_two_ranges_mixed_width` |
| Width inference: 2-char hex → width=1; 4-char hex → width=2 | PASS | `test_width_inference` |
| Case-insensitive hex (<C0> and <c0> equivalent) | PASS | `test_case_insensitive_hex` |
| Malformed range (width mismatch) → diagnostic + skipped | PASS | `test_malformed_range_width_mismatch` |
| Empty CMap → empty ranges | PASS | `test_empty_cmap`, `test_no_codespace_block` |
| Round-trip with Identity-H CMap fixture | N/A | No standalone CMap fixtures exist; tests cover parsing logic |

### Additional Tests

- `test_jis_range`: JIS lead/trail 2-byte pattern `<8140> <FEFE>`
- `test_three_byte_range`: 3-byte codespace support
- `test_four_byte_range`: 4-byte codespace support
- `test_invalid_width_too_large`: Rejects 5+ byte ranges
- `test_find_range`: Utility to match byte sequences to ranges
- `test_comment_in_block`: PostScript comment stripping
- `test_hex_string_with_whitespace`: Internal whitespace handling
- `test_odd_length_hex_string`: Dangling nibble padding
- `test_recovery_on_error`: Continues after malformed entries
- `test_convenience_function`: Public API entry points

## Public API

```rust
// Parse without diagnostics (for internal use)
pub fn parse_codespace_ranges(input: &[u8]) -> CodespaceRanges

// Parse with diagnostics (for error reporting)
pub fn parse_codespace_ranges_with_diags(input: &[u8]) -> (CodespaceRanges, Vec<Diagnostic>)
```

## Design Decisions

1. **4-byte storage for bounds**: Ranges up to 4 bytes are stored in fixed `[u8; 4]` arrays with leading zeros, simplifying comparison logic
2. **SmallVec capacity 8**: Most predefined CMaps (Identity-H/V, UTF-16 variants) have 1-2 ranges; 8 provides stack allocation for typical cases without overflow
3. **Recovery over hard failure**: Malformed entries emit diagnostics but don't stop parsing; subsequent valid ranges are still collected
4. **Case-insensitive hex**: Both `<C0>` and `<c0>` parse to `0xC0` per PDF spec
5. **Width validation**: Rejects ranges where lo.len() != hi.len() or width > 4

## Integration Notes

- Module is imported in `font/mod.rs` but not yet exported at crate level
- Sibling tokenizer bead will consume `CodespaceRanges` for multi-byte walking
- Coordinator `pdftract-19oy` (CMap parser + tokenizer) depends on this module

## Commit

- Hash: `1dfaf73`
- Message: `feat(pdftract-3g6ne): implement CMap codespace range parser`
- Pushed: `forgejo main`
