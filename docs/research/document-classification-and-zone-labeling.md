# Document Classification and Zone Labeling

## Overview

After raw text extraction, each glyph or span has a position, font reference, and character content — but no semantic role. Zone labeling is the process of assigning a role to each text block: `body`, `heading`, `header`, `footer`, `footnote`, `caption`, `sidebar`, `marginalia`, or `page_number`. This pass runs after block assembly (grouping spans into lines and lines into paragraphs) but before reading-order resolution.

---

## 1. Why Zone Labeling Matters

Without zone labeling, extracted text is a raw positional dump. The damage is concrete:

- **Running headers interleaved with body paragraphs.** A header reading "Chapter 3: Results" appears between sentences because its y-coordinate places it between two body blocks on the same page.
- **Page numbers embedded mid-sentence.** A numeric "42" extracted in column order falls between the last word of one paragraph and the first word of the next.
- **Footnote markers disrupting prose flow.** Superscript `³` extracted inline pulls the following footnote text — located at the bottom of the page — into the paragraph body.
- **Sidebar text inserted at random positions.** A pull quote in the right margin, if read left-to-right by x-coordinate, bisects the main-column paragraph it is adjacent to.

The cost compounds downstream. Language models, search indexers, and screen readers all treat the extracted string as coherent prose. Injected non-body content corrupts sentence boundary detection, keyword density, and logical paragraph structure. Zone labeling is the gate that filters what reaches the output string.

---

## 2. Page Margin Heuristics

The simplest zone signals are geometric: headers and footers live at fixed vertical positions near the page boundary.

**Threshold definition.** Define `header_zone_max_y` as the y-coordinate below which a block must start to be considered a candidate header (measuring from the top of the page). A reliable default is 10–15% of page height. Similarly, `footer_zone_min_y` is the y-coordinate above which a block must end to be a footer candidate, measured from the bottom — again, 10–15%.

```
header_zone_max_y = page_height * 0.12
footer_zone_min_y = page_height - page_height * 0.12
```

Blocks whose bounding box falls entirely within these bands are header/footer candidates, not yet confirmed.

**Page number pattern detection.** Within the footer (or header) band, apply regex against the extracted text:

```
^\d+$                          // bare number: 42
^Page\s+\d+(\s+of\s+\d+)?$    // Page 3 of 12
^[ivxlcdmIVXLCDM]+$            // roman numerals: xiv
^[-–]\s*\d+\s*[-–]$           // em-dash framing: — 42 —
```

A block matching any of these within the margin band receives label `page_number` at high confidence (≥ 0.90).

**Stability filter.** A single page cannot confirm a header or footer — any text can appear near the top by chance. Apply the stability filter (described in section 5) before committing the label.

---

## 3. Font-Based Classification

Font metadata distinguishes heading hierarchy from body text, and body from ancillary text like captions and footnotes.

**Build a font inventory.** On first pass over the document, collect `(font_name, font_size, is_bold, is_italic)` tuples from every span. Normalize font sizes to points. Cluster sizes into bins using a simple histogram with a 0.5pt merge tolerance to collapse rounding artifacts. The bin with the highest total character count is the **dominant body size** — call it `body_pt`.

**Heading detection.** A block where all spans share a font size `> body_pt * 1.25` and `is_bold == true` is a strong heading candidate. Multiple heading levels are recoverable by ordered font-size clustering: the largest non-body size is `h1`, the next is `h2`, and so on, up to three levels before the signal becomes unreliable.

**Caption and footnote detection.** Blocks where the font size is `< body_pt * 0.85` are small-text candidates. Combine with position (bottom-of-page for footnotes, adjacent to a whitespace gap for captions) and font style (often italic for captions) to disambiguate.

**Dominance rule.** If a block mixes body-sized and heading-sized spans (e.g., a sentence with a bold lead word), classify by the dominant span — the one covering more than 60% of character width.

---

## 4. Positional Heuristics

**Centred text as heading signal.** Compute the horizontal midpoint of a block's bounding box. If it falls within 5% of page width from the page centre, and the block is a single line, raise the heading confidence. Centring alone is not sufficient — font size must also exceed body size.

**Indentation patterns.** Measure the left-edge x-coordinate of the first line vs. subsequent lines in a paragraph block. Standard body paragraphs have a consistent left margin with optional first-line indent (positive or negative). A hanging indent — where the first line starts further left than continuation lines — is a strong footnote or bibliography signal. A large positive indent on every line suggests a block quote.

**Column boundary detection.** Collect the left-edge x-coordinates of all body-candidate blocks on a page. Cluster them; two tight clusters indicate a two-column layout, defining column boundaries. Any block whose x-origin falls outside both columns and within the page margin is a marginalia candidate.

**Outer margin detection.** For a single-column document, define the body column as the region bounded by the median left and right x-extents of body blocks (±5% tolerance). Text that starts to the right of `body_right + page_width * 0.05` is marginalia.

---

## 5. Cross-Page Consistency

A text block that recurs at the same position across multiple pages is definitionally a running element — header or footer — regardless of whether it triggered the margin-band heuristic.

**Position fingerprint.** For each page, record `(y_normalized, height, width)` for every candidate block, where `y_normalized = block_top / page_height`. Two blocks across pages are positionally equivalent if their `y_normalized` values differ by less than 0.01 and their widths are within 5%.

