# Scanned vs. Vector Page Classification

## Overview

Before `pdftract` can extract text from a PDF page, it must decide which pipeline to invoke. Routing a scanned page through vector extraction yields zero or garbled output. Routing a born-digital page through OCR wastes CPU and often produces lower-accuracy text than the embedded encoding. The classifier runs before any extraction work and produces a routing decision — one of `vector`, `ocr`, `hybrid`, or `assisted_ocr` — along with per-signal evidence that callers can inspect.

This document specifies the classification algorithm in full.

---

## 1. Page Type Taxonomy

Four categories require distinct extraction pipelines.

**Pure vector.** All text is encoded in content stream operators (`Tj`, `TJ`, `'`, `"`) with valid character-to-Unicode mappings. Fonts have `ToUnicode` CMaps or their encodings resolve fully through standard lookups. No full-page raster images are present. This is the fast path.

**Scanned image.** The page is a full-page raster image XObject. Text operators are absent or consist only of an invisible overlay (rendering mode `Tr 3`), which is the PDF/A pattern left by a prior OCR pass. Extraction requires rendering the page to a raster and running OCR.

**Hybrid.** Some regions contain valid vector text; other regions contain embedded image XObjects carrying text visible only as pixels. The classifier must produce a per-region routing map rather than a single page-level decision.

**Broken vector.** Text operators are present and font metrics are non-degenerate, but the font encoding is wrong or missing. Extracted character codes do not map to readable Unicode. The page looks like a vector page to a shallow scan but produces garbled or empty output. The classifier must detect the failure without running a full extraction pass, using density signals and partial decoding probes.

---

## 2. Fast Pre-Checks

Before committing to expensive image analysis or font decoding probes, evaluate three fast signals in order. Any definitive match short-circuits the rest of the classifier.

**No text operators.** Parse the content stream and count `BT`/`ET` block pairs that contain at least one text-showing operator (`Tj`, `TJ`, `'`, `"`). If no `BT` blocks exist, or every `BT`/`ET` pair is empty, the page has no vector text. With an image XObject present, classify as `scanned`; with neither, classify as `empty`.

**Invisible text only.** Parse every `Tr` (text rendering mode) operator within `BT`/`ET` blocks. If all text operators are guarded by `Tr 3` (invisible mode) and at least one full-page image XObject is present, the page is a scanned image with an OCR overlay. Classify as `scanned` and mark `has_ocr_layer: true`. This overlay should be used as a fallback hint in the `assisted_ocr` pipeline, not treated as authoritative.

**High-confidence vector.** If extracted character count exceeds the density floor (section 4), no image XObject covers more than 30% of page area, and character validity rate (section 5) exceeds 0.85, classify immediately as `vector`. This branch exits the classifier early for the majority of born-digital pages.

---

## 3. Image Coverage Analysis

For pages that do not exit via fast pre-checks, compute the fraction of page area covered by image XObjects.

For each `Do` operator that references an image XObject, retrieve the Current Transformation Matrix (CTM) at the point of invocation. An image XObject is defined in its own coordinate space as a unit square `[0,0,1,1]`. Transform this unit square through the CTM to obtain the rendered bounding box in page coordinates. Use the four corners of the transformed parallelogram to compute an axis-aligned bounding box (AABB). Sum the AABB areas across all image XObjects (handling overlaps by clipping against the union).

```
image_coverage_fraction = clipped_image_area / page_area
```

If `image_coverage_fraction > 0.80`, the page is predominantly image-based. Combine with text operator signals:

- Coverage > 0.80, no text operators → `scanned`
- Coverage > 0.80, invisible text only → `scanned` (with OCR layer)
- Coverage > 0.80, valid text operators → `hybrid` candidate, proceed to region analysis (section 7)

To distinguish full-page background images from content images, evaluate position and aspect ratio. A full-page background image has an AABB whose top-left corner is within 5% of page width/height of the page origin and whose dimensions are within 10% of the page dimensions. Images that do not match this geometry are content images and do not contribute to the background coverage fraction used for routing.

