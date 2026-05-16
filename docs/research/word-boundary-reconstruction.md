# Word Boundary Reconstruction

## Problem Statement

A substantial fraction of real-world PDFs — especially those produced by TeX/LaTeX toolchains, legacy CAD exporters, and older desktop publishing systems — contain no explicit space characters (U+0020) in their content streams. The visual whitespace between words is produced entirely through glyph positioning arithmetic. When a text extractor naively concatenates glyph-to-Unicode mappings without accounting for positional gaps, every word runs together and the output is unreadable. Reconstructing word boundaries is therefore one of the highest-impact correctness problems in PDF text extraction.

---

## 1. Why Spaces Are Missing

The PDF content stream model does not require producers to emit space characters. The spec defines word spacing (`Tw`) and character spacing (`Tc`) as graphics state parameters precisely because positioning is expected to substitute for literal space glyphs.

**TeX/dvips and pdfTeX** operate character-by-character. Each glyph is placed at an absolute or relative position computed by TeX's box-and-glue model. Inter-word glue is converted to a `Td` offset or a positive numeric element inside a `TJ` array; no 0x20 byte ever appears in the string arguments. This is by design: TeX fonts often lack a space glyph entirely, and the Type 1 / Type 2 charstring for character code 0x20, if present, has zero advance width.

**Advance-width substitution** is the general pattern: rather than encoding a space glyph, authoring tools advance the text position by a computed amount equal to the intended inter-word gap, then begin the next word. The result is visually identical to a space but structurally absent from the character stream.

---

## 2. Glyph Advance Width and Position

Every glyph has an advance width defined in the font's metric tables. In PDF:

- **Type 1 / TrueType fonts**: the `Widths` array in the font dictionary maps character codes to glyph widths in 1/1000 of the font's em unit.
- **CIDFonts**: the `DW` key provides a default advance width; the `W` key provides per-glyph overrides as a compact run-length encoding.

After rendering glyph `g` whose advance width is `w_g` (in glyph units), the text position advances to:

```
x_next_expected = x_current + (w_g * font_size / 1000)
```

If the actual x-position of the following glyph deviates positively from `x_next_expected` by more than a threshold, a gap exists. The magnitude of that gap determines its semantic: a small gap is likely a word space; a larger gap may indicate a sentence boundary, a tab stop, or a column separator.

---

## 3. Computing Expected Position Accurately

The simplified formula above omits three graphics state parameters that the PDF spec requires to be applied:

```
x_next_expected =
    x_current
    + (w_g / 1000 * font_size + Tc + Tw_if_space) * Tz / 100
```

Where:

- **`Tc`** (character spacing, set by the `Tc` operator): added to the advance of every glyph.
- **`Tw`** (word spacing, set by the `Tw` operator): added after any single-byte glyph whose character code is 0x20 only. For multi-byte encodings this term never applies.
- **`Tz`** (horizontal scaling percentage, set by the `Tz` operator, default 100): scales the entire horizontal advance.

Failure to apply `Tc` and `Tz` causes systematic over- or under-estimation of expected positions and produces false gap detections. A text matrix transformation (from `Tm` or `Td`) must be applied to convert glyph-space expected positions into device space before comparing with the next glyph's actual device-space coordinates.

---

## 4. The Gap Threshold

The central parameter is the minimum gap magnitude that triggers space insertion. Several strategies exist; an adaptive combination is most robust:

**Fixed fraction of font size.** A gap exceeding `0.2 * font_size` is commonly cited. This works for typical roman typefaces at body text sizes but breaks for narrow condensed faces or for documents that mix font sizes.

**Fraction of average glyph width.** Compute the mean advance width of the glyphs observed on the current text line (excluding outliers). A gap exceeding `0.3 * mean_advance` adapts better to condensed or wide typefaces.

**Font space glyph width.** If the font's `Widths` array contains an entry for character code 0x20, that width (converted to device units as `w_space * font_size / 1000`) is the canonical space reference. This is the most accurate signal when available.

