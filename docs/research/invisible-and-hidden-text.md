# Invisible and Hidden Text in PDFs

## Overview

PDF files routinely contain text that is present in the byte stream but not visually rendered to a reader. This occurs through several independent mechanisms: the text rendering mode operator, color matching with the page background, zero-opacity graphics states, clip-path suppression, and near-zero scaling. For a text extraction library, invisible text is often the most valuable content on the page — particularly in scan-based PDF/A files where an OCR layer carries the only machine-readable text. This document covers detection algorithms for each invisibility mechanism and the output policy `pdftract` should apply.

---

## 1. Text Rendering Modes (`Tr`)

The PDF specification (ISO 32000-2 §9.3.6) defines the `Tr` (text rendering mode) operator, which controls how glyph outlines are applied to the page. The argument is an integer 0–7:

| Mode | Name | Fill | Stroke | Clip |
|------|------|------|--------|------|
| 0 | Fill | yes | no | no |
| 1 | Stroke | no | yes | no |
| 2 | Fill then stroke | yes | yes | no |
| 3 | Invisible | no | no | no |
| 4 | Fill + clip | yes | no | yes |
| 5 | Stroke + clip | no | yes | yes |
| 6 | Fill + stroke + clip | yes | yes | yes |
| 7 | Clip only | no | no | yes |

Mode 3 is the canonical invisible text mechanism. The glyph is processed by the text engine — Unicode mapping, advance width, and spacing operators all apply normally — but nothing is painted. This is the mechanism used by scan-based PDF/A files to overlay OCR output. Mode 7 is similarly invisible but accumulates the glyph outline into the current clip path.

During content stream parsing, the current `Tr` value must be tracked as part of the graphics state. It defaults to 0 at the start of each page content stream and is reset by `q`/`Q` pushes and pops along with the rest of the graphics state. Every text span extracted should carry the rendering mode at the time of its `Tj`, `TJ`, `'`, `"`, or similar text-showing operator.

---

## 2. Invisible Text Over Scans (PDF/A Pattern)

The dominant real-world source of mode-3 text is the OCR-over-scan pattern used in PDF/A-3 and related archival formats. The structure is:

1. A raster image XObject is placed on the page via `Do`, covering substantially the full page area (typically the entire MediaBox).
2. A sequence of mode-3 text spans is overlaid at positions that correspond to the OCR engine's bounding box output for each word or glyph.

**Detection heuristic.** Flag a page as using this pattern when:
- At least one image XObject with an area ≥ 80% of the page MediaBox is present.
- At least one text span with `Tr == 3` exists on the same page.
- The text spans cluster within the image bounding box bounds.

When this pattern is detected, the mode-3 text spans are the authoritative extraction result. Re-running OCR on the raster would be redundant and potentially lower quality. Mark these spans with `source: "ocr_invisible_layer"` so callers can distinguish them from normally rendered text. The raster image itself should not be forwarded to an OCR pipeline when invisible text is already present.

**Coordinate correspondence.** OCR layers typically place each word or character at the correct position on the page coordinate system. Verify plausibility by checking that the text spans, when rendered at their specified positions, fall within the image XObject's bounding box. Spans placed outside the image area are likely artifacts and should be flagged separately.

---

## 3. White Text on White Background

Text whose fill color matches the page background is visually hidden even at `Tr 0`. Detecting this requires tracking the current fill color through the content stream and comparing it against the effective background.

**Color tracking operators.** The current fill color is set by:
- `rg r g b` — DeviceRGB fill color (values 0.0–1.0)
- `RG r g b` — DeviceRGB stroke color
- `k c m y k` — DeviceCMYK fill color
- `K c m y k` — DeviceCMYK stroke color
- `g gray` — DeviceGray fill
- `G gray` — DeviceGray stroke
- `cs name` — set fill color space to a named space
- `CS name` — set stroke color space
- `sc`/`scn` — set fill color components in current fill color space
- `SC`/`SCN` — set stroke color components in current stroke color space

The graphics state stack (`q`/`Q`) must save and restore the full color state including both the current color space and the current color value vector.

**White in each color space.** The canonical white values are:
- DeviceGray: `1.0`
- DeviceRGB: `1.0 1.0 1.0`
- DeviceCMYK: `0.0 0.0 0.0 0.0`
- CalRGB, CalGray, ICCBased: requires converting to a perceptual space (e.g., CIELAB) and checking L* ≥ 95.

**Background color determination.** The page background is ambiguous. The PDF viewer default is white, but a content stream may paint a filled rectangle covering the MediaBox with an arbitrary color before placing text. The most reliable approach is to build a simple z-order list of opaque filled rectangles that cover each point of the page, then for any text glyph center point, walk the z-order list downward from the text to find the topmost background element. If the background is an image XObject, extracting the background color at a point requires sampling the image raster — a heavier operation. In practice, comparing the fill color against `white` (per-color-space definition above) catches the overwhelming majority of white-on-white cases without full compositing.

---

## 4. Zero-Opacity and Transparency

PDF transparency (ISO 32000-2 §11) introduces alpha values separate from the color operators.

**Graphics state alpha.** The `gs` operator references an ExtGState resource dictionary. The relevant keys:
- `ca` — constant alpha for non-stroking (fill) operations; float 0.0–1.0
- `CA` — constant alpha for stroking operations; float 0.0–1.0

A text span with `ca == 0.0` (or effectively zero, e.g., < 0.01) at `Tr 0` is invisible. At `Tr 1`, invisibility is governed by `CA`. At `Tr 2`, both `ca` and `CA` must be checked. Track the current `ca` and `CA` values as part of the graphics state, initializing them to 1.0 per the PDF default.

