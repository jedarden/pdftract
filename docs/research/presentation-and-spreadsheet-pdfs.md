# Presentation and Spreadsheet PDFs

## Overview

PDFs produced by presentation tools (PowerPoint, Keynote, Google Slides) and spreadsheet tools (Excel, Google Sheets, LibreOffice Calc) are structurally distinct from document PDFs. They share a common deficiency: neither was designed for linear reading. A presentation arranges text for visual impact across a large canvas; a spreadsheet arranges text for data inspection across a grid. Generic extraction — concatenating text in top-to-bottom, left-to-right scan order — produces unusable output for both types. This document describes the structural characteristics of each, detection heuristics, and extraction algorithms suited to each.

---

## 1. Presentation PDF Characteristics

Presentation PDFs exhibit a consistent set of structural traits regardless of authoring tool.

**Page geometry.** Slides are exported at fixed aspect ratios. The traditional 4:3 ratio maps to 10×7.5 inches at 72 dpi (720×540 pt). Widescreen 16:9 maps to 13.33×7.5 inches (960×540 pt) or 10×5.625 inches (720×405 pt) depending on the application. A page whose width/height ratio is within 2% of 4/3 (1.333) or 16/9 (1.777) is a strong presentation signal.

**Text density.** Slides carry very little text relative to page area. A typical body-text PDF contains 500–1500 characters per square inch of text area. A slide may contain 40–200 characters across the entire page. Characters-per-square-point (csp) below roughly 0.08 is a reliable low-density indicator; the exact threshold should be tuned against a corpus.

**Font sizes.** Title text is typically 28–54pt. Body bullets are 18–32pt. Captions and fine print may drop to 12pt but rarely lower. The median font size across all glyph runs on a slide is almost always above 18pt. A document with median font size above 18pt and low character density is almost certainly a presentation.

**Short, disconnected text runs.** Each text box is an independent content stream fragment. Unlike document paragraphs, slide text boxes are spatially isolated and not connected by semantic flow. A single page may contain 4–12 discrete text clusters with large whitespace gaps between them. Measuring the ratio of whitespace area to glyph-bounding-box area across the page gives a sparsity coefficient; values above 0.80 are characteristic of slides.

**Heavy XObject usage.** Slides embed many images, icons, and vector graphics as XObjects. A page with more than three Form or Image XObjects and fewer than 300 glyphs is likely a slide. Decorative background shapes — filled rectangles, gradient regions, logos — are rendered as graphics, not text.

**No reading flow.** Text on a slide is positioned for visual composition, not for sequential reading. There is no implicit reading order between text boxes. The spatial sequence in which text appears in the content stream (painting order) is irrelevant to semantic order.

---

## 2. Detecting Presentation PDFs

Detection operates at two levels: document metadata and per-page geometry.

**Producer metadata.** The `/Producer` entry in the document's `/Info` dictionary and the `pdf:Producer` / `xmp:CreatorTool` fields in XMP metadata identify the authoring application. Relevant substrings to match (case-insensitive):

- `"Microsoft PowerPoint"` — PowerPoint on Windows/macOS
- `"Keynote"` — Apple Keynote
- `"Google Slides"` — Google Slides via Chromium-based export
- `"LibreOffice Impress"` — LibreOffice Impress

A metadata match alone is sufficient to set `document_type = "presentation"` with high confidence, though page-level heuristics should still run to detect mixed-type documents.

**Page-level heuristics.** When metadata is absent or ambiguous, aggregate the following signals across all pages:

1. `aspect_ratio_score`: fraction of pages whose width/height ratio is within 0.03 of 4/3 or 16/9.
2. `low_density_score`: fraction of pages with character density below 0.08 csp.
3. `large_font_score`: fraction of pages with median glyph font size above 18pt.
4. `sparse_text_score`: fraction of pages with more than 4 discrete text clusters and fewer than 300 total glyphs.
5. `xobject_score`: fraction of pages with XObject count exceeding glyph run count.

Combine scores with weights (e.g., aspect ratio 0.35, large font 0.25, low density 0.20, sparse text 0.15, xobject 0.05). A weighted sum above 0.60 triggers presentation mode. Store the raw score as `detection_confidence` in output metadata.

---

## 3. Slide Structure Extraction

Once a page is identified as a slide, text runs are classified into roles: `title`, `subtitle`, `bullet`, `caption`, `decorative`.

