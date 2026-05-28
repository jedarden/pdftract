# pdftract-4bylb: Docstrum fallback implementation

## Summary

The Docstrum algorithm (O'Gorman 1993) for reading order detection is fully implemented in `/home/coding/pdftract/crates/pdftract-core/src/layout/reading_order.rs` (lines 603-897). This serves as the fallback when XY-cut produces fragmented regions (> 10 regions with < 3 blocks each).

## Implementation details

### Core algorithm (lines 646-708)

The `docstrum<B>()` function:

1. **k=5 nearest neighbors** (line 669): Uses Docstrum standard k-value
2. **Center computation** (lines 672-680): Euclidean center-to-center in PDF user space
3. **Adjacency graph** (line 683): `build_adjacency_graph()` with k-nearest + angle constraints
4. **Root detection** (line 686): `find_roots()` identifies nodes with no incoming edges from above
5. **Root sorting** (lines 689-697): By (column ASC, y DESC)
6. **Traversal** (line 700): `traverse_components()` DFS in y-then-x order

### Angle constraints (lines 736-782)

- **Within-line**: ±30° from horizontal (0°)
- **Between-line**: ±30° from vertical (90° or -90°)
- Computed via `atan2(dy, dx)`; angles in radians

### Root detection (lines 798-827)

A root is a block with no incoming edges from blocks whose center-y is greater (i.e., no block above it connects to it). Handles circular/pathological cases by falling back to all nodes sorted by position.

### Graph traversal (lines 829-897)

- Undirected adjacency; directional traversal
- DFS per component, neighbors sorted by (y DESC, x ASC)
- Isolated blocks visited as standalone components

## Test results

All 5 Docstrum tests PASS:

| Test | Description | Result |
|------|-------------|--------|
| `test_docstrum_empty` | Empty input | PASS |
| `test_docstrum_single_block` | Single block | PASS |
| `test_docstrum_magazine_main_and_sidebar` | Main column before sidebar | PASS |
| `test_docstrum_all_one_line_horizontal` | Left-to-right horizontal flow | PASS |
| `test_docstrum_all_one_column_vertical` | Top-to-bottom vertical flow | PASS |
| `test_docstrum_scattered_pathological` | Scattered blocks visited | PASS |
| `test_docstrum_k_nearest_neighbors` | k=5 with >k blocks | PASS |
| `test_docstrum_angle_constraint_within_line` | ±30° horizontal constraint | PASS |
| `test_docstrum_angle_constraint_between_line` | ±30° vertical constraint | PASS |

## Incidental fixes

Fixed `/home/coding/pdftract/crates/pdftract-core/Cargo.toml` line 60:

- Changed `ureq` feature from `"rustls"` to `"tls"` (ureq 2.10 compatibility)
- This unblocks the build; ureq 2.x uses "tls" not "rustls" as the feature name

## Acceptance criteria verification

- ✅ Magazine main+sidebar: 2 components; main first, sidebar second
- ✅ Pathological scattered: each a root, visited (column, y desc)
- ✅ All-one-line horizontal: 1 component, left-to-right
- ✅ All-one-column vertical: 1 component, top-to-bottom

## Files modified

- `/home/coding/pdftract/crates/pdftract-core/Cargo.toml` (line 60: ureq feature fix)
- `/home/coding/pdftract/crates/pdftract-core/src/layout/reading_order.rs` (docstrum already implemented)

## References

- Plan lines 1728-1730 (Phase 4.5 Reading Order - Docstrum fallback)
- O'Gorman 1993: "The document spectrum for page layout analysis"
