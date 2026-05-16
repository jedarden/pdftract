# Graphics State Tracking for PDF Text Extraction

Correct text extraction in pdftract requires more than decoding glyph sequences. Whether a glyph is visible, what color it renders at, and where on the page it lands all depend on state that accumulates across operators in the content stream. Mishandling this state causes invisible text to contaminate output and visible text to be silently dropped.

---

## 1. The Graphics State Stack

The PDF content stream is a stateful machine. A graphics state object encapsulates every rendering parameter at a point in the stream. The `q` operator pushes a complete clone of the current state onto a stack; `Q` pops and restores it. The PDF specification (ISO 32000-2, §8.4.2) recommends implementations support at least 28 nesting levels.

The state that must be cloned on `q` includes:

- **CTM** — current transformation matrix, 6 floats `[a b c d e f]`
- **Clipping path** — the active clip region
- **Color space and color** — separately for fill and stroke
- **Line parameters** — width, cap style, join style, miter limit, dash pattern
- **Rendering intent** — a name value
- **Stroke adjustment flag**
- **Blend mode** — a name (e.g., `/Normal`, `/Multiply`)
- **Soft mask** — a dictionary or `/None`
- **Alpha constants** — `ca` (fill alpha) and `CA` (stroke alpha), both `f32` in `[0.0, 1.0]`
- **Alpha is shape flag** (`AIS`)
- **Text state** — the entire set of text parameters described in §2

A missing or shallow clone on `q` is a latent bug: an inner content stream that changes color or alpha will corrupt the outer stream's state after `Q`.

---

## 2. Text State Within the Graphics State

Text state is a subgroup of the graphics state and is saved and restored with `q`/`Q`. The text state operators and their targets:

| Operator | Parameter modified |
|----------|--------------------|
| `Tf name size` | Current font (resource name) and font size in text space |
| `Tc value` | Character spacing (added after each glyph, in text space units) |
| `Tw value` | Word spacing (added after each ASCII space, 0x20) |
| `Tz value` | Horizontal scaling, expressed as a percentage (100 = normal) |
| `TL value` | Leading, used by `T*` and `'` operators |
| `Tr value` | Text rendering mode (integer 0–7) |
| `Ts value` | Text rise, vertical offset in text space |

**Text matrices are separate.** The text matrix `Tm` and the text line matrix `Tlm` are *not* part of the graphics state. They are initialized by `BT` (begin text object) and are undefined outside a `BT`/`ET` pair. `Td`, `TD`, `T*`, and `Tm` modify these matrices during a text object. They are not saved or restored by `q`/`Q`. Implementations that try to persist them across `q`/`Q` will produce incorrect glyph positions.

---

## 3. Color Space Tracking

The current color space for fill and stroke are tracked independently.

**Explicit color space selection:**
- `cs name` — set fill color space to a named entry from `Resources/ColorSpace`
- `CS name` — set stroke color space to a named entry from `Resources/ColorSpace`
- Device names `/DeviceRGB`, `/DeviceGray`, `/DeviceCMYK` can appear directly

**Shorthand operators that set both space and color atomically:**

| Operator | Color space | Arguments |
|----------|-------------|-----------|
| `rg r g b` | DeviceRGB (fill) | three floats in [0,1] |
| `RG r g b` | DeviceRGB (stroke) | three floats in [0,1] |
| `g gray` | DeviceGray (fill) | one float in [0,1] |
| `G gray` | DeviceGray (stroke) | one float in [0,1] |
| `k c m y k` | DeviceCMYK (fill) | four floats in [0,1] |
| `K c m y k` | DeviceCMYK (stroke) | four floats in [0,1] |

**General color operators** `sc`/`scn` (fill) and `SC`/`SCN` (stroke) set the color within the currently active color space. The argument count depends on the space.

**Normalized luminance for visibility.** To determine whether text color contrasts with the page background (typically white), convert to a single luminance value:

- DeviceGray: luminance = `gray`
- DeviceRGB: luminance = `0.2126 * r + 0.7152 * g + 0.0722 * b` (sRGB coefficients per IEC 61966-2-1)
- DeviceCMYK: convert to RGB first: `r = (1-c)*(1-k)`, `g = (1-m)*(1-k)`, `b = (1-y)*(1-k)`, then apply the RGB formula
- CalRGB, ICCBased: use the RGB channel values after applying the color space transformation, then the RGB formula

Text with luminance near 1.0 on a white background is invisible regardless of alpha. Track fill color luminance as a `f32`; any value above approximately 0.95 against a white background should be flagged as potentially invisible.

---

## 4. ExtGState Dictionary