**Title identification.** Among all text runs on the page, select the run with the largest font size. If multiple runs share the largest size, prefer the topmost (highest y-coordinate in PDF space, i.e., lowest y value if origin is bottom-left). The title run is almost always within the top 30% of the slide height. A run whose bounding box top exceeds 40% of page height is unlikely to be a title regardless of font size.

**Bullet detection.** Runs with font size 0.55–0.85× the title font size, located below the title box, are body bullet candidates. Within a bullet cluster, hierarchical levels are inferred from two signals:

- **X-indent offset**: each level adds a consistent horizontal indent, typically 12–24pt per level. Compute the leftmost x-coordinate of each run; runs that share a left-edge (within 2pt tolerance) belong to the same level. Runs indented further are child levels.
- **Font size reduction**: level 2 bullets are often 2–4pt smaller than level 1. Track font size alongside indent to resolve ambiguous cases.

Bullet markers (•, –, ▸, numerals followed by `.` or `)`) should be detected and stripped from the text content but recorded in the `bullet_marker` field to allow downstream reconstruction.

**Decorative text filtering.** Text that meets any of the following criteria is marked `decorative` and excluded from the logical output:

- Single Unicode characters in the Private Use Area or Wingdings/Symbol encoding (icon fonts).
- Font size below 8pt (watermarks, slide number labels in corners).
- Bounding box overlapping a large filled rectangle or image XObject (background text).
- Opacity below 0.30 as set by the graphics state `ca`/`CA` operators.

---

## 4. Text Box Reading Order for Slides

Slides have no canonical reading order. The content-stream painting order reflects z-ordering (background to foreground), not reading sequence. A viable reading-order heuristic:

1. Assign the title run rank 0.
2. For remaining non-decorative runs, compute a sort key: `sort_key = (y_band * 1000) + x_position`, where `y_band = floor(y_center / (page_height * 0.15))`. This groups runs into horizontal bands of 15% page height each, then sorts left-to-right within a band.
3. The title always leads; bands are ordered top-to-bottom.

When two text boxes overlap (their bounding rectangles intersect), prefer the one with larger font size as earlier in reading order. If font sizes match, prefer the one with greater area.

---

## 5. Speaker Notes

PowerPoint and Keynote support per-slide speaker notes. PDF export behavior varies:

**Notes Pages layout.** Some exporters include notes by appending a "Notes Page" after each slide — a second page (or second half of a landscape-split page) containing the slide thumbnail in the top half and notes text in the bottom half. Detection: if a page's height is approximately 2× the width (portrait, matching a 4:3 landscape slide stacked), the bottom half below the midpoint likely contains notes text.

**In-page notes region.** Some exporters render notes in a visually distinct region on the same page: smaller font (typically 10–12pt), wider margins, and often a thin horizontal rule separating it from the slide content. Detect by: font size drop below 14pt in a run cluster located in the bottom 35% of the page, with horizontal extent spanning more than 70% of page width (wider than typical slide content).

**Labeling.** Notes text must be extracted separately from slide content and tagged `role: "notes"` in the output. Notes should not participate in bullet hierarchy reconstruction or reading-order sorting.

---

## 6. Spreadsheet PDF Characteristics

Spreadsheet PDFs are visually dominated by a regular grid of cells. Characteristic traits:

**Dense, small text.** Cell content is typically 8–11pt. Character density is very high — often 0.4–1.2 csp, comparable to dense body text but distributed uniformly across the page rather than in paragraph blocks.

**Thin border lines.** Cell borders are hairline rules: 0.25–0.5pt stroke width, drawn as horizontal and vertical path segments forming a grid. These are far thinner than the ruled lines typical of document tables (usually 0.75–1.5pt). Stroke width below 0.5pt is a strong spreadsheet indicator.

**Cell alignment patterns.** Number columns are right-aligned; label columns are left-aligned; headers are often centered. This alignment is consistent within a column across all rows — a much stronger regularity than in document tables.

**Multi-sheet exports.** Excel and LibreOffice Calc export multiple sheets as page sequences. Each sheet's pages share a running header or footer containing the sheet name. Sheet boundaries are not otherwise marked in the PDF structure.

---

## 7. Spreadsheet Table Extraction

The grid-detection algorithm from general table extraction applies, but with calibration specific to spreadsheet hairlines.

