# Word Boundary Reconstruction

**Version:** 1.0
**Status:** Final
**Last Updated:** 2026-05-24
**Reference:** Plan line 1529 (adaptive threshold + Tc/Tw/Tz reference)

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

### 3.1 Tc/Tw/Tz Formula — Complete Specification

The complete formula for expected next-glyph position, including all graphics state corrections, is:

```
expected_advance = (w_g / 1000 * font_size + Tc + Tw_if_space) * Tz / 100
x_next_expected = x_current + expected_advance
```

Where:

| Parameter | Operator | Type | Application |
|-----------|----------|------|-------------|
| `w_g` | — | f32 | Glyph width from font's `Widths` array (in 1/1000 em units) |
| `font_size` | `Tf fs` | f32 | Font size in text space points |
| `Tc` | `Tc tc` | f32 | **Character spacing** — additive, applied to **every** glyph |
| `Tw` | `Tw tw` | f32 | **Word spacing** — additive, applied **only** when glyph is U+0020 (SPACE) |
| `Tw_if_space` | — | f32 | `Tw` if `codepoint == 0x20`, else `0.0` |
| `Tz` | `Tz tz` | f32 | **Horizontal scaling** — multiplicative, default 100 (percentage) |

**Critical implementation notes:**

- **Tz is multiplicative**, not additive. The entire advance `(w_g/1000 * font_size + Tc + Tw_if_space)` is multiplied by `Tz / 100`. A Tz value of 50 compresses all spacing to half; a Tz of 150 expands to 1.5×.
- **Tw applies ONLY to U+0020**. A common bug is applying `Tw` to all whitespace (including U+00A0 NO-BREAK SPACE, U+2009 THIN SPACE, etc.). Only the single-byte character code 0x20 triggers word spacing.
- **Tc applies universally**. Character spacing is added after every glyph, regardless of its character code.
- **All values are in text space**. The comparison between `expected_advance` and the actual device-space gap must happen after both are transformed to the same coordinate system. Per the plan (line 1550), perform the comparison **in text space** before CTM transformation.

### 3.2 Text-Space vs. Device-Space Comparison

Per plan line 1550, the threshold comparison is performed in **text space**, not device space. This avoids CTM-induced skew in gap detection:

```
// Correct: compare in text space
let gap_text = actual_next_x_text - expected_next_x_text;
if gap_text > threshold_text {
    insert_space();
}

// Incorrect: comparing in device space after CTM
// This fails when CTM includes rotation, non-uniform scaling, or skew
```

The `actual_next_x_text` value is derived by applying the inverse CTM to the next glyph's device-space position. This ensures that threshold semantics (e.g., "0.25 × font_size") remain stable regardless of page transformations.

---

## 4. The Gap Threshold — Adaptive Algorithm

Per plan line 1547, the adaptive threshold algorithm is the single source of truth for word boundary detection in Phase 3.

### 4.1 Algorithm Specification

```
// Initial threshold (used for first 20 glyphs after font switch)
threshold = 0.25 * font_size

// After processing 20 glyphs in the current font:
if glyph_count >= 20 {
    // Collect all observed inter-glyph gaps in text space
    let gaps: Vec<f32> = observed_gaps();

    // Exclude outliers > 4× the current threshold (these are layout gaps, not word gaps)
    let filtered: Vec<f32> = gaps.into_iter()
        .filter(|g| *g <= 4.0 * threshold)
        .collect();

    // Compute median of filtered gaps
    let median_gap = median(&filtered);

    // Set new threshold to 1.5× the median
    threshold = 1.5 * median_gap;
}
```

### 4.2 Recalibration Rules

Per plan lines 1551-1553:

- **Per-font reset:** The 20-glyph recalibration window is reset on every font switch (`Tf` operator). Each new font starts fresh with zero samples and the fixed initial threshold.
- **Bootstrap behavior:** For the first 20 glyphs after a font switch (or at stream start), use the fixed initial threshold of `0.25 × font_size` with no recalibration.
- **Recalibration begins only after the 21st glyph** in the current font has been processed.

### 4.3 Why Median (Not Mean)?

The median is robust against outliers. A page may contain:
- A few very large gaps (column separators, tab stops)
- Many tight kerning pairs (negative or near-zero gaps)
- The true inter-word gap cluster (the target)

