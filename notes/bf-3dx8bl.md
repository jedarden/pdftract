# Scanline Fill Algorithm Analysis

**Bead:** bf-3dx8bl  
**Reference Code:** `crates/pdftract-core/src/font/type3_rasterizer.rs` lines 994-1052  
**Date:** 2026-08-06

## Overview

The `fill_polygon` function implements a classic scanline polygon fill algorithm optimized for Type3 glyph rasterization. It operates on a list of edge segments in bitmap coordinates and produces filled pixels on a bitmap.

## Function Signature

```rust
fn fill_polygon(&mut self, edges: &[(i32, i32, i32, i32)])
```

**Parameters:**
- `edges`: Slice of tuples `(x0, y0, x1, y1)` representing line segments in bitmap coordinates (pixel space)

**Context:**
- Called from `rasterize_path()` (line 952) when stroke=false (fill mode)
- `self.bitmap` is the target bitmap (dynamic sizing based on font bbox)
- Pixels are set to 0 (black) to indicate filled areas

## Algorithm Flow

### Phase 1: Edge Table Setup (Lines 929-943 in rasterize_path)

Before `fill_polygon` is called, edges are prepared from path commands:

```rust
// Collect path segments from commands
let mut segments = Vec::new();
for cmd in &self.path.commands {
    match cmd {
        PathCommand::MoveTo(p) => { /* update current point */ }
        PathCommand::LineTo(p) => segments.push((start, *p)),
        PathCommand::Rect(x, y, w, h) => {
            // Convert rectangle to 4 line segments
            segments.push((p0, p1));
            segments.push((p1, p2));
            segments.push((p2, p3));
            segments.push((p3, p0));
        }
        PathCommand::CubicTo(cp1, cp2, end) => {
            let curve_segments = Self::flatten_cubic_bezier(start, *cp1, *cp2, *end);
            segments.extend(curve_segments);
        }
        // ...
    }
}

// Transform and convert to bitmap coordinates
let mut edges = Vec::new();
for (p0, p1) in segments {
    let (x0, y0) = self.gstate.ctm.transform_point(p0.x, p0.y);
    let (x1, y1) = self.gstate.ctm.transform_point(p1.x, p1.y);
    
    // Round to nearest pixel
    edges.push((x0.round() as i32, y0.round() as i32, 
                x1.round() as i32, y1.round() as i32));
}
```

**Key Transformations:**
1. Path commands → line segments (curves flattened)
2. User space → bitmap space (via CTM)
3. Float coordinates → integer pixel coordinates

### Phase 2: Y-Bounds Calculation (Lines 998-1009)

```rust
let mut min_y = height;
let mut max_y = 0i32;

for &(_, y0, _, y1) in edges {
    min_y = min_y.min(y0.min(y1));
    max_y = max_y.max(y0.max(y1));
}

// Clamp to bitmap bounds
min_y = min_y.max(0);
max_y = max_y.min(height - 1);
```

**Purpose:** Find the vertical range of the polygon to limit scanline processing.

**Key Points:**
- Iterates through all edge endpoints to find y-extent
- Clamps to actual bitmap dimensions to avoid out-of-bounds access
- Optimization: skips scanlines that cannot possibly intersect the polygon

### Phase 3: Scanline Loop (Line 1012)

```rust
for y in min_y..=max_y {
    let mut intersections = Vec::new();
    // ... process this scanline
}
```

**Purpose:** Process each horizontal scanline within the polygon's y-bounds.

**Data Structure:** `intersections` - Dynamic vector of x-coordinates where edges cross this scanline.

### Phase 4: Edge Intersection Testing (Lines 1015-1033)

```rust
for &(x0, y0, x1, y1) in edges {
    // Skip horizontal edges
    if y0 == y1 {
        continue;
    }

    // Half-open interval test
    let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
    if y_min <= y && y < y_max {
        // Calculate x intersection
        let dy = y1 - y0;
        let t = (y - y0) as f64 / dy as f64;
        let x = x0 as f64 + t * (x1 - x0) as f64;
        intersections.push(x);
    }
}
```

**Key Edge Cases Handled:**

1. **Horizontal Edge Skip:** Edges where `y0 == y1` are skipped entirely because they don't contribute to scanline crossings (parallel to scanline).

