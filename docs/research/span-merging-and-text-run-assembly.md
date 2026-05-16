# Span Merging, Text Run Assembly, and Glyph-to-Word-to-Line Pipeline

## The Extraction Atom: The Single Glyph

Every text extraction pipeline begins with the smallest meaningful unit: the individual glyph. When pdftract processes a content stream, each `Tj` or `TJ` operator produces one or more glyphs, and each glyph is a self-contained record: a character code resolved to a Unicode scalar, a bounding box in user space computed from the current text matrix and font metrics, a reference to the active font and its size, and the current rendering mode. This is the atom from which all higher-level structure is assembled.

Starting at the glyph level is the only semantically correct choice. PDF does not encode words, lines, or paragraphs — it encodes positioned drawing commands for individual glyphs. Any grouping imposed above the glyph is an inference made by the extractor. By preserving every glyph's position and font state independently, pdftract retains the information needed to correctly evaluate every subsequent merging decision. Discarding glyph-level detail earlier in the pipeline is irreversible and will cause incorrect span boundaries, particularly for documents with mixed fonts, tracking adjustments, or complex kerning.

The bounding box of a glyph is computed as: origin at the glyph's current text position, width equal to the glyph's advance width scaled by the font size and the current horizontal scaling factor, height derived from the font's ascender and descender metrics. These four values, combined with the font reference and rendering mode, constitute the complete glyph record.

## Intra-Operator Span Assembly

Within a single `Tj` operator, all glyphs share the same font, size, rendering mode, and text matrix state at the start of the operator. They are trivially concatenated into a span — the bounding box of the assembled span is the union of the individual glyph bounding boxes, and the text content is the concatenation of their decoded Unicode characters.

The `TJ` operator introduces kerning displacements between glyphs via numeric elements in its array. Most of these displacements are fine-grained tracking adjustments that do not represent word boundaries. pdftract treats a `TJ` kerning value as a word-break signal only when the displacement exceeds 0.25 times the current font size in user space (expressed in thousandths of a text space unit, so the threshold in text space is 250 units). Displacements below this threshold adjust glyph positions but do not split the span. Displacements at or above this threshold cause the current span to be closed and a new span to begin after the gap. This threshold was calibrated against a broad corpus of PDFs where inter-word spacing in `TJ` arrays consistently falls in the 200–600 unit range while intra-word kerning is typically below 100 units.

## Inter-Operator Merging

Consecutive `Tj` and `TJ` operators frequently represent a single continuous text run that was split across operators for reasons internal to the producing application — trailing kerning adjustments, color changes that were reverted, or simply the output of rich text compositors that emit one operator per styled run. pdftract merges consecutive operators into a single span when all of the following conditions hold:

- The font and font size are identical.
- The rendering mode is identical.
- The vertical deviation between the baseline of the new operator and the baseline of the current open span is less than 0.1 times the font size.
- The horizontal gap between the right edge of the last glyph in the current span and the left edge of the first glyph in the new operator is less than 0.5 times the space width for the active font (the advance width of the space glyph, or 0.25 em if the font has no space glyph).

Small `Td` adjustments between operators — the common idiom for micro-positioning in PDF generators — do not prevent merging as long as the resulting positions fall within these tolerances. The vertical tolerance accommodates sub-pixel rounding in the text matrix, and the horizontal tolerance accommodates the natural variation in inter-character spacing without admitting gaps large enough to represent inter-word spacing.

When a merge is performed, the bounding box of the receiving span is extended to cover the new glyphs, and the text content is concatenated. When the conditions are not met, the current span is closed and a new span is opened for the incoming operator.

## Line Formation

Spans are grouped into lines by baseline proximity. Two spans belong to the same line if their baselines differ by no more than 0.5 points in user space. This tolerance is tighter than the inter-line spacing of any typical document, which means it will not merge glyphs from adjacent lines while still accommodating the small floating-point rounding errors that accumulate during text matrix computation.

Superscripts and subscripts present a special case. A superscript glyph has a baseline elevated above the line by roughly 30–40% of the font size, and a subscript is depressed by a similar amount. pdftract detects these as glyphs whose baseline deviates from the dominant baseline of the current line cluster by more than 0.5 points but whose font size is detectably smaller than the line's primary font size (typically less than 75% of the dominant size). These glyphs are assigned to the nearest line cluster rather than being promoted to a new line, and they are tagged with a `superscript` or `subscript` flag on their span.

Within a line, spans are sorted by x-coordinate for left-to-right text. For RTL text, detected by the presence of Unicode bidirectional characters in the right-to-left categories (Arabic, Hebrew, and their associated punctuation ranges), spans are sorted by reverse x-coordinate. Mixed-direction lines preserve the visual reading order by applying the Unicode Bidirectional Algorithm at the span level after positional sorting.