Using the mean would skew toward the large gaps, causing false negatives. Using the median finds the central tendency of the **typical** gap.

### 4.4 Outlier Exclusion

The `4.0 × threshold` filter prevents large layout gaps from contaminating the median. Without this filter:
- A single 300-point column gap would shift the threshold upward
- Smaller but legitimate word gaps (e.g., 8 points) would fall below the inflated threshold
- Spaces would be missed

The 4× multiplier is empirically chosen: it is large enough to exclude all layout gaps while small enough to retain the word-gap cluster.

### 4.5 Fallback Hierarchy

When the adaptive algorithm cannot be applied (insufficient glyphs, all gaps are outliers), fall back in this order:

1. **Font space glyph width:** If the font's `Widths` array contains a nonzero entry for character code 0x20, use `w_space * font_size / 1000`.
2. **CIDFont DW/W:** For CIDFonts, use `DW` (default width) or look up CID 0x0020 in the `W` array.
3. **FontDescriptor MissingWidth:** If present, use this as a lower bound for unknown glyphs.
4. **Half-em heuristic:** Use 500 glyph units → `0.5 * font_size`.
5. **Initial threshold:** Use `0.25 * font_size` as the final fallback.

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

## 11. Complete Algorithm — Pseudo-Code

The following pseudo-code specifies the complete word boundary reconstruction algorithm as implemented in Phase 3.2.

### 11.1 Data Structures

```
struct TextState {
    tc: f32,           // Character spacing (Tc operator)
    tw: f32,           // Word spacing (Tw operator)
    tz: f32,           // Horizontal scaling (Tz operator, default 100)
    font_size: f32,    // Font size in text space
    text_matrix: Matrix, // Current text matrix (Tm, Td, TD, T*)
    line_matrix: Matrix, // Current line matrix
    font_id: ObjectId, // Current font resource identifier
}

struct Glyph {
    codepoint: char,
    unicode_source: UnicodeSource,
    confidence: f32,
    bbox: [f32; 4],    // [x0, y0, x1, y1] in PDF user space
    font_name: Arc<str>,
    font_size: f32,
    rendering_mode: u8,
    fill_color: Color,
    is_word_boundary: bool,  // Synthetic space injected before this glyph
    mcid: Option<u32>,
}

struct WordBoundaryState {
    per_font_gap_samples: HashMap<ObjectId, Vec<f32>>, // font_id -> gaps
    per_font_glyph_count: HashMap<ObjectId, u32>,      // font_id -> count
    per_font_threshold: HashMap<ObjectId, f32>,        // font_id -> threshold
    last_glyph_end_x: Option<f32>,                     // Last glyph's end position
    last_glyph_baseline_y: Option<f32>,                // Last glyph's baseline
}
```

### 11.2 Main Loop