**Sliding window.** Process pages in groups of five (or fewer at document boundaries). A block position that appears in at least four of five consecutive pages is a running element. Assign `header` or `footer` based on whether it sits in the top or bottom margin band; if it falls outside both bands but recurs consistently, assign the closer one.

**Recto/verso alternation.** Academic and book PDFs often alternate left-aligned headers on even pages (verso) with right-aligned headers on odd pages (recto). Detect this by checking whether positionally equivalent blocks alternate between page-parity groups. When alternation is confirmed, apply the header label to both positions. Text content may differ (e.g., chapter title vs. section title); only position need match.

**Recurring text fragments.** Normalize extracted text (trim whitespace, collapse runs) and hash each candidate block. A hash appearing on more than 50% of pages is a strong running-element signal even if position varies slightly (e.g., centred headers on different-width pages).

---

## 6. Footnote Detection

Footnote detection requires matching two artifacts: the inline marker and the footnote body.

**Inline markers.** During span assembly, track spans where `font_size < body_pt * 0.75` and the span baseline is raised above the line baseline by more than 2pt. These are superscript candidates. Extract the character: if it is a digit, letter, or standard footnote symbol (∗ † ‡ § ¶), record it as a marker with its position.

**Footnote body location.** On the same page, look for blocks in the lower region (below `page_height * 0.65`) that begin with a matching marker character, optionally followed by a period or space. The block's font size is typically `< body_pt * 0.85`.

**Separator rule.** Many PDF producers render a short horizontal rule (a thin rectangle path, typically 30–50% of column width, 0.5–1pt thick) immediately above the footnote area. When such a path is detected, all text blocks below it and above the footer band are footnote candidates, raising their confidence.

**Overflow footnotes.** A footnote body that begins on page N and continues on page N+1 has no marker on page N+1. Detect this by tracking whether the last footnote block on a page ends mid-sentence (no terminal punctuation followed by whitespace). If so, the first small-font block at the bottom of the next page inherits the footnote label.

---

## 7. Caption Detection

**Proximity to image placeholders.** PDF image XObjects (type `/XObject`, subtype `/Image`) and form XObjects used as figures occupy rectangular regions on the page. After extracting all XObject bounding boxes, identify text blocks whose bounding box top is within `body_pt * 3` of an XObject's bottom (for below-figure captions) or whose bottom is within the same threshold of an XObject's top (for above-figure captions).

**Prefix pattern.** Apply regex to the block's first token:

```
^(Figure|Fig\.|Table|Tbl\.|Scheme|Plate|Exhibit|Supplementary\s+Figure)\s+\d+
```

A prefix match raises caption confidence to ≥ 0.85 independent of position.

**Short block heuristic.** Captions are rarely longer than three lines. If a block adjacent to an image XObject contains more than three lines, treat only the first three as caption and reclassify the remainder as body.

---

## 8. Sidebar and Pull Quote Detection

**Narrow column detection.** A sidebar occupies a column significantly narrower than the main body column. If body column width is `W`, a block whose bounding box width is `< W * 0.45` and whose x-extent overlaps the body column by at least 10% is a sidebar candidate.

**Font differentiation.** Pull quotes are typically set in a larger or italic typeface to distinguish them visually from body text. A block that is bold or italic, centred or right-aligned, and horizontally overlaps the main column is a pull quote candidate. Assign label `sidebar` for narrow-column placement, or remain `body` with reduced confidence if the signal is ambiguous.

**Bounding box overlap logic.** Compute the intersection-over-union (IoU) of the candidate block and the main body column rectangle. IoU above 0.3 but below 0.9 indicates a partial overlap consistent with sidebar placement.

---

## 9. Confidence and Fallback

Each block receives a `zone_confidence: f32` in [0.0, 1.0] computed from a weighted sum of signals:

| Signal | Weight |
|---|---|
| Margin band (geometric) | 0.30 |
| Font size deviation from body | 0.25 |
| Cross-page recurrence | 0.25 |
| Regex / prefix pattern match | 0.15 |
| Positional heuristic (indent, centre) | 0.05 |

Weights are normalized per label. When no label achieves confidence ≥ 0.50, default to `body`. This is the safe fallback: false negatives (unlabeled headers/footers passed through as body) are preferable to false positives (body text discarded as a header).

Expose the confidence in output so callers can tune their own threshold. A caller building a full-text search index may accept all blocks regardless of zone. A caller building a clean prose renderer may filter to `zone == body && zone_confidence >= 0.70`.

---

## 10. Output Representation

Every block in the JSON output carries:

```json
{
  "text": "...",
  "zone": "body",
  "zone_confidence": 0.82,
  "bbox": { "x0": 72.0, "y0": 144.0, "x1": 540.0, "y1": 160.5 },
  "page": 3
}
```

Valid `zone` values:

| Value | Description |
|---|---|
| `body` | Main prose content |
| `heading` | Section or chapter heading |
| `header` | Running page header |
| `footer` | Running page footer |
| `footnote` | Footnote body text |
| `caption` | Figure, table, or scheme caption |
| `sidebar` | Sidebar or pull quote |
| `marginalia` | Margin annotation or note |
| `page_number` | Standalone page number |

The `zone` field is always present. `zone_confidence` is always a finite `f32`. Callers that want unfiltered text iterate all blocks; callers that want clean prose filter to `zone == "body"` or `zone in ["body", "heading"]`. Zone information is never used to modify `text` content — it is metadata only.