## Word Boundary Injection

After spans within a line are assembled and sorted, pdftract injects word boundaries by scanning the inter-glyph gaps along the line. The challenge is that the "correct" gap threshold for word separation varies by font, point size, and document style. A fixed threshold produces over-segmentation in tightly spaced text and under-segmentation in loosely tracked text.

pdftract uses an adaptive threshold computed per line via a gap histogram. For each consecutive glyph pair within the line, pdftract records the horizontal gap between the right edge of the first glyph and the left edge of the second (after accounting for kerning displacements). These gaps are binned into a histogram. In documents with normal word spacing, this histogram is bimodal: a dense cluster of small intra-word gaps near zero (including negative values from kerning) and a second cluster of inter-word gaps centered around the space width of the font. The threshold is placed at the valley between these two peaks, found by scanning from the intra-word peak toward larger gap values until the bin count begins increasing again.

When the histogram is unimodal (e.g., in very short lines with one or two words), pdftract falls back to a fixed threshold of 0.3 times the space width of the dominant font. A space character is injected into the output at each gap that exceeds the threshold.

## Block Formation from Lines

Lines are grouped into text blocks — contiguous regions of related text such as paragraphs, headings, captions, or table cells. Block formation uses three signals:

1. **Inter-line spacing**: consecutive lines whose vertical gap falls within 20% of the median inter-line spacing for the local region are candidates for the same block. A gap more than 1.5 times the median spacing signals a block break.
2. **Left margin alignment**: lines within a block share a left margin within a tolerance of 2 points (accounting for first-line indentation, which is detected as a single-line offset and does not trigger a block break).
3. **Font size consistency**: a shift in the dominant font size between consecutive lines signals a block break and a potential heading boundary.

Each block is assigned a `kind` label derived from font characteristics: `heading` if the dominant font size exceeds 1.2 times the body text size for the page, `body` for standard paragraph text, `caption` for small-font text adjacent to figures, and `code` for monospaced font blocks.

## Column-Aware Line Grouping

On multi-column pages, naive line grouping by vertical proximity will incorrectly merge lines from separate columns into the same block. pdftract detects column boundaries before block formation by analyzing the x-coordinate distribution of all span left edges on the page. A significant gap in this distribution — a bin with zero or near-zero occupancy flanked by dense clusters on both sides — marks a column boundary. Lines whose x-ranges fall entirely within a single column band are constrained to merge only with other lines in the same column. Lines that span column boundaries (e.g., full-width headings) are identified by their x-extent and excluded from the column constraint.

## Mixed-Font Spans

When a single logical word is rendered with a font change mid-word — an italic letter in a roman word, a Greek character in a Latin text — pdftract must decide whether to split the word at the font boundary or preserve the word as a single unit with a mixed-font flag. Splitting at the font boundary produces incorrect tokenization that breaks downstream search and selection.

pdftract preserves word integrity by merging glyphs across font transitions within a word. A font transition within a word is detected when the horizontal gap between the last glyph of the current font and the first glyph of the incoming font is below the word-break threshold. The merged span carries a `flags` field with bits for `bold`, `italic`, and `mixed_font`. The `mixed_font` flag signals to consumers that the span's font reference is nominal (the font of the first glyph) and that per-glyph font information is available in the glyph record array if needed.

## Ligature Handling

Ligatures such as fi, fl, ffi, and ffl are encoded in many fonts as single glyph codes that map to multi-character Unicode sequences. The glyph occupies a bounding box that covers the combined extent of both constituent characters. pdftract expands ligatures to their Unicode equivalents in the text output but must distribute the bounding box across the expanded characters for character-granularity output.

The distribution is proportional: the ligature's bounding box width is divided among the constituent characters according to the nominal advance widths of those characters in the font's character metrics. If the font does not provide individual metrics for the components (common with symbol fonts), the box is divided equally. This approximation introduces a small positional error — typically less than 0.5 points — that is acceptable for word-level bbox queries but should be noted when sub-character precision is required.

## Output Granularity

A single extraction pass through the content stream produces the complete glyph record array. From this array, pdftract assembles word-granularity spans (one span per word, with the merged bounding box and concatenated text), character-granularity spans (one span per glyph, preserving individual bounding boxes), and paragraph-granularity spans (one span per block, with the block's bounding box and full text content) without re-parsing the PDF. The granularity is selected at query time by applying the appropriate merge level to the cached glyph array. This design means that a single page parse supports all output modes — character-level bbox queries for text selection, word-level spans for search, and paragraph-level spans for document structure analysis — without redundant work.
