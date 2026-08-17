# Edge Struct Examination (Bead bf-gnicui)

## Location
File: `crates/pdftract-core/src/font/type3_rasterizer.rs`
Lines: 803-814

## Structure Definition

```rust
#[derive(Debug, Clone, Copy)]
pub(crate) struct Edge {
    /// Current X intersection position (tracked as we move through scanlines)
    pub(crate) x: i32,
    /// Minimum Y coordinate (top of edge)
    pub(crate) y_min: i32,
    /// Maximum Y coordinate (bottom of edge)
    pub(crate) y_max: i32,
    /// Change in X across the edge
    pub(crate) dx: i32,
    /// Change in Y across the edge
    pub(crate) dy: i32,
}
```

## Field Analysis

All fields use `i32` (signed 32-bit integer) type:

| Field | Type | Purpose |
|-------|------|---------|
| `x` | i32 | Current X intersection position during scanline processing |
| `y_min` | i32 | Minimum Y coordinate (top of the edge) |
| `y_max` | i32 | Maximum Y coordinate (bottom of the edge) |
| `dx` | i32 | Change in X across the edge (delta X) |
| `dy` | i32 | Change in Y across the edge (delta Y) |

## Derived Traits
- `Debug`: Enables debug formatting
- `Clone`: Allows cloning of Edge instances
- `Copy`: Enables copy semantics (Edge is a simple POD type)

## Context
The Edge struct is used in the Type3 font rasterizer for polygon filling operations. It represents edges in the scanline conversion algorithm, tracking the edge's position as it advances through scanlines.

## Implementation Notes
- The struct is crate-private (`pub(crate)`)
- All fields are public within the crate
- Uses integer coordinates for scanline processing
- Part of the Type3 glyph rendering pipeline
