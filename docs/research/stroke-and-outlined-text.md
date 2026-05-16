# Stroke-based and Outlined Text in PDFs

## Overview

The `Tr` operator controls whether glyph outlines are filled, stroked, both, neither, or used to define a clip path. Mode 0 (fill) covers most PDF text, but the remaining modes appear regularly enough that pdftract must handle all eight. The critical insight is that rendering mode does not alter encoding or Unicode mapping — a character code maps to a codepoint through the same ToUnicode CMap, Differences array, or built-in encoding regardless of visual rendering. `Tr` affects paint, not identity.

---

## 1. Text Rendering Modes (Tr 0–7)

The PDF specification (ISO 32000-2 §9.3.6) defines eight text rendering modes indexed 0 through 7. The `Tr` operator sets the mode within the text state, which is part of the graphics state and subject to `q`/`Q` save-restore semantics.

| Mode | Name | Fill | Stroke | Clip added |
|------|------|------|--------|------------|
| 0 | Fill | yes | no | no |
| 1 | Stroke only | no | yes | no |
| 2 | Fill then stroke | yes | yes | no |
| 3 | Invisible | no | no | no |
| 4 | Fill + clip | yes | no | yes |
| 5 | Stroke + clip | no | yes | yes |
| 6 | Fill + stroke + clip | yes | yes | yes |
| 7 | Clip only | no | no | yes |

`Tr` defaults to 0 at the start of each page content stream and must be tracked in the graphics state stack. Every text-showing operator — `Tj`, `TJ`, `'`, `"` — emits glyphs under the active Tr. pdftract records the rendering mode into each extracted span when the operator is processed.

---

## 2. Rendering Mode 1 — Stroke Only

In mode 1, the renderer traces the glyph outline and strokes it without filling the interior, producing hollow or wireframe letterforms. This appears in display typography, decorative headings, and logo-embedded PDFs.

From an extraction standpoint, mode 1 text is fully accessible. Character codes are present in the content stream in exactly the same form as mode 0 text. The font's encoding and ToUnicode CMap apply identically, and advance widths drive glyph positioning unchanged. pdftract reads mode 1 spans through the same decoding path with no divergence.

One nuance is bounding box computation. A stroked glyph visually occupies more space than a filled one by half the stroke width on each side. If pdftract computes tight bounds for layout analysis, it should account for the stroke width when the rendered boundary matters for reading-order or column detection. For Unicode output this is irrelevant. Confidence for mode 1 is equivalent to mode 0 — the character data is unambiguously present and the rendering mode does not indicate OCR-derived or erroneous content.

---

## 3. Rendering Mode 2 — Fill Then Stroke

Mode 2 applies both a fill and a stroke pass to each glyph, producing bold or outlined letterforms with a colored border around a filled body. It is common in slide decks and documents where letter contrast over a complex background is needed.

Extraction from mode 2 is identical to mode 0. Both paint passes use the same character codes, advance widths, and Unicode mapping. Fill color, stroke color, and stroke width differ, but none affect text identity. One error to avoid is double-counting: the PDF content stream issues a single text-showing operator per glyph regardless of Tr — the rendering mode governs paint passes, not stream events. pdftract emits one span per text-showing operator with the rendering mode recorded in metadata.

---

## 4. Rendering Mode 3 — Invisible Text

Mode 3 applies no fill and no stroke. The glyph produces no visible marks, but the text engine still processes it — advance widths accumulate, the text matrix advances, spacing operators apply. This is the most consequential rendering mode for extraction.

### 4.1 The PDF/A Scan-plus-OCR Pattern

The dominant use of mode 3 is the searchable scan. A scanner captures a raster image of the page; OCR software recognizes the text and embeds the results as invisible glyphs positioned to overlay the image. The resulting PDF has a visible image layer and an invisible text layer carrying all machine-readable content. PDF/A-3b and PDF/UA both permit this pattern, and it is the standard output of commercial document scanning and archiving pipelines.

pdftract must extract mode 3 text without exception. Suppressing it would silently discard the only machine-readable content in a large fraction of real-world documents — archived records, legal filings, and government materials. The extraction path is identical to mode 0: character codes are read from the content stream, mapped through font encoding, and output as Unicode.