**Fallback half-em.** When no space glyph is defined, use 500 glyph units (half the em) as the reference width: `0.5 * font_size`.

**Adaptive histogram method.** Collect all observed inter-glyph gaps on a page. The distribution is typically bimodal: a sharp peak near zero (tight kerning pairs) and a broader peak near the space width. Fit or locate these two peaks; use the valley between them as the threshold. This requires sufficient glyph count (at least ~50 gaps) to be reliable and can be computed incrementally per-font-size class.

In practice, use the font space glyph width when available, fall back to the adaptive histogram when sufficient data exists, and use `0.25 * font_size` otherwise.

---

## 5. TJ Operator Kerning Arrays

The `TJ` operator accepts an array whose elements alternate between byte strings and numeric offsets. A numeric element displaces the text position by `-offset * font_size / 1000` (the sign convention is reversed from normal advance: positive values move left, negative move right — i.e., positive offsets are backward).

Wait — to be precise per the PDF spec: the displacement is `-(offset / 1000) * font_size` in text space. A **negative** numeric element therefore moves the position forward (adds gap); a **positive** element kerns tighter (moves backward). TeX uses negative offsets for kerning between adjacent letters and large negative offsets (typically below −250 in 1000-unit space) to implement word separation.

The space-detection rule for `TJ` numeric elements:

```
if offset < -space_threshold_in_glyph_units {
    insert_space()
}
```

Where `space_threshold_in_glyph_units` maps the device-space threshold back to 1000-unit glyph space: `threshold_device * 1000 / font_size`. TeX-generated PDFs commonly use offsets around −250 to −350 to represent a normal inter-word space in a 1000-unit font. Treat each transition between a string element and a numeric element, and back to a string, as a potential gap site.

---

## 6. Td/TD/Tm Positioning

When the PDF content stream transitions between text positioning commands, the text matrix changes. Relevant operators:

- **`Td tx ty`**: moves the text line position by `(tx, ty)` in text space.
- **`TD tx ty`**: same as `Td` but also sets `TL = -ty`.
- **`Tm a b c d e f`**: sets the text matrix directly.

