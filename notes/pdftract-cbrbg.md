# Verification Note: pdftract-cbrbg

## Summary
Implemented span flag detector for Phase 4.1 as specified in the plan.

## Implementation Details

### Files Modified
- `crates/pdftract-core/src/lib.rs` - Added span_flags module
- `crates/pdftract-core/src/span_flags.rs` - New module (467 lines)

### Functionality
Implemented `detect_span_flags()` function that returns a u8 bitmask combining 5 style flag bits:
- `flags::BOLD` (bit 0)
- `flags::ITALIC` (bit 1)
- `flags::SMALLCAPS` (bit 2)
- `flags::SUBSCRIPT` (bit 3)
- `flags::SUPERSCRIPT` (bit 4)

### Detection Logic (per plan lines 1667-1671)

**BOLD detection** (set if ANY of):
- Font name contains "Bold", "Bd", "Black", "Heavy", "ExtraBold", etc.
- FontDescriptor /Flags bit 18 (ForceBold) set
- FontDescriptor /StemV > 120

**ITALIC detection** (set if ANY of):
- Font name contains "Italic" or "Oblique"
- FontDescriptor /ItalicAngle != 0

**SMALLCAPS detection** (set if ANY of):
- Font name contains "SC", "SmallCaps", or ".sc"
- FontDescriptor /Flags bit 3 (Symbolic/SmallCap) set

**SUBSCRIPT detection**:
- `text_rise < -0.1 * font_size`

**SUPERSCRIPT detection**:
- `text_rise > 0.1 * font_size`

### API
```rust
use pdftract_core::span_flags::{detect_span_flags, FontInfo, flags};

let font = FontInfo::new()
    .with_name("Times-Bold".to_string())
    .with_flags(0x40000) // ForceBold bit
    .with_stem_v(150.0);

let span_flags = detect_span_flags(&font, -2.0, 12.0);
assert!(span_flags & flags::BOLD != 0);
assert!(span_flags & flags::SUBSCRIPT != 0);
```

## Acceptance Criteria Status

### PASS
- [x] "Times-Bold" font → BOLD set
- [x] "Helvetica-Italic" font → ITALIC set
- [x] "Times-BoldItalic" font → BOLD | ITALIC set
- [x] text_rise -2pt with font_size 12pt → SUBSCRIPT set (rise/size = -0.167 < -0.1)
- [x] text_rise +1.5pt with font_size 12pt → SUPERSCRIPT set
- [x] text_rise -0.5pt with font_size 12pt → NEITHER (rise/size = -0.042, within threshold)
- [x] font.descriptor.flags bit 18 set → BOLD set
- [x] /StemV 150 → BOLD set

### Additional Test Coverage
- [x] Subset prefix handling (ABCDEF+Times-Bold)
- [x] Small caps with ".sc" suffix pattern
- [x] ItalicAngle detection (non-zero angle)
- [x] All flags combination (BoldItalic + SmallCaps + Superscript)
- [x] Zero font_size edge case (no division by zero)
- [x] Mutually exclusive SUB/SUPER (can't have both)

## Test Results
```
cargo test -p pdftract-core span_flags --lib
test result: ok. 27 passed; 0 failed; 0 ignored; 0 measured
```

## Quality Gates
- [x] `cargo check --all-targets` - PASSED
- [x] `cargo clippy --all-targets -- -D warnings` - PASSED
- [x] `cargo fmt` - APPLIED
- [x] All tests pass

## Git Commit
- Commit: `cad7d2c`
- Message: `feat(pdftract-cbrbg): implement span flag detector for Phase 4.1`
- Pushed to: `github/main`

## References
- Plan: Phase 4.1 Flag detection (lines 1648-1653, 1667-1671)
- Bead: pdftract-cbrbg