The `gs name` operator loads a graphics state parameter dictionary from `Resources/ExtGState`. This is the primary mechanism for setting transparency parameters. Keys relevant to text:

| Key | Type | Effect |
|-----|------|--------|
| `ca` | number | Fill (non-stroking) alpha constant, 0.0–1.0 |
| `CA` | number | Stroke alpha constant, 0.0–1.0 |
| `BM` | name or array | Blend mode |
| `SMask` | dict or `/None` | Soft mask; `/None` clears any active mask |
| `AIS` | boolean | Alpha is shape |
| `SA` | boolean | Stroke adjustment |
| `Font` | array `[ref size]` | Sets current font and size, same effect as `Tf` |

`apply_gs` must iterate the dictionary and update only the keys present — absent keys leave the corresponding state unchanged.

**SMask dictionary structure.** When `SMask` is a dictionary rather than `/None`:
- `S` — `/Alpha` or `/Luminosity`: determines how the mask value is extracted from the group result
- `G` — a Form XObject stream that is rendered to produce the mask
- `BC` — backdrop color (array of color components)
- `TR` — transfer function applied to the mask values

---

## 5. Clipping Path Management

The initial clipping path for a page is the MediaBox (or CropBox if present). Within content streams, clipping is modified by `W` (nonzero winding rule) and `W*` (even-odd rule). These operators are path-painting modifiers: they take effect *after* path construction is complete and *before* or *instead of* a painting operator. The sequence is:

1. Construct path via `m`, `l`, `c`, `re`, etc.
2. Issue `W` or `W*` — marks intent to clip
3. Issue a painting operator (`S`, `f`, `n`, etc.) or just `n` to apply the clip without painting

The clip region is **intersected** with the constructed path shape — it can only shrink, never expand. The resulting clip becomes the new current clipping path.

For text extraction, maintaining an exact polygon clip is expensive. A practical approximation: track the clip as an axis-aligned bounding box (`[x_min, y_min, x_max, y_max]` in user space). When `W`/`W*` fires, intersect the tracked bbox with the bounding box of the current path. For most documents this approximation is exact; non-rectangular clips are edge cases flagged for further analysis.

The clipping path is fully saved and restored by `q`/`Q`.

---

## 6. Current Transformation Matrix

The CTM is a 3×3 matrix in column-major form, represented by 6 values `[a b c d e f]` with the third row implicitly `[0 0 1]`. The `cm a b c d e f` operator **pre-multiplies** the current CTM by the new matrix:

```
CTM_new = [a b c d e f] × CTM_current
```

In row-vector convention (PDF uses row vectors), concatenation means the new transform is applied first. The implementation must preserve exact multiplication order.

Matrix concatenation:

```rust
fn concat(m: [f64; 6], ctm: [f64; 6]) -> [f64; 6] {
    [
        m[0]*ctm[0] + m[1]*ctm[2],
        m[0]*ctm[1] + m[1]*ctm[3],
        m[2]*ctm[0] + m[3]*ctm[2],
        m[2]*ctm[1] + m[3]*ctm[3],
        m[4]*ctm[0] + m[5]*ctm[2] + ctm[4],
        m[4]*ctm[1] + m[5]*ctm[3] + ctm[5],
    ]
}
```

Glyph positions are computed in text space, transformed by the text matrix `Tm`, then by the CTM. The resulting device-space coordinates determine where the glyph appears on the page and whether it falls within the clipping bbox.

---

## 7. Blend Mode Effects on Visibility

The blend mode controls how a graphics object composites over the content beneath it. For text extraction, the key question is whether the blend mode can render text invisible.

- **`/Normal` and `/Compatible`** — the source color replaces the destination at the source's alpha. At `ca=1.0`, text is fully opaque in its declared color.
- **`/Multiply`** — multiplies source and destination color channels. Text drawn in black (0,0,0) on any background remains black. Text drawn in white (1,1,1) becomes invisible against a white background.
- **`/Screen`** — `1 - (1-s)*(1-d)`. Light-colored text lightens rather than covers.
- **`/Overlay`, `/HardLight`, `/SoftLight`** — result depends on the luminance of the destination, which is unknown without rendering.
- **`/Difference`, `/Exclusion`** — text color is the absolute difference with the background.

Practical rule: if blend mode is not `/Normal` or `/Compatible`, the actual rendered color cannot be determined without knowing the destination. Flag such text as `blend_mode_dependent` and rely on `ca` as the primary visibility signal. A `ca` of 0.0 guarantees invisibility; any positive value with a non-Normal blend mode is ambiguous.

---

## 8. Soft Mask Interaction