Between consecutive text painting operators (Tj, TJ, ' ", etc.), if the text matrix changes such that the new horizontal position in device space exceeds `x_last_glyph_end` by more than the space threshold, insert a space.

Rules:

- **Positive horizontal jump** (new x > expected x by threshold): insert a space.
- **Negative horizontal jump** (new x < expected x): do not insert a space; this is a backtrack, indicating overlapping text, a correction, a superscript/subscript, or right-to-left text reordering. Log as a `backtrack` event in debug metadata.
- **Jump between `BT`/`ET` blocks**: treat the start of each new text object as a potential word boundary using the same threshold rule, comparing the new block's starting position to the ending position of the last glyph from the previous block.

---

## 7. Vertical Gap Interpretation

A change in the y-coordinate of the text position signals a line change rather than a word gap. The threshold:

```
if abs(delta_y) > 0.5 * line_height {
    emit line break
}
```

Where `line_height` is approximated as the current font size multiplied by the leading factor (default 1.2 if no explicit `TL` is set). A vertical gap exceeding approximately 1.5× the line height with no intervening content suggests a paragraph break.

Output conventions:

- **Line break**: emit `\n`.
- **Paragraph break**: emit `\n\n`.
- **Continuation on same line after vertical micro-adjustment** (|Δy| < 0.1 × font_size): treat as same line, no break; this covers subscript/superscript corrections.

Avoid inserting a horizontal space when a vertical line break is also emitted, as the two are mutually exclusive for a given gap event.

---

## 8. Font-Specific Space Width

The space threshold must be font-local. A narrow condensed typeface may have an inter-word space of only 150 glyph units (15% of em), while a wide serif face may use 350 units (35%). Using a global threshold produces both false positives (splitting ligatures) and false negatives (missing spaces in dense faces).

Resolution strategy (in priority order):

1. Look up character code 0x20 in the font's `Widths` array. If present and nonzero, use it.
2. For CIDFonts, look up CID 0x0020 in the `W` array, then fall back to `DW`.
3. Consult the font's `FontDescriptor` for `MissingWidth`; if the space glyph is absent, this is the width assigned to unknown glyphs (often useful as a lower bound).
4. If all metrics are absent, use 500 glyph units as the default half-em heuristic.
5. Override with the adaptive histogram estimate when ≥50 inter-glyph gaps are available for the current font at the current nominal size.

Cache the resolved space width per `(font_resource_name, font_size)` pair to avoid redundant lookups per glyph.

---

## 9. Multi-Column Gap vs. Word Gap

A horizontal gap exceeding approximately `2 * font_size` in device space on the same baseline is not a word gap — it is a tab stop, column separator, or layout gutter. Inserting a space at such a site produces a run of text that incorrectly merges content from separate columns.

Detection heuristic: if `delta_x > 2.0 * font_size` and `abs(delta_y) < 0.1 * font_size`, classify the gap as a **layout gap** rather than a word gap. The appropriate response depends on the layout mode:

- In single-column mode: preserve as a sequence of tab characters or whitespace (extractor-configuration-dependent).
- In multi-column mode: treat as a column boundary and do not concatenate the two spans into the same text run at this point; defer ordering to the reading-order algorithm.

This decision point integrates with the column detection logic described in `complex-layout-reading-order.md`. The word-boundary reconstructor should expose the gap classification (`word_gap`, `layout_gap`, `line_break`, `paragraph_break`) in its span metadata so that the layout stage can consume it without re-deriving it.

---

## 10. Output and Configuration

**Inferred space tagging.** Explicitly encoded space glyphs (character code 0x20 present in the stream) and inferred spaces (inserted by gap detection) must be distinguishable in the intermediate representation. Each inferred space span carries `inferred: true` in its debug metadata. This enables downstream consumers to audit false positives without reprocessing the PDF.

**Configuration parameter: `space_detection_threshold`.** Expose a per-extractor configuration value:

```rust
pub enum SpaceThreshold {
    /// Automatically select per font using the priority strategy above.
    Auto,
    /// Fixed fraction of font size (e.g., 0.25).
    FractionOfFontSize(f32),
    /// Absolute value in device-space points.
    AbsolutePoints(f32),
}
```

Default: `SpaceThreshold::Auto`. When `Auto`, the extractor uses font metric lookups with adaptive histogram fallback. Callers processing documents where every inter-word gap is explicit can set `SpaceThreshold::AbsolutePoints(f32::MAX)` to disable inference entirely.

**Per-page statistics.** The `PageOutput` structure exposes:

```rust
pub struct PageSpaceStats {
    pub explicit_space_count: u32,
    pub inferred_space_count: u32,
    pub backtrack_event_count: u32,
    pub layout_gap_count: u32,
}
```

A high `inferred_space_count` relative to `explicit_space_count` (ratio > 5:1) is a reliable signal that the document was produced by a TeX toolchain or a similarly space-omitting authoring system. This signal can inform downstream heuristics such as ligature normalization and hyphenation handling.

---

## Implementation Notes for Rust

- Maintain a `TextState` struct that tracks `Tc`, `Tw`, `Tz`, `font_size`, `text_matrix`, and `line_matrix` as mutable graphics state, updated by the corresponding PDF operators.
- After each glyph is rendered, record `glyph_end_x` (device space) as `glyph_start_x + advance_device`.
- Before rendering the next glyph, compute `expected_x` from the full formula including `Tc` and `Tz`; compare actual x to `expected_x`; classify and emit gap events.
- For `TJ` arrays, iterate elements in order; accumulate string runs and emit gap events at each sign-significant numeric element before consuming the next string run.
- Store the font space width cache in a `HashMap<(ObjectId, OrderedFloat<f32>), f32>` keyed by font object ID and nominal font size to handle fonts used at multiple sizes.
- The adaptive histogram should bucket gaps into bins of width `0.01 * font_size` and perform a simple two-peak scan (find the global maximum, zero out ±3 bins, find the second maximum) to locate the space-width peak without a full GMM fit.