### 4.2 Confidence Scoring for Mode 3

pdftract applies a two-tier confidence model for mode 3 spans. When the font carries an explicit ToUnicode CMap, confidence is high — the OCR engine wrote recognized Unicode directly and the text is as reliable as the OCR pass itself. When the font lacks a ToUnicode CMap and relies on a Differences array, built-in encoding, or glyph-name inference, confidence is moderate — an indirect path seen in older scan workflows where the recovered Unicode may benefit from downstream validation.

### 4.3 Detecting Mode 3 Abuse

OCR errors propagate silently through invisible text layers. A visible image may show "exhibit" while the invisible layer encodes "exh1bit" because the OCR engine misread a numeral. pdftract cannot correct OCR errors at extraction time, but it should tag mode 3 spans with `ocr_layer: true` when a raster image covers the same page region. If the font's ToUnicode CMap is incomplete — glyph codes unmapped, or mapping to U+FFFD — pdftract adds `encoding_incomplete: true` and lowers confidence. These tags give downstream systems enough signal to flag suspect spans without requiring pdftract to adjudicate correctness.

---

## 5. Rendering Modes 4–7 — Clipping Combinations

Modes 4 through 7 each accumulate the glyph outline into the current clipping path in addition to their paint behavior. Mode 4 fills and clips; mode 5 strokes and clips; mode 6 fills, strokes, and clips; mode 7 clips only. In all cases the glyph outline is added to the clip path immediately after rendering, and subsequent graphics operations are clipped to the accumulated glyph shapes until the graphics state is restored with `Q`. This produces the typographic masking effect where imagery is visible only through letterforms.

For Unicode extraction, clip modes are handled identically to their non-clip counterparts: mode 4 as mode 0, mode 5 as mode 1, mode 6 as mode 2, mode 7 as mode 3. The clip path accumulation has no bearing on text content. What pdftract must track correctly is the graphics state mutation: the clip path built during mode 4–7 glyphs persists until `Q`, and if pdftract models clip paths for image region inference it must advance that model glyph by glyph. Failing to do so corrupts clip state for all subsequent operations on the page.

---

## 6. Rendering Mode and Font Type Interaction

Rendering mode does not interact with font type in any way that affects Unicode extraction. Type 1, TrueType, CFF, OpenType, CIDFont Type 0, CIDFont Type 2, and Type 3 fonts all expose character codes through the same encoding mechanisms regardless of Tr. The `Tr` operator controls paint operations applied to the glyph outline; it has no effect on how the outline is retrieved from the font program or how the character code is resolved to a name or codepoint.

pdftract's font decoding layer — resolving `Encoding`, `ToUnicode`, `Differences`, glyph names, and CID-to-Unicode mappings — is invoked identically for every rendering mode. There is no branch point on Tr in the decoding path.

---

## 7. Output Metadata and Confidence Policy

Every span produced by pdftract carries a `rendering_mode` integer whose value is the `Tr` active when the span's text-showing operator was processed. The field defaults to 0 when no explicit `Tr` operator has appeared. Downstream consumers should interpret it as follows:

- **Modes 0, 1, 2** — Text is visually rendered. Confidence is determined by encoding quality, not rendering mode.
- **Mode 3** — Text is invisible. If `ocr_layer: true`, treat as OCR-derived content. Confidence is high when the font has a complete ToUnicode CMap; moderate when it does not.
- **Modes 4, 5, 6** — Text is rendered and modifies the clip path. Confidence follows modes 0, 1, 2 respectively.
- **Mode 7** — Text is invisible and modifies the clip path. Confidence follows mode 3 rules.

pdftract must not suppress or omit text from any rendering mode in its primary output — filtering by rendering mode is a caller policy. The library surfaces all text present in the content stream with accurate metadata. The graphics state tracker must save and restore `Tr` on every `q`/`Q` pair, reset it to 0 at each page boundary, and propagate it into every emitted span. Tracking `Tr` as a first-class part of the graphics state is what makes correct extraction across all eight modes possible.