```
fn process_content_stream(operations: Vec<Op>) -> Vec<Glyph> {
    let mut text_state = TextState::new();
    let mut wb_state = WordBoundaryState::new();
    let mut output_glyphs = Vec::new();

    for op in operations {
        match op {
            Op::Tf(font_id, size) => {
                // Font switch — reset adaptive state
                text_state.font_id = font_id;
                text_state.font_size = size;
                wb_state.reset_font(font_id);
            }
            Op::Tc(tc) => {
                text_state.tc = tc;
            }
            Op::Tw(tw) => {
                text_state.tw = tw;
            }
            Op::Tz(tz) => {
                text_state.tz = tz;
            }
            Op::Tm(a, b, c, d, e, f) => {
                text_state.text_matrix = Matrix::new(a, b, c, d, e, f);
                text_state.line_matrix = text_state.text_matrix;
            }
            Op::Td(tx, ty) => {
                // Relative translation
                let delta = Matrix::translation(tx, ty);
                text_state.text_matrix = delta * text_state.text_matrix;
                text_state.line_matrix = delta * text_state.line_matrix;
            }
            Op::TD(tx, ty) => {
                // Equivalent to Td then TL = -ty
                text_state.tl = -ty;
                let delta = Matrix::translation(tx, ty);
                text_state.text_matrix = delta * text_state.text_matrix;
                text_state.line_matrix = delta * text_state.line_matrix;
            }
            Op::TStar => {
                // Move to start of next line
                let delta = Matrix::translation(0.0, -text_state.tl);
                text_state.text_matrix = delta * text_state.text_matrix;
                text_state.line_matrix = delta * text_state.line_matrix;
            }
            Op::Tj(string) => {
                let glyphs = decode_string(string, &text_state);
                for glyph in glyphs {
                    let (should_insert_space, gap_classification) =
                        detect_word_boundary(&glyph, &text_state, &mut wb_state);
                    if should_insert_space {
                        output_glyphs.push(make_synthetic_space(&text_state));
                    }
                    output_glyphs.push(glyph);
                    wb_state.record_glyph(&glyph, &text_state);
                }
            }
            Op::TJ(array) => {
                for element in array {
                    match element {
                        TJElement::String(s) => {
                            // Same as Tj
                            let glyphs = decode_string(s, &text_state);
                            for glyph in glyphs {
                                let (should_insert_space, gap_classification) =
                                    detect_word_boundary(&glyph, &text_state, &mut wb_state);
                                if should_insert_space {
                                    output_glyphs.push(make_synthetic_space(&text_state));
                                }
                                output_glyphs.push(glyph);
                                wb_state.record_glyph(&glyph, &text_state);
                            }
                        }
                        TJElement::Offset(offset) => {
                            // Numeric offset in TJ array: displacement = -offset/1000 * font_size * Tz/100
                            let displacement = -offset / 1000.0 * text_state.font_size * text_state.tz / 100.0;
                            // Large negative offsets (offset < 0) produce word gaps
                            if offset < -250 {  // Empirical threshold for TeX word gaps
                                output_glyphs.push(make_synthetic_space(&text_state));
                            }
                            // Advance text matrix
                            let advance = Matrix::translation(displacement, 0.0);
                            text_state.text_matrix = advance * text_state.text_matrix;
                        }
                    }
                }
            }
            Op::Quote(string) => {
                // Equivalent to T* then Tj
                // (handled by TStar then Tj case)
            }
            Op::DoubleQuote(aw, ac, string) => {
                // Set Tw = aw, Tc = ac, then Quote
                text_state.tw = aw;
                text_state.tc = ac;
                // (handled by Quote case)
            }
            _ => { /* Other operators ignored */ }
        }
    }

    output_glyphs
}
```

### 11.3 Word Boundary Detection

```
fn detect_word_boundary(
    glyph: &Glyph,
    text_state: &TextState,
    wb_state: &mut WordBoundaryState,
) -> (bool, GapClassification) {
    // First glyph: no boundary
    let Some(last_end_x) = wb_state.last_glyph_end_x else {
        return (false, GapClassification::FirstGlyph);
    };

    // Extract current glyph's start position in text space
    let current_start_x = glyph.bbox[0];  // x0 in user space
    let current_baseline_y = extract_baseline_y(&glyph.bbox);

    // Vertical gap detection (line break)
    let last_baseline_y = wb_state.last_glyph_baseline_y.unwrap();
    let delta_y = (current_baseline_y - last_baseline_y).abs();
    let line_height = text_state.font_size * 1.2;  // Default leading factor

    if delta_y > 0.5 * line_height {
        if delta_y > 1.5 * line_height {
            return (false, GapClassification::ParagraphBreak);
        } else {
            return (false, GapClassification::LineBreak);
        }
    }

    // Horizontal gap detection
    let gap = current_start_x - last_end_x;

    // Negative gap: backtrack (overlapping text, RTL, correction)
    if gap < 0.0 {
        return (false, GapClassification::Backtrack);
    }

    // Get threshold for current font
    let threshold = wb_state.get_or_compute_threshold(text_state.font_id, text_state.font_size);

    // Large gap: layout gap (column separator, tab stop)
    if gap > 2.0 * text_state.font_size {
        return (false, GapClassification::LayoutGap);
    }

    // Word gap
    if gap > threshold {
        return (true, GapClassification::WordGap);
    }

    (false, GapClassification::TightKerning)
}
```

### 11.4 Adaptive Threshold Computation

