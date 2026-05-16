# Complex Layout Reading Order Reconstruction

## The Fundamental Problem

PDF content streams encode painting order, not reading order. When an authoring tool renders a two-column academic paper, it may emit all text runs in the left column first, then the right column — or it may interleave them by y-coordinate, painting each horizontal band across both columns before advancing downward. A newspaper layout with three columns and a pull quote may serialize its content in any order the compositor chose. The PDF specification makes no guarantee.

The consequence is direct: even when every glyph is decoded correctly with perfect Unicode mapping, assembling text runs in content-stream order produces output that is unreadable. A reader sees: the first paragraph of column A, then the first paragraph of column B, then the second paragraph of column A. Sentences from unrelated paragraphs abut each other. The information is present but the text is noise.

For mixed-layout pages — a full-width title, a two-column abstract and body, a full-width footnote zone — the problem compounds. No single sorting heuristic handles all three layout regions correctly. A naïve y-descending, x-ascending sort works for single-column documents but produces interleaved text for any multi-column region.

Reading order reconstruction must therefore operate as a distinct post-extraction phase that groups raw glyph streams into spatial regions and imposes a linguistically correct traversal order over those regions.

---

## Baseline Clustering into Lines

The first stage collapses individual glyph bounding boxes into text lines. A glyph box is characterized by its baseline y-coordinate, its left and right x-extent, and its advance width. Glyphs belong to the same line when their baseline y-coordinates fall within a tolerance window:

```
|baseline_a - baseline_b| <= line_height * 0.3
```

where `line_height` is estimated from the median cap-height of the font size in use. The 0.3 factor accommodates minor baseline drift from kerning and glyph descent variation without merging adjacent lines.

Superscripts and subscripts complicate this threshold. A superscript glyph sits above the baseline of its host span and has a reduced font size; it visually belongs to the line it annotates but will fail the baseline proximity test. Detection heuristic: if a glyph's font size is less than 0.7× the modal font size on the line and its baseline is within one line-height of the line, classify it as a super/subscript and attach it to the nearest enclosing span rather than starting a new line.

Rotated text (common in table headers and figure labels, encoded via the text matrix `Tm`) requires separate handling. Extract the rotation angle from the text matrix, cluster rotated glyphs by their rotated baseline, and treat each rotation group as an independent line set. Rotated lines are assigned to their spatial bounding box for zone assignment but are not merged into the main reading order flow; they are emitted as annotated spans within whichever zone contains them.

The output of line clustering is an ordered list of `TextLine` structs, each carrying a bounding box (union of all constituent glyph boxes), a dominant font size, a baseline y-coordinate, and an ordered list of `Span` entries sorted by x-ascending.

---

## Line Merging and Column Assignment via Gap Analysis

With lines established, column detection operates on their x-extents. For each line, record the set of horizontal gaps — intervals of x-space not covered by any glyph in that line. Aggregate gap histograms across a sliding window of consecutive lines (typically 5–10 lines). A gap position that recurs across multiple lines and exceeds `median_word_space × 3` is a column separator candidate.

`median_word_space` is estimated from the modal inter-glyph spacing within lines at the dominant font size. For 12pt Times New Roman this is approximately 3.5pt; the column-gap threshold becomes roughly 10.5pt, which cleanly separates two-column academic layouts (gap ≈ 18–24pt) from inter-word spaces.

Column count inference: sort candidate separator x-positions; the number of columns equals the number of separators plus one. Validate by checking that each column band contains at least `min_lines_per_column` (default: 3) lines. A single separator that only spans two or three lines is more likely a paragraph indent or a figure caption offset than a true column boundary.

Each line is assigned to a column index based on which column band its x-centroid falls into. Lines whose bounding boxes span multiple column bands (full-width lines) are assigned to a synthetic "full-width" zone, which is handled during layout merging.

---

## Recursive XY-Cut Algorithm