**Soft masks.** A soft mask (`SMask` in the ExtGState dictionary) may reduce effective alpha further. An `SMask` of type `Luminosity` or `Alpha` applied to a transparency group containing text can render that text invisible even if `ca` is nonzero. Full soft mask evaluation requires compositing the transparency group, which is expensive. For detection purposes, flag any text span inside a content stream with an active `SMask` (i.e., `SMask` is not `/None`) as potentially invisible and emit it with `visibility_confidence: low`.

---

## 5. Clipped-Away Text

The clip path operators `W` (nonzero winding rule) and `W*` (even-odd rule) modify the current clipping region by intersecting it with the current path. Text rendered when the clip region has zero or negligible area is visually absent.

**Clip path tracking.** The clipping region is part of the graphics state and is saved/restored by `q`/`Q`. It starts as the page MediaBox. Each `W` or `W*` narrows it by intersecting with the path constructed by the preceding `m`/`l`/`c`/`re` operators. The current transformation matrix (`cm`) transforms subsequent coordinates and must be applied to path coordinates before intersection.

**Detection.** For each text glyph, compute its bounding box in default user space (using the current text matrix, font metrics, and font size). Intersect this rectangle with the current clip region. If the intersection area is below a threshold (e.g., < 0.01 square points), mark the glyph as clipped-invisible.

Exact clip path intersection for arbitrary Bézier paths is expensive. A practical approximation: represent the clip path as an axis-aligned bounding box (AABB) at each step. This will produce false negatives for concave clip paths but catches the common case of clipping to a zero-width or zero-height rectangle.

---

## 6. Text Scaled to Near-Zero

A font size of 0.0 or near-zero renders glyphs at sub-pixel scale, making them invisible:

- `Tf fontname size` — if `size < 0.1`, the rendered glyph height is negligible.
- `Tz scale` — horizontal scaling as a percentage; `Tz 0` collapses all glyph advance widths to zero, stacking all characters at a single point.

**Detection thresholds.** Flag a text span as size-invisible when:
- The effective font size (after applying the current transformation matrix scale factor) is < 0.1 points, or
- `Tz` is < 1.0 (1% horizontal scaling).

The effective font size must account for the CTM. Compute the scale factor as `sqrt(a² + b²)` from the current CTM `[a b c d e f]` and multiply by the `Tf` size argument.

---

## 7. Color Space Detection for Fills

Determining whether a fill is white requires correctly resolving the current color space. The fill color space is established by `cs` and defaults to DeviceGray in early content streams or DeviceRGB in most modern PDFs. Color space names resolve through the page's `Resources/ColorSpace` dictionary. The four categories:

- **Device spaces** (DeviceGray, DeviceRGB, DeviceCMYK): white values are fixed as above.
- **CIE-based spaces** (CalGray, CalRGB, Lab): convert the color value to CIE L*a*b* and check L* ≥ 95, |a*| ≤ 5, |b*| ≤ 5.
- **ICCBased**: requires loading and evaluating the embedded ICC profile. For extraction purposes, inspect the `Alternate` entry in the ICCBased stream dictionary as a fallback color space and apply its whiteness rule.
- **Indexed**: the color value is a table index; look up the base color and apply the base space rule.
- **Pattern** and **Separation/DeviceN**: too complex for simple whiteness detection; flag as `visibility_confidence: low`.

---

## 8. Intentional Obfuscation and DRM

Some PDFs deliberately exploit text extraction to prevent accurate copying while maintaining visual fidelity:

**Position shuffling.** Individual characters are placed at arbitrary positions via separate `Tj` or `TJ` operators with large kerning adjustments, making the logical reading order in the byte stream non-sequential. Visually, the PDF renderer draws the correct text because the positions are meticulously computed. Extraction that reads characters in byte-stream order produces gibberish. Detection: flag pages where the average glyph-center-to-glyph-center distance divided by glyph advance width exceeds a threshold (e.g., > 5.0), suggesting non-linear character placement.

**Deliberate CMap corruption.** The `ToUnicode` CMap in the font dictionary maps glyph IDs to Unicode code points. An adversarial PDF may install a ToUnicode CMap where the mappings are deliberately wrong — e.g., all glyphs map to `U+0041` (A), or the CMap is omitted entirely. The visual rendering uses the actual glyph outlines and is correct; extraction using ToUnicode returns nonsense. Detection: compare the extracted Unicode string entropy against the expected entropy for the detected language. A string of all-identical characters or a very low-entropy sequence over a full paragraph is a strong signal. `pdftract` has no reliable recovery path for this case; it should document the limitation and report `extraction_quality: obfuscated`.

---

## 9. Output Policy

**Default behavior.** Extract all text spans regardless of rendering mode or computed visibility. This is the most useful default for search indexing and RAG pipelines, which benefit from invisible OCR layers.

**Span metadata.** Each extracted `TextSpan` should carry:

```rust
pub struct TextSpan {
    pub text: String,
    pub rendering_mode: u8,       // Tr value 0–7
    pub visible: bool,            // false if any invisibility mechanism applies
    pub visibility_flags: VisibilityFlags, // bitfield: INVISIBLE_TR | WHITE_COLOR | ZERO_ALPHA | CLIPPED | NEAR_ZERO_SIZE
    pub source: SpanSource,       // Normal | OcrInvisibleLayer | Unknown
    pub visibility_confidence: Confidence, // High | Low (low when SMask or DeviceN color)
}
```

**Caller filtering.** Provide an extraction option `visible_only: bool` that filters the output to spans where `visible == true`. This is appropriate for display-faithful extraction. Default: `false`.

**OCR invisible layer.** Spans with `rendering_mode == 3` on a page matching the scan-pattern heuristic are assigned `source: SpanSource::OcrInvisibleLayer`. These spans should not be deduplicated against OCR pipeline output — they are the preferred result.