```
impl WordBoundaryState {
    fn get_or_compute_threshold(&mut self, font_id: ObjectId, font_size: f32) -> f32 {
        // Return cached threshold if available
        if let Some(&threshold) = self.per_font_threshold.get(&font_id) {
            return threshold;
        }

        // Check if we have enough samples
        let count = *self.per_font_glyph_count.get(&font_id).unwrap_or(&0);
        if count < 20 {
            // Bootstrap: use initial threshold
            return 0.25 * font_size;
        }

        // Compute median of observed gaps (excluding outliers)
        let gaps = self.per_font_gap_samples.get(&font_id).unwrap();
        let current_threshold = 0.25 * font_size;  // Initial for outlier filtering

        let filtered: Vec<f32> = gaps.iter()
            .copied()
            .filter(|g| *g <= 4.0 * current_threshold)
            .collect();

        let median_gap = median(&filtered);
        let new_threshold = 1.5 * median_gap;

        // Cache and return
        self.per_font_threshold.insert(font_id, new_threshold);
        new_threshold
    }

    fn record_glyph(&mut self, glyph: &Glyph, text_state: &TextState) {
        let font_id = text_state.font_id;

        // Increment glyph count
        *self.per_font_glyph_count.entry(font_id).or_insert(0) += 1;

        // Compute expected advance
        let glyph_width = lookup_glyph_width(glyph.codepoint, font_id);  // in 1/1000 em
        let tc = text_state.tc;
        let tw_if_space = if glyph.codepoint == '\u{0020}' { text_state.tw } else { 0.0 };
        let tz = text_state.tz;
        let font_size = text_state.font_size;

        let expected_advance = (glyph_width / 1000.0 * font_size + tc + tw_if_space) * tz / 100.0;

        // Compute actual gap (in text space)
        if let Some(last_end) = self.last_glyph_end_x {
            let actual_start = glyph.bbox[0];
            let gap = actual_start - last_end;

            // Record gap for adaptive threshold (only positive gaps)
            if gap > 0.0 {
                self.per_font_gap_samples.entry(font_id).or_insert_with(Vec::new).push(gap);
            }
        }

        // Update last glyph position
        self.last_glyph_end_x = Some(glyph.bbox[1]);  // x1
        self.last_glyph_baseline_y = Some(extract_baseline_y(&glyph.bbox));
    }

    fn reset_font(&mut self, font_id: ObjectId) {
        // Clear gap samples and threshold on font switch
        self.per_font_gap_samples.remove(&font_id);
        self.per_font_threshold.remove(&font_id);
        // Note: we do NOT reset glyph_count; we want to track total glyphs per font
        // for diagnostics. The threshold is recomputed from scratch for each font.
    }
}

fn median(values: &[f32]) -> f32 {
    if values.is_empty() {
        return 0.0;
    }
    let mut sorted = values.to_vec();
    sorted.sort_by(|a, b| a.partial_cmp(b).unwrap());
    let len = sorted.len();
    if len % 2 == 0 {
        (sorted[len / 2 - 1] + sorted[len / 2]) / 2.0
    } else {
        sorted[len / 2]
    }
}
```

### 11.5 Glyph Advance Calculation

```
fn compute_glyph_advance(
    glyph_code: u32,
    font: &Font,
    text_state: &TextState,
) -> f32 {
    // Look up glyph width from font metrics (in 1/1000 em units)
    let w_g = font.glyph_width(glyph_code).unwrap_or(500);  // Default half-em

    // Apply Tc (character spacing) — always added
    let tc = text_state.tc;

    // Apply Tw (word spacing) — only if glyph is SPACE
    let tw_if_space = if glyph_code == 0x20 {
        text_state.tw
    } else {
        0.0
    };

    // Apply Tz (horizontal scaling) — multiplicative
    let tz = text_state.tz;
    let font_size = text_state.font_size;

    // Formula from plan line 1547
    (w_g / 1000.0 * font_size + tc + tw_if_space) * tz / 100.0
}
```

---

## Implementation Notes for Rust

- Maintain a `TextState` struct that tracks `Tc`, `Tw`, `Tz`, `font_size`, `text_matrix`, and `line_matrix` as mutable graphics state, updated by the corresponding PDF operators.
- After each glyph is rendered, record `glyph_end_x` (device space) as `glyph_start_x + advance_device`.
- Before rendering the next glyph, compute `expected_x` from the full formula including `Tc` and `Tz`; compare actual x to `expected_x`; classify and emit gap events.
- For `TJ` arrays, iterate elements in order; accumulate string runs and emit gap events at each sign-significant numeric element before consuming the next string run.
- Store the font space width cache in a `HashMap<(ObjectId, OrderedFloat<f32>), f32>` keyed by font object ID and nominal font size to handle fonts used at multiple sizes.
- The adaptive histogram should bucket gaps into bins of width `0.01 * font_size` and perform a simple two-peak scan (find the global maximum, zero out ±3 bins, find the second maximum) to locate the space-width peak without a full GMM fit.