2. **Half-Open Interval:** Uses `[y_min, y_max)` interval semantics:
   - Include lower endpoint: `y_min <= y`
   - Exclude upper endpoint: `y < y_max`
   - **Prevents double-counting vertices** where two edges meet

3. **Floating-Point Intersection Calculation:**
   - Uses `f64` precision for intersection calculation
   - Parameter `t = (y - y0) / dy` represents fractional position along edge
   - `x = x0 + t * (x1 - x0)` is linear interpolation formula
   - Storing as `f64` preserves subpixel precision for later rounding

### Phase 5: Intersection Sorting (Lines 1035-1036)

```rust
intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
```

**Purpose:** Sort x-coordinates left-to-right to enable pair-wise filling.

**Note:** Uses `partial_cmp()` to handle potential NaN values (though shouldn't occur with valid input).

### Phase 6: Pair-Wise Filling (Lines 1038-1050)

```rust
for i in (0..intersections.len()).step_by(2) {
    if i + 1 < intersections.len() {
        let x_start = intersections[i].ceil() as i32;
        let x_end = intersections[i + 1].floor() as i32;

        for x in x_start..=x_end {
            if x >= 0 && x < width {
                self.bitmap.set(x, y, 0);
            }
        }
    }
}
```

**Scanline Fill Rule:**
- Fill between pairs of intersections (0-1, 2-3, 4-5, ...)
- Implements the **even-odd fill rule**
- When crossing an edge, toggle in/out state

**Rounding Strategy:**
- `x_start = ceil()` - round up to include the first pixel
- `x_end = floor()` - round down to include the last pixel
- Prevents gaps at subpixel boundaries

**Bounds Check:** Guards against x-coordinates outside bitmap width.

## Key Data Structures

### Edges
```rust
[(i32, i32, i32, i32)]  // (x0, y0, x1, y1)
```
- Integer bitmap coordinates
- Transformed from path commands via CTM
- Include lines from flattened Bezier curves

### Intersections
```rust
Vec<f64>  // x-coordinates of scanline crossings
```
- Floating-point for subpixel precision
- Sorted left-to-right
- Used in pairs for even-odd rule

### Bitmap
```rust
self.bitmap: Bitmap
```
- Target rasterization surface
- Dynamic sizing (not fixed 32x32)
- Set pixel to 0 (black) for filled areas

## Curve Flattening Details

Bezier curves are converted to line segments **before** scanline filling:

```rust
fn flatten_cubic_bezier_recursive(
    segments: &mut Vec<(Point, Point)>,
    p0, p1, p2, p3: Point,
    depth: usize
) {
    const FLATNESS_THRESHOLD: f64 = 0.5;
    const MAX_DEPTH: usize = 8;

    // Check flatness or max recursion depth
    let flatness = Self::curve_flatness(p0, p1, p2, p3);
    if flatness <= FLATNESS_THRESHOLD || depth >= MAX_DEPTH {
        segments.push((p0, p3));
        return;
    }

    // Subdivide and recurse
    let (left, right) = Self::subdivide_cubic_bezier(p0, p1, p2, p3, 0.5);
    Self::flatten_cubic_bezier_recursive(segments, left.0, left.1, left.2, left.3, depth + 1);
    Self::flatten_cubic_bezier_recursive(segments, right.0, right.1, right.2, right.3, depth + 1);
}
```

**Parameters:**
- `FLATNESS_THRESHOLD = 0.5` pixels - curve is flat if deviation ≤ 0.5 pixels
- `MAX_DEPTH = 8` - prevents infinite recursion for highly curved paths

**Algorithm:** De Casteljau's algorithm with midpoint subdivision (t=0.5).

## Edge Cases and Design Choices

### 1. Vertex Double-Counting Prevention
**Problem:** A vertex where two edges meet could be counted twice (once by each edge).

**Solution:** Half-open interval `[y_min, y_max)` ensures each vertex is counted exactly once by the edge that starts at it (not the edge that ends at it).

### 2. Horizontal Edges
**Problem:** Edges parallel to scanlines don't cross them.

**Solution:** Skip entirely - they don't cross scanlines and don't affect in/out state.

### 3. Subpixel Precision
**Problem:** Intersection calculations produce floating-point coordinates.

**Solution:** Store intersections as `f64`, round with `ceil()`/`floor()` during fill to prevent gaps.

### 4. Bitmap Bounds
**Problem:** Polygon may extend beyond bitmap boundaries.

**Solution:** Clamp y-range at start, check x-bounds during fill (line 1045).

### 5. Empty/Odd Intersection Lists
**Problem:** Degenerate polygons or off-screen edges.

**Solution:** Loop gracefully handles odd counts (pairs only, last single ignored).

### 6. Degenerate Polygons
**Problem:** Lines or points produce few intersections.

**Solution:** Zero or one intersection produces no fill pairs.

## Performance Characteristics

**Time Complexity:** O(H × E + I log I)
- H = height of polygon in scanlines  
- E = number of edges
- I = average intersections per scanline
- Each edge tested against each scanline in its y-range

**Space Complexity:** O(E)
- Intersection list per scanline (max E entries)
- No additional persistent allocation

**Optimizations Present:**
1. Y-bounds culling skips irrelevant scanlines
2. Horizontal edge elimination reduces tests
3. In-place sorting on small per-scanline lists
4. Single allocation for intersections vector ( reused across scanlines)

## Integration with Path Rasterization Pipeline

The scanline fill is the final stage of a pipeline:

1. **Path Command Parsing** (`execute_operator` → `op_move_to`, `op_line_to`, `op_cubic_to`)
2. **Path Collection** (`CurrentPath` stores `PathCommand` enum)
3. **Segment Extraction** (`rasterize_path` converts commands to line segments)
4. **Curve Flattening** (`flatten_cubic_bezier_recursive` subdivides Beziers)
5. **CTM Transform** (`transform_point` maps to bitmap space)
6. **Edge Collection** (gather transformed line segments)
7. **Scanline Fill** (`fill_polygon` → this function)

**Call site:**
```rust
// In rasterize_path() at line 945-952
if stroke {
    for (x0, y0, x1, y1) in edges {
        self.draw_line(x0, y0, x1, y1);
    }
} else {
    self.fill_polygon(&edges);  // Fill mode
}
```

## Comparison with Reference Implementation

This implementation follows the classic scanline fill algorithm as described in:
- Foley, van Dam, Feiner, Hughes: *Computer Graphics: Principles and Practice*
- Wikipedia: "Scanline rendering" / "Polygon filling"

**Notable Design Choices:**
1. **Even-odd rule** (not non-zero winding) - simpler for Type3 glyphs
2. **Half-open intervals** - standard technique for vertex handling
3. **Per-scanline edge testing** - straightforward, works well for small glyph bitmaps
4. **Curve flattening before fill** - simplifies intersection logic
5. **No active edge list** - for glyph-sized polygons, O(H×E) is acceptable

## Potential Issues and Limitations

1. **No Anti-Aliasing:** Pixels are either fully black or white - no grayscale at edges
2. **Even-Odd Rule Only:** Non-zero winding rule not supported (Type3 spec doesn't require it)
3. **Scanline Order:** Top-to-bottom traversal - cache-friendly but could be bottom-to-top
4. **Edge Sorting:** Per-scanline sort - could use active edge list for large polygons (unnecessary for glyphs)
5. **No Clipping Before Transform:** Edges transformed to bitmap space before clipping - may create off-screen edges that are immediately discarded

## Verification Notes

**Acceptance Criteria Status:**
- ✅ Read lines 1018-1079 (actual function at lines 994-1052)
- ✅ Document algorithm flow in notes/
- ✅ Identify key data structures (edges, intersections, scanlines)
- ✅ Note edge cases handled (horizontal edges, vertex double-counting, subpixel precision, bounds checking)

**Files Referenced:**
- `crates/pdftract-core/src/font/type3_rasterizer.rs:994-1052` (fill_polygon)
- `crates/pdftract-core/src/font/type3_rasterizer.rs:850-952` (rasterize_path)
- `crates/pdftract-core/src/font/type3_rasterizer.rs:796-847` (flatten_cubic_bezier_recursive)

**Related Beads:**
- Parent: bf-57nmoy (Type3 rasterizer implementation)
- Dependency: bf-tdofbx (path structures available)

## References

- Parent bead: bf-57nmoy
- Implementation: `crates/pdftract-core/src/font/type3_rasterizer.rs:994-1052`
- Edge setup: `crates/pdftract-core/src/font/type3_rasterizer.rs:929-943`
- Curve flattening: `crates/pdftract-core/src/font/type3_rasterizer.rs:796-847`
- Depends on: bf-tdofbx (path structures)