XY-cut is the classical divide-and-conquer approach to layout segmentation. Given a set of text bounding boxes occupying a rectangular page region:

1. Project all boxes onto the y-axis. Find the widest horizontal whitespace gap — a y-interval containing no box. This becomes the horizontal cut point, splitting the region into a top half and a bottom half.
2. Within each half, project onto the x-axis. Find the widest vertical whitespace gap. This becomes the vertical cut, splitting into left and right sub-regions.
3. Recurse on each sub-region until no further cuts are possible (the region contains a single column of text or a single text block).
4. The reading order is a depth-first left-to-right, top-to-bottom traversal of the resulting binary tree: for a horizontal cut, top before bottom; for a vertical cut, left before right.

The algorithm is elegant and handles the common cases — two-column academic papers, three-column newsletters — reliably. Its failure modes are:

- **Ambiguous cuts**: when a horizontal gap and a vertical gap have nearly equal widths, the cut order is uncertain. Heuristic: prefer the horizontal cut when gap sizes are within 20% of each other, since reading order is more frequently top-to-bottom than left-to-right.
- **Non-rectangular regions**: a figure that bleeds into the text column creates a non-rectangular text region that a rectangular cut cannot correctly isolate. Pre-detect figures by their bounding boxes and remove them from the text box set before applying XY-cut.
- **Close column gaps**: when the inter-column gap is narrow (common in three-column tabloid layouts), small descenders or accented capitals may bridge the gap, causing the algorithm to fail to find a clean cut. Apply a minimum gap threshold and fall back to Docstrum if no valid vertical cut is found.

---

## Docstrum Algorithm

Docstrum reconstructs reading order from nearest-neighbor relationships rather than whitespace gaps, making it more robust for skewed pages, curved text, and layouts with narrow inter-column margins.

For each text component (a glyph or short span), compute the k nearest neighbors by Euclidean centroid distance, typically k = 5. Classify each neighbor pair by the angle of the connecting vector:

- **Within-line pair**: the connecting vector is near-horizontal (angle within ±45° of 0°/180°) and the distance is less than `2 × char_width`. These pairs become edges in a within-line graph.
- **Between-line pair**: the vector is near-vertical (angle within ±45° of 90°/270°) and the distance is less than `2 × line_height`. These become between-line edges.

Connected components of within-line edges form text lines. Connected components of between-line edges, applied to those lines, form text blocks (paragraphs and columns).

The dominant within-line angle across all pairs gives the page skew; the dominant between-line distance gives the line spacing. Both are valuable for quality validation.

Docstrum's weakness is computational: O(n²) neighbor computation for n components, though a k-d tree reduces this to O(n log n) in practice. It also struggles when text density is very low (wide inter-word gaps that exceed the within-line distance threshold), which can fragment lines incorrectly.

---

## Smearing and Connected-Component Approaches

Projection-based smearing converts the 2D layout problem into 1D histogram analysis. Rasterize all text bounding boxes onto a 1D horizontal projection: for each y-row, count the number of covered pixels. Smooth with a Gaussian kernel (σ ≈ line_height / 4). Peaks correspond to text rows; valleys correspond to inter-line gaps. Apply a threshold to produce a binary row mask.

Similarly, project onto the vertical axis: each x-column counts occupied pixels. Peaks are text columns; valleys are column gaps or margins.

The RLSA (Run-Length Smoothing Algorithm) variant works in binary image space: apply a horizontal smearing operator that closes gaps shorter than a threshold C_h (typically 3× the average character width), then a vertical smearing operator with threshold C_v (typically 3× the line height). The resulting connected components are text blocks. RLSA is fast and works well for typewritten or OCR-processed documents.

Smearing approaches fail when column gaps are narrower than the smoothing kernel or when text blocks have irregular densities (justified text with variable inter-word spacing creates misleading projection valleys).

---

## Mixed-Layout Pages