---

## 12. Edge Cases and Special Handling

### 12.1 Zero-Width Joiners (ZWJ, U+200D)

Zero-width joiners are used in complex script ligatures (e.g., Devanagari, Arabic) to indicate that adjacent characters should form a single ligature. When processing ZWJ:

- **Do NOT insert a space** before or after a ZWJ character.
- The ZWJ itself has zero width but affects glyph shaping; treat it as a combining mark.
- In the output, preserve the ZWJ as part of the Unicode stream so downstream renderers can apply proper shaping.
- The adaptive threshold algorithm should exclude ZWJ-adjacent gaps from the median computation to avoid skewing the threshold.

### 12.2 Combining Marks

Combining marks (U+0300–U+036F, etc.) modify the preceding base character and should not trigger word boundaries:

- **Do NOT insert a space** before a combining mark.
- The combining mark's bbox may overlap or be adjacent to the base character's bbox.
- In text space, the combining mark's advance width is typically zero; the gap detection should treat zero-width glyphs as transparent.
- When the gap from the base character to the combining mark exceeds the threshold, classify it as `GapClassification::CombiningMark` and continue without inserting a space.

### 12.3 CJK Text

Chinese, Japanese, and Korean scripts do not use space characters for word separation. The adaptive threshold algorithm does NOT apply to CJK text:

- **CJK detection:** Use Unicode script property (Script_Extensions) to identify CJK characters. If the majority of glyphs in a run are CJK, disable adaptive word boundary detection.
- **Word segmentation:** Proper CJK word segmentation requires dictionary-based or ML-based segmentation, which is outside the scope of Phase 4. For v1.0, emit CJK runs as continuous text without inferred spaces.
- **CJK punctuation:** Punctuation marks (U+3000 IDEOGRAPHIC SPACE, U+3001, U+3002, etc.) ARE word boundaries. Treat them as explicit space glyphs when they appear.
- **Mixed Latin/CJK:** When a run contains both Latin and CJK text, apply the appropriate algorithm per-script segment. Detect script transitions using the `Script` property and insert a boundary between segments.

### 12.4 Justified Text

Justified text has expanded inter-word gaps to fill the line width. This can skew the adaptive threshold:

- **Problem:** A justified page may have inter-word gaps of 15–20 points (vs. 8–10 points normal). If all gaps on a page are inflated, the median shifts upward, causing false negatives on non-justified pages.
- **Mitigation:** The adaptive algorithm's 20-glyph window limits the impact. Only a subset of glyphs (those within the first 20 of the font) are used for the initial median, reducing the influence of a single justified line.
- **Detection:** If the variance of observed gaps is high (stddev > 0.5 × median), the page may contain mixed justification. Consider recomputing the threshold per-line rather than per-font in this case.
- **Configuration:** Add a `justification_handling` enum to extractor config: `Auto` (default), `AssumeUnjustified` (use fixed 0.25 × font_size), `AssumeJustified` (use larger 0.5 × font_size).

### 12.5 Monospaced Fonts

Monospaced fonts (e.g., Courier, Consolas) have fixed advance widths for all glyphs. This affects gap detection:

- **Character-based spacing:** In monospaced fonts, every glyph occupies the same horizontal cell. Gaps are always multiples of the cell width.
- **Lower threshold:** Monospaced fonts often have wider-than-default spacing (e.g., 600 glyph units per character vs. 500 for proportional). The initial threshold of 0.25 × font_size may be too low; use 0.4 × font_size for known monospaced fonts.
- **Font detection:** Check the font's `FontFlags` (bit 0 is FixedPitch) or the `PostScriptName` for "Courier", "Consolas", "monospace".
- **Space width:** In monospaced fonts, the space glyph width is often the same as other characters (e.g., 600 units). Do NOT rely on the space glyph width as the canonical reference; use the adaptive median instead.

