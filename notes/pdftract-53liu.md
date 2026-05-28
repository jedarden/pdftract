# pdftract-53liu: Phase 4.2 Line Formation (coordinator)

## Summary

Coordinator bead for Phase 4.2 Line Formation. All 4 children beads completed successfully:
- pdftract-sdx9z: Line struct + baseline computation
- pdftract-6bwq4: Baseline clustering algorithm (0.5 * median_font_size)
- pdftract-1jkme: Within-line span sorting (LTR/RTL)
- pdftract-1ofnz: RTL direction detection (unicode-bidi)

## Acceptance Criteria Status

| Criterion | Status | Evidence |
|-----------|--------|----------|
| All 4 children closed | PASS | All 4 children verified closed |
| Two-column layout: columns NOT merged into one line | PASS | test_two_column_separate_blocks (Phase 4.4) |
| Superscript span at higher y: clustered with baseline text | PASS | test_cluster_spans_superscript_stays_on_same_line |
| Arabic text: bidi R characters detected, spans sorted right-to-left | PASS | test_detect_line_direction_arabic_text |
| Mixed Latin+Arabic line: detected as "mixed" direction | PASS | test_detect_line_direction_mixed_latin_arabic |

## Implementation Summary

### Line struct (`layout/line.rs`)
- `Line<S>` generic struct with spans, bbox, baseline, direction, page_relative_y
- `LineDirection` enum (Ltr, Rtl, Mixed) with serde support
- `compute_baseline(bbox) = y0 + (bbox_height * 0.2)` per plan formula

### Baseline clustering
- `cluster_spans_into_lines(spans, median_font_size)` groups spans by baseline proximity
- Threshold: `0.5 * median_font_size` (not hardcoded)
- Handles superscripts correctly (small font, slightly higher baseline stays with main line)
- Sorts spans by x0 within each line (LTR default)

### RTL detection
- `detect_line_direction(text)` using `unicode-bidi` crate
- Counts L vs R/AL bidi classes
- Returns Ltr if ltr > rtl OR both zero (empty/neutral)
- Returns Rtl if rtl > ltr
- Returns Mixed if tied (both > 0)

### Within-line sorting
- `sort_spans_in_line(line)` handles LTR (x0 asc), RTL (x1 desc), Mixed (fallback to x0 asc)
- Stable sort preserves insertion order on ties
- NaN bbox handled as Ordering::Equal

## Test Results

```
Summary [   0.040s] 44 tests run: 44 passed, 2409 skipped
```

All line module tests pass including:
- 11 baseline computation tests
- 11 clustering algorithm tests
- 12 RTL direction detection tests
- 7 span sorting tests
- 3 block formation tests (including two-column)

## References

- Plan: Phase 4.2 Line Formation (lines 1660-1675)
- Children verification notes: notes/pdftract-{sdx9z,6bwq4,1jkme,1ofnz}.md
