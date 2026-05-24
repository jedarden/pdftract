# pdftract-sdx9z: Line struct + baseline computation per span

## Summary

Implemented Phase 4.2 line formation infrastructure:
- Created `layout/line.rs` module with `Line` struct and `compute_baseline` function
- Added `LineDirection` enum with serde support (Ltr, Rtl, Mixed)
- Added helper functions for bbox operations

## Files Modified

- `crates/pdftract-core/src/layout/line.rs` (new): 247 lines
- `crates/pdftract-core/src/layout/mod.rs`: Exported new line module

## Acceptance Criteria Status

### PASS
- ✅ `compute_baseline([0, 100, 50, 110])` returns `102.0` (height 10, baseline = 100 + 10*0.2)
- ✅ `compute_baseline([0, 100, 50, 100])` returns `100.0` (zero-height case)
- ✅ `union_bboxes` correctly computes union of 3 span bboxes
- ✅ `LineDirection` serde roundtrips to/from "ltr"/"rtl"/"mixed"
- ✅ All 11 unit tests pass in `layout::line::tests`

## Implementation Details

### `Line` struct
```rust
pub struct Line<S> {
    pub spans: Vec<S>,
    pub bbox: [f32; 4],         // Union of span bboxes
    pub baseline: f32,          // Average of member span baselines
    pub direction: LineDirection,
    pub page_relative_y: f32,   // (page_height - bbox[3]) / page_height
}
```

### `compute_baseline` function
```rust
pub fn compute_baseline(bbox: &[f32; 4]) -> f32 {
    let height = bbox[3] - bbox[1];
    bbox[1] + height * 0.2  // 0.2 = descender approximation
}
```

### `LineDirection` enum
```rust
pub enum LineDirection {
    Ltr,   // Left-to-right
    Rtl,   // Right-to-left
    Mixed, // Bidirectional
}
```

## Plan References

- Phase 4.2 baseline (lines 1665-1666): `y0 + (bbox_height * 0.2)` formula
- RTL detection (line 1686): `unicode-bidi` crate for future bidi character category lookup

## Testing

All tests pass:
```
running 38 tests
test layout::line::tests::test_compute_baseline_normal_span ... ok
test layout::line::tests::test_compute_baseline_zero_height ... ok
test layout::line::tests::test_compute_baseline_large_height ... ok
test layout::line::tests::test_line_direction_serdes_ltr ... ok
test layout::line::tests::test_line_direction_serdes_rtl ... ok
test layout::line::tests::test_line_direction_serdes_mixed ... ok
test layout::line::tests::test_line_accessors ... ok
test layout::line::tests::test_union_bboxes_single ... ok
test layout::line::tests::test_union_bboxes_multiple ... ok
test layout::line::tests::test_union_bboxes_empty ... ok
test layout::line::tests::test_union_bboxes_nested ... ok
test layout::line::tests::test_union_bboxes_disjoint ... ok

test result: ok. 38 passed; 0 failed
```

## Next Steps

Future beads will implement:
- Actual line clustering algorithm (baseline proximity grouping)
- RTL detection using `unicode-bidi` crate
- Span-to-line aggregation with reading order sorting