### 12.6 Diagonal Text and Rotated Pages

When the CTM includes rotation, text may be rendered at non-horizontal angles:

- **Comparison in text space:** Per plan line 1550, the gap comparison is performed in text space BEFORE CTM transformation. This makes the algorithm invariant to rotation.
- **Baseline extraction:** After CTM transformation, the glyph's baseline is no longer horizontal. To compute delta_y for line break detection, project the glyph positions onto the text-space y-axis (before CTM) rather than using device-space coordinates.
- **Rotated page detection:** If the CTM rotation is near 90°, 180°, or 270°, the page may be intentionally rotated. Emit a diagnostic but continue processing; the text-space comparison ensures correctness.

### 12.7 RTL (Right-to-Left) Text

Arabic, Hebrew, and other RTL scripts reverse the visual order relative to the code point order:

- **Logical vs. visual order:** The PDF content stream encodes glyphs in logical order (reading order), but the CTM may reverse the x-direction for visual rendering.
- **Gap detection:** The gap detection algorithm operates on logical order (the order of Tj/TJ operators). Do NOT reverse the order of glyphs before computing gaps.
- **Negative gaps:** In RTL text with bidi reordering, the next glyph in logical order may appear to the LEFT of the previous glyph in visual space, producing a negative gap. Classify this as `GapClassification::BidiReorder` and do NOT insert a space.
- **Bidi boundaries:** When the direction changes (LTR → RTL or vice versa), insert a boundary. Use the Unicode Bidi algorithm to detect direction changes.

### 12.8 Ligatures

Ligatures combine multiple characters into a single glyph (e.g., "fi", "fl", "ffi"):

- **No internal gaps:** A ligature glyph has no internal gaps; the adaptive threshold should not insert spaces within a ligature.
- **Ligature substitution:** During Phase 2.2 font resolution, ligature glyphs map to multiple Unicode code points (e.g., the "fi" ligature → 'f' + 'i'). Emit the decomposed characters as separate code points but do NOT insert a space between them.
- **Detection:** If a glyph's Unicode source is "ligature_decomposition", mark it as `is_ligature: true` in debug metadata. The gap detector should skip the threshold check for the first character in a ligature (only the last character advances the text position).

### 12.9 Soft Hyphens (SHY, U+00AD)

Soft hyphens indicate line break opportunities. They are rendered as hyphens when a line breaks at that position, but are invisible otherwise:

- **Invisible hyphens:** When a soft hyphen does NOT coincide with a line break, it has zero width and should be suppressed from output.
- **Visible hyphens:** When a line break occurs at a soft hyphen, emit '-' and insert a line break.
- **Detection:** The PDF content stream may encode soft hyphens as explicit SHY characters or as hyphen glyphs with conditional rendering. In the latter case, check the glyph's rendering mode and position relative to the line end.

### 12.10 Tab Stops and Tab Characters

PDF does not have a native "tab" character, but authors simulate tabs using large Td offsets or TJ array offsets:

- **Tab detection:** If a gap exceeds `2.0 × font_size` (per section 9), classify it as a layout gap rather than a word gap.
- **Tab character emission:** In single-column mode, emit a tab character (`\t`) for layout gaps. In multi-column mode, treat the gap as a column boundary and defer to the reading-order algorithm.
- **Configuration:** Add a `tab_character` enum to extractor config: `None` (default, treat as space), `Tab` (emit `\t`), `Space` (emit multiple spaces to approximate tab width).

---

## 13. Validation Methodology

### 13.1 Test Corpus Location

The authoritative test corpus for word boundary reconstruction validation is located at:

```
tests/fixtures/word-boundary-corpus/
```

This corpus contains PDFs with ground-truth tokenization labels, organized by category:

| Subdirectory | Description | Count |
|--------------|-------------|-------|
| `tex-academic/` | Academic papers from arXiv (TeX-generated) | 50 |
| `justified-text/` | Justified newspaper columns | 20 |
| `multi-column/` | Multi-column layouts with detected columns | 15 |
| `cjk-mixed/` | Chinese/Japanese/Korean with Latin script | 10 |
| `monospaced/` | Code listings and terminal output | 10 |
| `ligature-heavy/` | Documents with frequent ligatures (fi, fl, ffi) | 8 |
| `rtl-bidi/` | Arabic and Hebrew text with bidirectional content | 8 |
| `edge-cases/` | Synthetic tests for specific edge cases | 20 |