**Grid construction.** Collect all horizontal and vertical line segments with stroke width ≤ 0.5pt. Cluster horizontals by y-coordinate (tolerance 1pt) and verticals by x-coordinate (tolerance 1pt). The resulting grid defines cell bounding boxes as the rectangles formed by adjacent horizontal and vertical pairs.

**Merged cell detection.** A merged cell is identified by the absence of an interior grid line where one would be expected. For a cell spanning columns c1 through c2, the vertical line at x-coordinate between c1 and c2 is missing in the row range occupied by the merged cell. Build the full grid skeleton and flag every missing interior segment; the corresponding cell region is a merge candidate, confirmed if a single text run's bounding box spans the merged region.

**Multi-line cell content.** A cell may contain line-wrapped text. Multiple glyph runs within the same cell bounding box, at different y-coordinates, belong to the same cell. Concatenate with a space or newline depending on whether the runs' baselines differ by more than 1.2× the font size (hard wrap) or less (soft wrap from kerning artifacts).

---

## 8. Sheet Boundaries in Multi-Sheet Exports

Running headers in spreadsheet PDFs typically contain the sheet name, the file name, or both. Detection algorithm:

1. Extract all text in the top 8% and bottom 8% of each page (header/footer zones).
2. Collect unique header strings across all pages; the string that changes between page groups is the sheet name candidate.
3. Group consecutive pages sharing an identical header string into a sheet run.
4. The first occurrence of a new header string marks a sheet boundary.

If no header is present, fall back to detecting a column-count change or a significant shift in the leftmost column x-position between consecutive pages.

---

## 9. Data Type Inference for Spreadsheet Cells

After cell text is extracted, infer the data type of each cell:

- **Integer**: matches `^-?\d{1,3}(,\d{3})*$` or `^-?\d+$` (locale-aware thousands separator).
- **Float**: matches `^-?\d+[.,]\d+$` after normalizing decimal separator.
- **Currency**: leading or trailing currency symbol (`$`, `€`, `£`, `¥`) with numeric body; strip symbol, parse as float.
- **Percentage**: trailing `%`; parse the numeric body and store as a float in [0, 1] (divide by 100).
- **Date**: attempt parsing against a priority list: ISO 8601 (`YYYY-MM-DD`), US (`M/D/YYYY`), EU (`D.M.YYYY`), short year variants. Store as an ISO 8601 string.
- **Boolean**: exact match against `TRUE`/`FALSE`, `Yes`/`No`, `✓`/`✗`, `1`/`0` (in cells where the column appears boolean-dominant).
- **Text**: fallback for anything not matched above.

Locale inference: if more than 30% of numeric cells use a comma as the decimal separator (values like `1.234,56`), set `locale = "eu"` and swap separator roles before parsing.

---

## 10. Output Representation

### Presentation Output

```
PresentationDocument {
  document_type: "presentation",
  detection_confidence: f32,        // 0.0–1.0
  producer: Option<String>,
  slides: Vec<Slide>,
}

Slide {
  slide_number: u32,
  title: Option<String>,
  subtitle: Option<String>,
  bullets: Vec<Bullet>,             // hierarchical
  body_text: Vec<String>,           // non-bullet body runs
  notes: Option<String>,
}

Bullet {
  level: u8,                        // 0 = top level
  marker: Option<String>,           // bullet character or numeral
  text: String,
  children: Vec<Bullet>,
}
```

### Spreadsheet Output

```
SpreadsheetDocument {
  document_type: "spreadsheet",
  detection_confidence: f32,
  producer: Option<String>,
  sheets: Vec<Sheet>,
}

Sheet {
  sheet_name: Option<String>,
  page_range: Range<u32>,           // 0-indexed page numbers
  table: Table,                     // reuses table schema from table-structure-reconstruction
}
```

The `Table` type is defined in `table-structure-reconstruction` and carries rows, cells, column spans, and merge annotations. Each `Cell` gains an additional `inferred_type` field (`CellType` enum: `Integer`, `Float`, `Currency`, `Percentage`, `Date`, `Boolean`, `Text`) populated by the data type inference pass.

The top-level `document_type` field uses the discriminant `"presentation" | "spreadsheet" | "document" | "form" | "mixed"`. A `"mixed"` classification applies when page-level heuristics disagree across more than 20% of pages — for example, a document that embeds a slide or a report that opens with a data table. In the mixed case, per-page classification is stored in a `page_classifications` array alongside the top-level type.
