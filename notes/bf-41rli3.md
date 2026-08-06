# Bead bf-41rli3: Cubic Bezier Curve Subdivision Implementation

## Summary
Implemented cubic Bezier curve subdivision to convert curved paths into line segments for rasterization.

## Implementation

### Files Modified
- `crates/pdftract-core/src/font/type3_rasterizer.rs`

### Functions Added

#### 1. `cubic_bezier_point(p0, p1, p2, p3, t) -> Point`
- Evaluates a point on a cubic Bezier curve at parameter t using de Casteljau's algorithm
- Uses the Bezier formula: B(t) = (1-t)³P₀ + 3(1-t)²tP₁ + 3(1-t)t²P₂ + t³P₃
- Input: Four control points and parameter t ∈ [0, 1]
- Output: Point on the curve at parameter t

#### 2. `subdivide_cubic_bezier(p0, p1, p2, p3, t) -> ((Point, Point, Point, Point), (Point, Point, Point, Point))`
- Splits a cubic Bezier curve into two curves at parameter t using de Casteljau's algorithm
- Computes intermediate points and returns left and right curve segments
- Used for recursive subdivision

#### 3. `curve_flatness(p0, p1, p2, p3) -> f64`
- Calculates how close the curve is to a straight line
- Uses midpoint deviation method:
  - Computes curve midpoint at t=0.5
  - Computes chord midpoint (line from p0 to p3)
  - Returns distance between these midpoints
- Lower values indicate flatter curves

#### 4. `flatten_cubic_bezier_recursive(segments, p0, p1, p2, p3, depth)`
- Recursively subdivides Bezier curves until flat enough or max depth reached
- Constants:
  - `FLATNESS_THRESHOLD = 0.5` pixels
  - `MAX_DEPTH = 8` levels
- Appends line segments to output vector

#### 5. `flatten_cubic_bezier(p0, p1, p2, p3) -> Vec<(Point, Point)>`
- Public interface for curve flattening
- Returns vector of line segments approximating the Bezier curve

### Integration with Path Commands

Updated `rasterize_path()` to handle three curve command types:

1. **`CubicTo(cp1, cp2, end)`** - Full cubic Bezier with explicit control points
2. **`ShorthandCubicTo(cp2, end)`** - First control point implied (reflection symmetry)
3. **`ShorthandCubicToY(cp1, end)`** - Second control point implied (reflection symmetry)

All three now use `flatten_cubic_bezier()` to convert curves to line segments before rasterization.

## Acceptance Criteria Verification

✅ **PASS** - Function to evaluate Bezier point at parameter t (`cubic_bezier_point`)
✅ **PASS** - Function to subdivide curve adaptively based on flatness (`flatten_cubic_bezier_recursive`)
✅ **PASS** - Recursive subdivision with max depth limit (MAX_DEPTH = 8)
✅ **PASS** - Converts curves to line segment list (returns `Vec<(Point, Point)>`)
✅ **PASS** - Code compiles (verified with `cargo check --package pdftract-core`)

## Technical Notes

- Used de Casteljau's algorithm for numerical stability
- Midpoint deviation flatness check is efficient and accurate enough for 32x32 rasterization
- Subdivision at t=0.5 provides good balance between segments and accuracy
- MAX_DEPTH=8 prevents infinite recursion while still allowing fine detail
- FLATNESS_THRESHOLD=0.5 pixels is appropriate for 32x32 glyph bitmaps

## Testing

The implementation was verified by:
1. Compiling the code successfully with `cargo check`
2. Following the existing code patterns in type3_rasterizer.rs
3. Using standard Bezier curve algorithms (de Casteljau's)

Runtime testing would require:
- Type 3 fonts with Bezier curves in their glyph definitions
- Visual inspection of generated bitmaps
- Comparison with reference renderers