**Total:** 141 PDF documents with ground-truth labels.

### 13.2 Ground Truth Format

Each PDF in the corpus has a corresponding `.labels.jsonl` file with one line per page:

```jsonl
{"page": 1, "tokens": ["The", "quick", "brown", "fox", "jumps", "over", "the", "lazy", "dog."], "gaps": [0.12, 0.08, 0.11, 0.31, 0.09, 0.28, 0.07, 0.15, 0.0], "gap_units": "text_space_points"}
```

- **`tokens`**: The expected sequence of word tokens after space insertion.
- **`gaps`**: The inter-glyph gaps in text space (same length as `tokens` + number of explicit spaces in the PDF).
- **`gap_units`**: Always `"text_space_points"`; gaps are measured before CTM transformation.

### 13.3 Validation Metrics

Run the word boundary reconstructor on each PDF and compare the output tokens to the ground truth:

| Metric | Formula | Target |
|--------|---------|--------|
| **Token precision** | TP / (TP + FP) | ≥ 0.98 |
| **Token recall** | TP / (TP + FN) | ≥ 0.97 |
| **Boundary F1** | 2 × precision × recall / (precision + recall) | ≥ 0.975 |
| **Space error rate** | (FP + FN) / total_expected_spaces | ≤ 0.02 |

Where:
- **TP (true positive)**: Correctly inferred word boundary.
- **FP (false positive)**: Spurious space insertion (e.g., within a ligature).
- **FN (false negative)**: Missed word boundary (two words merged).

### 13.4 Per-Category Acceptance Criteria

| Category | Precision | Recall | Notes |
|----------|-----------|--------|-------|
| `tex-academic/` | ≥ 0.99 | ≥ 0.98 | High tolerance for tight kerning |
| `justified-text/` | ≥ 0.97 | ≥ 0.96 | Adaptive threshold must handle inflated gaps |
| `multi-column/` | ≥ 0.95 | ≥ 0.95 | Layout gaps must NOT be classified as word gaps |
| `cjk-mixed/` | ≥ 0.99 | ≥ 0.99 | CJK runs must NOT receive inferred spaces |
| `monospaced/` | ≥ 0.98 | ≥ 0.98 | Wider-than-default threshold |
| `ligature-heavy/` | ≥ 0.99 | ≥ 0.99 | No spaces within ligatures |
| `rtl-bidi/` | ≥ 0.97 | ≥ 0.96 | Negative gaps handled correctly |
| `edge-cases/` | ≥ 1.00 | ≥ 1.00 | Synthetic tests; must pass all |

### 13.5 Regression Test Suite

The `tests/word_boundary_test.rs` integration test loads the entire corpus and asserts the per-category acceptance criteria. Run with:

```bash
cargo test word_boundary --test word_boundary_test -- --nocapture
```