---

## 4. Text Operator Density

Count the number of character codes emitted by all text-showing operators on the page. Normalize by an expected character count derived from page dimensions and a reference text density.

For a standard A4 page (595 × 842 points) at 10pt font size with normal line spacing and margins, expected character count is approximately 3000–4000. Scale linearly for non-standard page sizes. Compute `density_ratio = extracted_char_count / expected_char_count(page_area)`.

A `density_ratio < 0.05` despite the presence of text operators is diagnostic of broken vector: the font is present but almost no characters decode. A ratio in `[0.05, 0.30)` is ambiguous — sparse typesetting or partial encoding failure. A ratio above 0.30 is consistent with valid vector text.

This check must operate on raw character codes from the content stream before ToUnicode mapping, measuring how many characters the font machinery attempts to emit rather than how many produce valid Unicode. Validity is assessed separately in section 5.

---

## 5. Character Validity Rate

After decoding character codes through the font's encoding chain (Encoding dictionary, ToUnicode CMap, CIDToGIDMap), apply the readability checks defined in `text-readability-validation.md` to compute a per-page validity rate.

The three primary signals:

**Replacement character density.** `replacement_ratio = fffd_count / total_codepoints > 0.10` flags that font as broken.

**PUA density.** `pua_ratio > 0.40` over U+E000–U+F8FF and U+F0000–U+FFFFF indicates a missing ToUnicode map.

**Symbol font bleed-through.** Codepoints mapping to Zapf Dingbats or Symbol block ranges appear when a non-text font is decoded using a text font's encoding.

Compute these per-font and aggregate to a page-level `character_validity_rate`:

```
character_validity_rate = valid_chars / total_chars
```

where `valid_chars` excludes U+FFFD, PUA codepoints above the 5% tolerance, and control characters outside U+0009 and U+000A.

If `character_validity_rate < 0.70`, classify the page as `broken_vector` and route to OCR fallback. If `character_validity_rate` falls in `[0.70, 0.85)`, route to `assisted_ocr`: use the partial vector output as position hints for OCR alignment but do not trust the decoded text as final output.

---

## 6. Glyph Bounding Box Plausibility

For each extracted glyph, compute its bounding box in page coordinates by applying the text matrix, text line matrix, and CTM. Plausibility bounds:

- **Width:** `[0.01 × font_size, 2.0 × font_size]`. Glyphs narrower than 1% of the font size are likely zero-width artifacts from a dummy text layer. Glyphs wider than twice the font size are likely the result of a corrupt text matrix.
- **Height:** `[0.3 × font_size, 3.0 × font_size]`. Heights below 30% suggest degenerate scaling; heights above 300% suggest an incorrect CTM.

Compute `implausible_bbox_fraction = implausible_glyph_count / total_glyph_count`. If this exceeds 0.20, the font matrix or CTM is corrupted. Route to OCR fallback.

Additionally, for consecutive glyphs on the same text line, compute the intersection-over-union (IoU) of their bounding boxes. If more than 50% of adjacent pairs have IoU > 0.50, glyphs are stacked at a single position — a common pattern in dummy text layers from low-quality OCR tools. Flag as `broken_vector` and suppress the text layer.

---

## 7. Region-Level Hybrid Detection

When `image_coverage_fraction` is in `[0.20, 0.80)` and text operators are present, the page requires per-region analysis.

Partition the page into rectangular regions using the AABBs of image XObjects as boundaries. For each region:

1. Collect all text operator glyph bboxes whose center falls within the region.
2. If glyph count is zero, the region is image-only → assign `ocr`.
3. If glyph count is non-zero, apply character validity rate and bbox plausibility checks to glyphs within the region. If `character_validity_rate >= 0.85` and `implausible_bbox_fraction < 0.20`, assign `vector`. Otherwise assign `ocr`.

The result is a region routing map:

