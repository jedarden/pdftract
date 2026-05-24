# Code Block Detection (pdftract-8n270)

## Summary

Implemented code block classification (Phase 4.4) for detecting indented monospace code blocks.

## Implementation

Created new module `crates/pdftract-core/src/layout/code.rs` with:

1. **`is_monospace_font_name(font_name: &str) -> bool`**
   - Checks if font name (with subset prefix stripped) contains monospace indicators
   - Indicators: "mono", "courier", "code", "fixed", "console" (case-insensitive)

2. **`is_fixed_pitch_flag(flags: Option<u32>) -> bool`**
   - Checks if FixedPitch flag (bit 0) is set in FontDescriptor flags
   - Per PDF spec, bit 0 indicates fixed-pitch (monospace) fonts

3. **`is_monospace_span(font_name: &str, flags: Option<u32>) -> bool`**
   - Combines both checks: monospace if name OR FixedPitch flag indicates it

4. **`classify_code<S>(block, column_baseline_x0, font_size) -> bool`**
   - Classifies block as code if:
     - ALL spans use monospace font
     - Block is indented ≥ 2em from column baseline (2 × font_size)

5. **`compute_column_baseline<S>(blocks) -> f32`**
   - Computes median x0 of non-code paragraph blocks in column
   - Represents typical left edge of body text for indentation comparison

6. **`classify_page_code_blocks<S>(blocks)`**
   - Post-processing pass that upgrades paragraph blocks to "code" kind
   - Uses column baseline and monospace detection

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| All-Courier, indented 24pt, font_size 12pt (2em=24) | ✅ PASS | `classify_code` returns true |
| All-monospace, not indented | ✅ PASS | `classify_code` returns false |
| Mixed serif+monospace | ✅ PASS | `classify_code` returns false |
| One serif span at end | ✅ PASS | `classify_code` returns false |
| FixedPitch flag set, no "Mono" in name | ✅ PASS | Still classified as code |

## Files Modified

- `crates/pdftract-core/src/layout/code.rs` (new)
- `crates/pdftract-core/src/layout/mod.rs` (exported code module)

## Testing

All unit tests pass (107 passed, 0 failed):
```bash
cargo test --package pdftract-core --lib code
```

Test coverage includes:
- Font name matching (Courier, Mono, Code, Fixed, Console)
- FixedPitch flag detection
- Monospace span detection
- Code block classification
- Column baseline computation
- Page-level code block upgrade

## Design Notes

1. **MonospaceSpan trait**: Allows code detection to work with different span representations
2. **Font subset prefixes**: Correctly strips "ABCDEF+" prefixes before checking font names
3. **2em threshold**: As specified in plan, uses 2 × font_size for indentation requirement
4. **Post-processing approach**: Code detection runs after block formation (Phase 4.4)
5. **Median baseline**: Uses median (not mean) for robustness against outliers

## Integration

The code module is now exported from `layout::mod` and ready for integration into the extraction pipeline. The post-processing pass `classify_page_code_blocks` can be called after `group_lines_into_blocks` to upgrade paragraph blocks to code blocks.

## TODO

Per plan line 1726: "Indent threshold may miss flush-left code; add TODO."
- Flush-left code blocks (no indentation) are currently NOT detected as code
- This is intentional per the acceptance criteria ("not indented: NOT Code")
- Future enhancement could detect flush-left code via additional heuristics

## References

- Plan section: Phase 4.4 (line 1708)
- Bead: pdftract-8n270
- ISO 32000-1 Table 123 (FontDescriptor flags, bit 0 = FixedPitch)