Expected output:
```
running 8 tests
test tex_academic ... ok
test justified_text ... ok
test multi_column ... ok
test cjk_mixed ... ok
test monospaced ... ok
test ligature_heavy ... ok
test rtl_bidi ... ok
test edge_cases ... ok

test result: ok. 8 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### 13.6 Continuous Validation

On every commit to `main`, the CI pipeline runs the full corpus test suite and publishes the metrics to the `word-boundary-metrics.json` artifact. Monitor this artifact for regressions:

```bash
# Download the latest metrics from CI
gh run download --name word-boundary-metrics
jq '.categories | map(select(.f1 < 0.975))' word-boundary-metrics.json
```

If any category falls below the F1 threshold, block the merge and require explicit sign-off with a justification (e.g., "category expanded with harder cases").

### 13.7 Manual Validation for New PDFs

For ad-hoc validation of a new PDF not in the corpus:

1. Run pdftract with `--debug-spans` to emit span metadata including `inferred: true` for spaces.
2. Compare the output text to visual inspection of the PDF in a viewer (e.g., Adobe Acrobat, pdf.js).
3. Count false positives (spurious spaces) and false negatives (missed spaces).
4. If the error rate exceeds 2%, consider adding the PDF to the corpus with hand-annotated labels.

---

## 14. Summary and Implementation Checklist

### 14.1 Core Algorithm Recap

The word boundary reconstruction algorithm is specified in the following order:

1. **Initial threshold:** `0.25 × font_size` for the first 20 glyphs per font.
2. **Adaptive adjustment:** After 20 glyphs, compute median inter-glyph gap (excluding outliers > 4× threshold), set threshold to `1.5 × median`.
3. **Tc/Tw/Tz corrections:** Apply character spacing (Tc) to every glyph, word spacing (Tw) only to U+0020, and horizontal scaling (Tz) multiplicatively.
4. **Gap comparison:** Compare in text space (before CTM transformation). If gap > threshold, insert synthetic space.
5. **Font switch reset:** Reset adaptive state on every `Tf` operator.

### 14.2 Implementation Checklist

For the Phase 3.2 implementation, verify the following items:

- [ ] **TextState struct** tracks `Tc`, `Tw`, `Tz`, `font_size`, `text_matrix`, `line_matrix`.
- [ ] **Per-font gap samples** stored in `HashMap<ObjectId, Vec<f32>>`.
- [ ] **Per-font glyph count** tracked for 20-glyph bootstrap window.
- [ ] **Initial threshold** of `0.25 × font_size` used for first 20 glyphs.
- [ ] **Median computation** with outlier exclusion (> 4× current threshold).
- [ ] **Threshold update** to `1.5 × median` after 20 glyphs.
- [ ] **Font switch reset** clears gap samples and threshold on `Tf` operator.
- [ ] **Tc/Tw/Tz formula** implemented as `(w_g/1000 × font_size + Tc + Tw_if_space) × Tz/100`.
- [ ] **Tw applied only to U+0020** (not to other whitespace).
- [ ] **Tz multiplicative** (not additive).
- [ ] **Gap comparison in text space** (before CTM transformation).
- [ ] **Vertical gap detection** for line breaks (`|Δy| > 0.5 × line_height`).
- [ ] **Layout gap classification** for gaps > `2.0 × font_size`.
- [ ] **Negative gap handling** for backtracks and bidi reordering.
- [ ] **TJ array offset processing** with sign-reversed displacement.
- [ ] **Synthetic space emission** tagged with `inferred: true`.
- [ ] **Per-page statistics** (`explicit_space_count`, `inferred_space_count`, `backtrack_event_count`, `layout_gap_count`).

### 14.3 Edge Cases Verified

- [ ] Zero-width joiners (ZWJ) do not trigger spaces.
- [ ] Combining marks do not trigger spaces.
- [ ] CJK text detection disables adaptive threshold.
- [ ] Justified text does not skew the median (20-glyph window limits impact).
- [ ] Monospaced fonts use wider initial threshold (0.4 × font_size).
- [ ] Rotated text: gap comparison in text space (invariant to CTM rotation).
- [ ] RTL text: negative gaps classified as bidi reordering.
- [ ] Ligatures: no spaces within decomposed ligature characters.
- [ ] Soft hyphens: suppressed when invisible, emitted as '-' when at line break.
- [ ] Tab stops: gaps > 2× font_size classified as layout gaps.

### 14.4 Validation Status

- [ ] Corpus location: `tests/fixtures/word-boundary-corpus/` (141 PDFs).
- [ ] Integration test: `tests/word_boundary_test.rs` passes all 8 categories.
- [ ] Token precision ≥ 0.98 across all categories.
- [ ] Token recall ≥ 0.97 across all categories.
- [ ] Boundary F1 ≥ 0.975 across all categories.
- [ ] Space error rate ≤ 0.02 across all categories.
- [ ] CI publishes metrics to `word-boundary-metrics.json` on every commit.

### 14.5 References

- Plan line 1529: Adaptive threshold + Tc/Tw/Tz reference.
- Plan line 1547: Word boundary threshold (adaptive) specification.
- Plan line 1550: Comparison space (text space before CTM).
- Plan line 1551: Recalibration window scope (reset on font switch).
- Plan line 1552: Bootstrap behavior (first 20 glyphs).
- Phase 3.1 (Graphics state machine): Source of Tc/Tw/Tz operators.
- Phase 4.5 (Reading order): Integration with layout gap classification.
- Phase 7.10 (Invoice profile): `table_detection: strict_borders` knob interaction.

---

**End of Document — Word Boundary Reconstruction v1.0**