A soft mask applies a per-pixel transparency derived from a separately rendered Form XObject. The effective alpha at any pixel is `ca * mask_value(x, y)`. Since `mask_value` is in `[0.0, 1.0]`, the constant alpha `ca` is an upper bound on the effective alpha.

Fully rendering the mask Form XObject is expensive and outside the scope of a text extraction pass. The practical approach:

1. When `SMask` is set to a dictionary (not `/None`), set a boolean flag `soft_mask_present: true` on the graphics state.
2. Use `ca` as a lower-bound visibility signal: if `ca == 0.0`, text is invisible regardless of the mask.
3. For `ca > 0.0` with an active soft mask, text is marked `soft_mask_present` and conservatively included in output — it may be partially or fully transparent depending on the mask, but exclusion risks losing real content.

Clearing: `gs` with `SMask /None` clears the active soft mask.

---

## 9. Form XObject Graphics State Isolation

When `Do name` invokes a Form XObject, the PDF processor must:

1. Save the current graphics state (equivalent to `q`)
2. Concatenate the Form XObject's `/Matrix` (if present) with the current CTM
3. Apply the Form XObject's `/BBox` as an additional clip
4. Parse the Form XObject's content stream, using its `/Resources` dictionary for name resolution
5. Restore the graphics state (equivalent to `Q`) when the stream ends

Graphics state mutations inside the Form XObject — color changes, alpha updates, CTM modifications, clip changes — do not persist after the `Do` operator completes. Resource name resolution switches to the Form XObject's `/Resources` during parsing and reverts after. Failing to isolate Form XObject state is a common source of color and font state corruption.

---

## 10. Implementation: the `GraphicsState` Struct

```rust
#[derive(Clone)]
pub struct GraphicsState {
    // Transformation
    pub ctm: [f64; 6],

    // Color (fill)
    pub fill_color_space: ColorSpace,
    pub fill_color: ColorValue,
    pub fill_alpha: f32,

    // Color (stroke)
    pub stroke_color_space: ColorSpace,
    pub stroke_color: ColorValue,
    pub stroke_alpha: f32,

    // Transparency
    pub blend_mode: BlendMode,
    pub soft_mask_present: bool,

    // Clipping (bbox approximation in user space)
    pub clip_bbox: Option<[f64; 4]>,  // [x_min, y_min, x_max, y_max]

    // Text state
    pub text_rendering_mode: u8,      // 0–7 per PDF spec
    pub text_rise: f64,
    pub font_name: Option<String>,
    pub font_size: f64,
    pub char_spacing: f64,
    pub word_spacing: f64,
    pub horiz_scaling: f64,           // percentage, default 100.0
    pub leading: f64,
}

pub struct GraphicsStateStack {
    stack: Vec<GraphicsState>,
}

impl GraphicsStateStack {
    pub fn save(&mut self) {
        let top = self.stack.last().expect("empty stack").clone();
        self.stack.push(top);
    }

    pub fn restore(&mut self) {
        if self.stack.len() > 1 {
            self.stack.pop();
        }
    }

    pub fn current(&mut self) -> &mut GraphicsState {
        self.stack.last_mut().expect("empty stack")
    }
}
```

**`is_text_visible`** must combine all signals:

```rust
pub fn is_text_visible(&self) -> bool {
    // Rendering mode 3 = invisible (clip only)
    if self.text_rendering_mode == 3 { return false; }

    // Zero alpha = invisible
    if self.fill_alpha == 0.0 { return false; }

    // Clipping: if clip bbox has zero area, text is outside
    if let Some(bbox) = self.clip_bbox {
        if bbox[0] >= bbox[2] || bbox[1] >= bbox[3] { return false; }
    }

    // High-luminance fill on assumed white background
    let lum = self.fill_color.luminance();
    if lum > 0.95 && self.fill_alpha > 0.0 { return false; }

    true
}
```

**`apply_gs`** iterates the ExtGState dictionary entries and applies each recognized key. Unknown keys are ignored per the spec's extensibility rules.

**`apply_cm`** calls the `concat` function above to pre-multiply the new matrix into the current CTM.

---

## Summary

Full graphics state tracking is not optional for accurate text extraction. The rendering mode, alpha constants, blend mode, soft mask, fill color, clipping path, and CTM each independently contribute to whether a glyph appears on the page and where. The stack mechanics of `q`/`Q` must clone the complete state. Form XObjects must isolate their state changes. Text matrices (`Tm`, `Tlm`) are separate from the graphics state and must not be conflated with it. The `is_text_visible` predicate synthesizes all tracked signals into a single decision that drives inclusion or exclusion of glyphs from the extraction output.