A mixed-layout page contains horizontal bands of different column structures: a full-width title block, a two-column body, a full-width footer with page number. Correct reading order requires detecting these transitions.

Detection: scan the line set from top to bottom. For each horizontal band of lines (grouped by proximity in y), compute the x-spread. A band whose x-spread exceeds 85% of the page width is a full-width zone. A band whose lines cluster into distinct x-groups is a multi-column zone.

Column-count transitions (from full-width to two-column and back) define zone boundaries. The correct reading order is:

1. Full-width top zone (title, authors, abstract label) — top to bottom.
2. Multi-column body — column by column, left to right, reading each column fully before advancing to the next.
3. Full-width bottom zone (acknowledgements, references header if full-width) — top to bottom.

Figures that interrupt column flow (a figure spanning both columns mid-body) are detected by their bounding boxes crossing the column separator. They are extracted as `Figure` zones at their y-position in the document and inserted into the reading order at the point where the figure y-position occurs within the column being read.

---

## Sidebar and Inset Handling

A sidebar is a narrow text region adjacent to the main body that is not part of the primary reading flow. Detection criteria: bounding box width less than 40% of the page text width; x-position abutting the page margin; and either a visually distinct font family/size or a surrounding rule line (a `re` + `S` sequence in the content stream at the sidebar boundary coordinates).

Insets are text boxes whose bounding boxes overlap with body text — common in magazine layouts and promotional callouts. Detect by checking whether any text block's bounding box intersects the body text zone with an overlap ratio exceeding 10%.

Policy for both: extract sidebar and inset content after the main body text of the page. Tag output spans with `zone: "sidebar"` or `zone: "inset"` so downstream consumers can suppress or separately process them. Do not attempt to interleave sidebar content with body text at the word level — the reading orders are independent.

---

## Footnote Ordering

Footnotes occupy a horizontal band at the bottom of the page, below a separator rule (typically a short horizontal line element), in a font size smaller than the body (usually 0.7–0.8× body size). Detection: find horizontal rule elements in the lower 25% of the page text area; text blocks below the topmost such rule with font size below 0.85× modal body font size constitute the footnote zone.

Correct ordering: footnotes are emitted after all body text on that page. For multi-column pages, footnotes may span the full column width or be column-specific (column-specific footnotes appear in the same x-band as their host column). Order column-specific footnotes within their column's output; order full-width footnotes after all columns.

Footnote reference marks in the body text (superscript numerals or symbols) can be matched to the corresponding footnote by their textual label. Expose a `footnote_refs` map in page metadata linking body-text span positions to footnote block IDs for consumers that wish to inline them.

---

## Confidence Scoring and Fallback

Reading order reconstruction can fail silently — the output text is syntactically plausible but semantically wrong. Detecting this requires a language-model signal:

- **Character n-gram perplexity**: score the reconstructed text sequence against a character 4-gram model trained on natural language (English default; fall back to script-detected language model). Threshold: if perplexity exceeds 3× the baseline for clean prose, flag the reading order as suspect.
- **Word boundary coherence**: count the fraction of word boundaries that fall at natural break points (space, punctuation) versus mid-word. A high mid-word break rate indicates incorrect line concatenation or wrong reading order.

When confidence falls below threshold, apply the alternate algorithm: if XY-cut was primary, retry with Docstrum; if Docstrum was primary, retry with XY-cut. Accept whichever produces lower perplexity.

Expose in output metadata:

```rust
pub struct ReadingOrderMetadata {
    pub algorithm: ReadingOrderAlgorithm,  // XyCut | Docstrum | Smearing | NaturalOrder
    pub confidence: f32,                   // 0.0–1.0
    pub fallback_used: bool,
}
```

Provide a `natural_order` fallback mode that sorts text lines strictly by `(y_descending, x_ascending)` — deterministic, fast, correct for single-column documents, and predictably wrong for multi-column. Callers who need reproducible output over possibly incorrect output can opt into this mode explicitly.
