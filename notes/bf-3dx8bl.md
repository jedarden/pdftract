# Scanline Fill Algorithm Analysis

**Bead:** bf-3dx8bl
**Date:** 2026-08-06
**Reference:** `crates/pdftract-core/src/font/type3_rasterizer.rs` lines 992-1052 (fill_polygon method)

## Overview

The Type3 rasterizer implements a classic **scanline polygon fill algorithm** for rendering filled vector paths in PDF Type3 fonts. The algorithm operates on bitmap coordinates and handles arbitrary polygon shapes by computing edge-scanline intersections.

## Algorithm Structure

### Phase 1: Edge Table Setup (lines 929-943)

The algorithm begins by constructing an edge table from the path commands:

```rust
// Path segments are collected from PDF commands
let mut segments = Vec::new();
for cmd in &self.path.commands {
    match cmd {
        PathCommand::LineTo(p) => segments.push((start, *p)),
        PathCommand::Rect(x, y, w, h) => /* 4 line segments */,
        PathCommand::CubicTo(cp1, cp2, end) => /* flatten bezier */,
        // ... other commands
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

**Key data structures:**
- `segments: Vec<(Point, Point)>` - Path segments in user space
- `edges: Vec<(i32, i32, i32, i32)>` - Edges in bitmap coordinates (x0, y0, x1, y1)

**Path command handling:**
- **MoveTo** - Updates current point without creating segments
- **LineTo** - Creates a single line segment
- **Rect** - Decomposes into 4 line segments (rectangle perimeter)
- **ClosePath** - Connects current point back to the initial move point
- **CubicTo** - Flattens Bezier curves into multiple line segments
- **ShorthandCubicTo** - Variant of cubic with implicit control points

### Phase 2: Y-Bounds Calculation (lines 998-1009)

Before the scanline loop, the algorithm determines the vertical extent of the polygon:

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

This optimization ensures scanlines are only processed where the polygon actually exists.

### Phase 3: Scanline Loop (lines 1011-1051)

For each scanline `y` from `min_y` to `max_y`:

#### Step 3a: Find Intersections (lines 1015-1033)

```rust
let mut intersections = Vec::new();

for &(x0, y0, x1, y1) in edges {
    // Skip horizontal edges
    if y0 == y1 {
        continue;
    }
    
    // Half-open interval: include lower, exclude upper
    let (y_min, y_max) = if y0 < y1 { (y0, y1) } else { (y1, y0) };
    if y_min <= y && y < y_max {
        // Calculate x intersection using linear interpolation
        let dy = y1 - y0;
        let t = (y - y0) as f64 / dy as f64;
        let x = x0 as f64 + t * (x1 - x0) as f64;
        intersections.push(x);
    }
}
```

**Critical edge case - Half-open intervals:**
The condition `y_min <= y && y < y_max` prevents double-counting vertices. A vertex shared by two edges would be counted twice if both edges used closed intervals on both ends. By using half-open intervals (including the lower endpoint, excluding the upper), each vertex is counted exactly once.

#### Step 3b: Sort Intersections (lines 1035-1036)

```rust
intersections.sort_by(|a, b| a.partial_cmp(b).unwrap());
```

The intersections are sorted by x-coordinate to determine fill order.

#### Step 3c: Fill Between Pairs (lines 1038-1050)

```rust
for i in (0..intersections.len()).step_by(2) {
    if i + 1 < intersections.len() {
        let x_start = intersections[i].ceil() as i32;
        let x_end = intersections[i + 1].floor() as i32;
        
        for x in x_start..=x_end {
            if x >= 0 && x < width {
                self.bitmap.set(x, y, 0);  // Black pixel
            }
        }
    }
}
```

**Pixel boundary handling:**
- `x_start.ceil()` - First pixel at or to the right of the intersection
- `x_end.floor()` - Last pixel at or to the left of the intersection

This ensures pixels are only filled when the scanline truly passes through the polygon interior.

## Key Data Structures

| Structure | Type | Purpose |
|-----------|------|---------|
| `edges` | `Vec<(i32, i32, i32, i32)>` | Polygon edges as (x0, y0, x1, y1) |
| `intersections` | `Vec<f64>` | X-coordinates of edge-scanline intersections |
| `segments` | `Vec<(Point, Point)>` | Path segments in user space (before transform) |

## Edge Cases Handled

1. **Horizontal edges** - Skipped entirely (`y0 == y1` check), as they don't contribute to scanline intersections
2. **Vertex double-counting** - Prevented by half-open interval `y_min <= y && y < y_max`
3. **Bitmap boundaries** - Both y-bounds (lines 1008-1009) and x-bounds (line 1045) are clamped
4. **Odd intersection counts** - The `step_by(2)` loop gracefully handles unpaired intersections (though properly-formed polygons should always have even counts)
5. **Bezier curves** - Flattened to line segments before rasterization (via `flatten_cubic_bezier`)

## Mathematical Foundation

The intersection calculation uses linear interpolation (line 1027-1030):

```
x = x0 + (y - y0) * (x1 - x0) / (y1 - y0)
```

This derives from the parametric line equation:

```
P(t) = P0 + t * (P1 - P0)
```

where `t = (y - y0) / (y1 - y0)` is the fraction along the edge at scanline `y`.

## Performance Characteristics

- **Time complexity:** O(E + H * (E + I log I)) where E = edges, H = polygon height, I = average intersections per scanline
- **Space complexity:** O(E) for edge storage, O(I) for intersections
- **Optimizations:** Y-bounds clamping avoids processing empty scanlines

## Integration Points

The scanline algorithm is invoked from `rasterize_path` (line 852) when `stroke` is `false`:

```rust
if stroke {
    for (x0, y0, x1, y1) in edges {
        self.draw_line(x0, y0, x1, y1);  // Stroke mode
    }
} else {
    self.fill_polygon(&edges);  // Fill mode (scanline algorithm)
}
```

## References

- Parent bead: bf-57nmoy
- Implementation: `crates/pdftract-core/src/font/type3_rasterizer.rs:994-1051`
- Edge setup: `crates/pdftract-core/src/font/type3_rasterizer.rs:929-943`
- Depends on: bf-tdofbx (path structures)