```rust
pub struct RegionRoute {
    pub bbox: [f32; 4],   // [x0, y0, x1, y1] in page coordinates
    pub method: ExtractionMethod,
}
```

The extraction engine uses this map to run vector extraction over `vector` regions and rasterize + OCR only the image-covered regions, avoiding a full-page OCR pass when much of the page is efficiently extractable via vector.

---

## 8. Confidence Scoring and Routing Decision

Each classification signal contributes evidence toward four confidence scores: `vector_confidence`, `scan_confidence`, `hybrid_confidence`, and `broken_vector_confidence`. Signals are weighted additively:

| Condition | Route |
|---|---|
| `vector_confidence > 0.80` | `vector` |
| `scan_confidence > 0.80` and `character_validity_rate < 0.20` | `ocr` |
| `broken_vector_confidence > 0.60` | `ocr` (fallback) |
| Both vector and scan confidence moderate | `hybrid` |
| `character_validity_rate ∈ [0.70, 0.85)` | `assisted_ocr` |

Signal weights (normalized to sum 1.0 per dimension):

- No text operators: +0.9 scan
- Invisible text only (`Tr 3`): +0.7 scan, +0.3 scan OCR-layer
- `image_coverage_fraction > 0.80`: +0.6 scan
- `density_ratio > 0.30` and `character_validity_rate > 0.85`: +0.8 vector
- `character_validity_rate < 0.70`: +0.7 broken-vector
- `implausible_bbox_fraction > 0.20`: +0.5 broken-vector
- Adjacent glyph IoU > 0.50 on majority: +0.6 broken-vector, +0.4 scan

---

## 9. Cost-Aware Routing

OCR is expensive — a Tesseract pass on a 300 DPI A4 raster typically takes 1–3 seconds. Cost gating rules:

If `vector_confidence > 0.80`, skip OCR even on hybrid pages for vector regions. Invoke OCR only for regions where vector extraction failed or produced low-confidence output. For `assisted_ocr` pages (`character_validity_rate ∈ [0.70, 0.85)`), pass partial vector output as bounding-box hints to the OCR engine to reduce search space.

The `ocr_threshold` configuration parameter sets the minimum `character_validity_rate` below which OCR is attempted. Default is `0.85`; archival document pipelines with predictably broken encodings should lower it to `0.60` or below.

```toml
[extraction]
ocr_threshold = 0.85
```

---

## 10. Output Metadata

The classifier emits structured metadata for every page, available in the extraction output regardless of which pipeline ran:

```rust
pub struct PageClassificationMeta {
    pub extraction_method: ExtractionMethod,  // vector | ocr | hybrid | assisted_ocr
    pub vector_confidence: f32,
    pub ocr_confidence: f32,
    pub classification_signals: Vec<ClassificationSignal>,
    pub image_coverage_fraction: f32,
    pub text_operator_count: u32,
    pub character_validity_rate: f32,
    pub region_routes: Option<Vec<RegionRoute>>,  // populated for hybrid pages
}

pub enum ClassificationSignal {
    NoTextOperators,
    InvisibleTextOnly,
    HighImageCoverage,
    LowDensityRatio { density_ratio: f32 },
    LowCharacterValidity { rate: f32 },
    ImplausibleGlyphBboxes { fraction: f32 },
    AdjacentGlyphOverlap { fraction: f32 },
    FullPageBackgroundImage,
    OcrLayerDetected,
}
```

`classification_signals` is an ordered array of the signals that fired, enabling callers to audit classification decisions, build quality dashboards, and tune the `ocr_threshold` based on observed validity rates across their document corpus.

---

## Implementation Notes

The classifier runs as a single-pass content stream parse. Image XObject AABBs are computed during the same parse as text operator collection, sharing the CTM stack. No font decoding is required for fast pre-checks or image coverage stages; font probes for character validity are lazy and terminate early once sufficient evidence accumulates. The region routing map for hybrid pages is computed in a second pass over the accumulated evidence, keeping the first pass allocation-free beyond the glyph accumulator buffer.
