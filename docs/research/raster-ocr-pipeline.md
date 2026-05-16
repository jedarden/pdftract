# Raster OCR Pipeline for PDF Text Recovery

## Overview

Not all PDF pages carry extractable vector text. Scanned documents, image-only PDFs, and PDFs with corrupt or dummy text layers require OCR to recover readable content. This document describes the full pipeline from trigger detection through output alignment, as it applies to `pdftract`.

---

## 1. When to Trigger OCR

### Detection Signals

Four independent signals indicate that a page requires OCR:

**No text operators.** A content stream parse that yields zero `Tj`, `TJ`, `'`, `"`, or `Do` (form XObject with text) operators is the strongest indicator. If the page contains only image XObjects and path operators, OCR is mandatory.

**Suspiciously low character density.** Compute the ratio of character glyph bounding box area to page area. Body text pages should yield densities above roughly 0.03 (3%). A page with a large raster image and a handful of stray characters (OCR artifacts from a prior tool, or a page number alone) falls below this threshold and warrants re-examination.

**Bounding box misalignment (fake text layer).** Some scanned PDFs carry an invisible text layer placed by a prior OCR pass. Validate each character's glyph bounding box against the underlying raster. Render the page and sample pixel intensity in the region each glyph should occupy. If the dominant pixel value is white (background) in >80% of sampled glyphs, the text layer is synthetic and untrustworthy. The character positions may also be zero-width or all positioned at a single coordinate, which is another reliable indicator of a dummy layer.

**Below-threshold extraction confidence.** If the PDF uses a Type3 or CIDFont with missing ToUnicode entries, character codes cannot be mapped reliably. Track the fraction of unmapped characters per page; above 25% missing, confidence in vector extraction is too low and OCR should take over.

### Decision Algorithm

Use a **vector-first with OCR fallback** strategy. Attempt vector extraction; if any of the above signals fire, queue the page for OCR. Do not run both in parallel by default — OCR is expensive and the result comparison logic is non-trivial. The parallel-and-compare approach is justified only when assisted OCR (section 4) is in use and you need to resolve conflicts between the two sources. In that case, run the two passes concurrently and arbitrate at merge time.

---

## 2. Image Preprocessing Pipeline

Raw page rasters fed directly into Tesseract produce poor results. A deterministic preprocessing chain is essential.

### Rasterization DPI

Render the PDF page to a raster using a PDF rendering backend (e.g., `pdfium-render` or `mupdf` bindings). Use **300 DPI minimum** for standard body text. For pages with font sizes below 8pt or fine print, use **400 DPI**. Higher DPI yields better Tesseract accuracy up to roughly 600 DPI; beyond that, gains plateau and memory cost dominates.

Store the raster as a grayscale or 8-bit image. Color channels add no accuracy benefit for Latin-script OCR and increase memory pressure.

### Deskewing

Scanned pages are rarely axis-aligned. Two reliable methods:

- **Hough line transform on text baselines.** Apply a Canny edge detector, then accumulate Hough votes for near-horizontal lines (angles within ±10° of horizontal). The mode angle of the dominant cluster is the skew angle. Rotate the image by the negative of that angle before OCR.
- **Projection profile maximization.** For each candidate rotation angle in a sweep (e.g., -10° to +10° in 0.1° steps), compute the horizontal projection profile (sum of white pixels per row). Text baselines produce sharp peaks; maximize the variance of this profile across candidate angles to find true horizontal alignment.

The projection profile method is more robust for low-resolution or lightly printed pages; the Hough approach is faster for clean scans.

### Binarization

Convert grayscale to binary (black text on white background):

- **Otsu thresholding** works well for uniformly lit pages with bimodal intensity histograms. It minimizes intra-class variance and requires no tuning.
- **Sauvola local adaptive thresholding** is essential for pages with uneven illumination (e.g., curved book spines, shadow gradients). It computes a per-pixel threshold from a local window mean and standard deviation: `T(x,y) = mean * (1 + k * (std/R - 1))` where `k ≈ 0.5` and `R = 128`. Window size of 15–31 pixels at 300 DPI is typical.

Prefer Sauvola for physical scans; prefer Otsu for digital-origin documents printed and re-scanned at a consistent exposure.

### Denoising and Morphological Cleanup

After binarization:
- Apply a **median filter** (3×3 or 5×5 kernel) to suppress salt-and-pepper noise without blurring character strokes.
- Apply **morphological opening** (erosion then dilation) with a 1×1 structuring element to remove isolated single-pixel noise blobs.
- Do not apply closing (dilation then erosion) before OCR — it merges character strokes and degrades accuracy.

### Contrast Normalization

Before binarization, stretch the grayscale histogram so that the 2nd percentile maps to 0 and the 98th percentile maps to 255. This compensates for faded or overexposed scans. Apply this before Sauvola to ensure the local statistics are computed on a well-conditioned input.

---

## 3. Tesseract Integration

### Engine and API Mode

Tesseract exposes three OEM (OCR Engine Mode) values:
- `OEM_TESSERACT_ONLY` (0): legacy cube engine; fast, lower accuracy.
- `OEM_LSTM_ONLY` (1): LSTM-based neural engine; best accuracy for most scripts.
- `OEM_TESSERACT_LSTM_COMBINED` (2): runs both and combines; marginally better, significantly slower.

Use `OEM_LSTM_ONLY` (1) as the default. Fall back to `OEM_TESSERACT_LSTM_COMBINED` only if LSTM alone produces below-threshold confidence on a page.

### Page Segmentation Mode

PSM selection critically affects accuracy:
- `PSM_AUTO` (3): default; suitable for full pages with mixed content.
- `PSM_SINGLE_BLOCK` (6): a single uniform block of text; use when the page is a known body-text region.
- `PSM_SINGLE_LINE` (7): use when processing a single text line extracted from a larger region.
- `PSM_SINGLE_COLUMN` (4): multi-size text in a single column; useful for narrow document columns.
- `PSM_SPARSE_TEXT` (11): page with scattered text, no assumed reading order; use for form fields or tables with isolated cells.
- `PSM_VERTICAL` (5): vertical CJK text (see section 6).

For full-page OCR, start with `PSM_AUTO`. For region-level OCR (where bounding boxes are already known), use `PSM_SINGLE_BLOCK` or `PSM_SINGLE_LINE` depending on region height.

### `tesseract-rs` Crate Interface

The `tesseract` crate (wrapping `leptonica` + `libtesseract`) exposes a Rust-safe interface. Key initialization:

```rust
let mut api = tesseract::Tesseract::new(Some("/usr/share/tessdata"), Some("eng"))
    .set_page_seg_mode(tesseract::PageSegMode::PsmAuto)
    .set_variable("tessedit_char_whitelist", "")?;  // empty = all characters
```

Pass a pre-binarized image rather than letting Tesseract binarize internally. Tesseract's internal Otsu implementation ignores Sauvola-style adaptation, which degrades accuracy on uneven scans. Use `SetImage` with a Leptonica `PIX*` allocated from your preprocessed raster.

### Language Packs and Confidence

Confidence scores are available at two granularities:
- `GetMeanTextConf()`: page-level mean confidence, 0–100.
- Per-word `Confidence()` from the `ResultIterator` at `RIL_WORD` level.

A page-level confidence below 60 signals that OCR failed and a fallback (different preprocessing, different PSM, or marking the page as unextractable) is needed. Per-word confidence is used to tag individual spans (section 9).

---

## 4. Assisted OCR (Vector Hints)

When vector text is partially valid (low-confidence but spatially correct), use it to guide OCR rather than discarding it. Two mechanisms:

**`SetRectangle` per known word region.** If vector extraction produced bounding boxes for individual words, crop the raster to each word's bounding box (with a small margin, e.g., 5px), set PSM to `PSM_SINGLE_LINE`, and run Tesseract on each crop independently. This restricts the LSTM's attention to a known region and avoids segmentation errors on surrounding noise.

**HOCR alignment.** Run full-page OCR with HOCR output, then match HOCR word boxes against vector word boxes using IoU (intersection over union). Where IoU > 0.7 and vector confidence is above threshold, prefer the vector text (it carries the correct encoding from the font). Where IoU > 0.7 but vector confidence is below threshold, prefer the OCR text. Unmatched OCR words (no corresponding vector box) are accepted as new content.

Conflict resolution rule: when vector and OCR produce different strings for the same box, prefer OCR if vector confidence < 0.4, prefer vector if OCR word confidence < 50, and flag the span as ambiguous otherwise.

---

## 5. HOCR Output and Coordinate Alignment

Tesseract's HOCR output is an HTML document with a hierarchy of classed elements: `ocr_page`, `ocr_carea` (content area/block), `ocr_par`, `ocr_line`, `ocr_word`. Each element's `title` attribute contains a `bbox x0 y0 x1 y1` value in raster pixel coordinates (origin top-left).

To map back to PDF coordinate space:
1. Divide pixel coordinates by the raster DPI to get inches.
2. Multiply by 72 to get PDF user units (points).
3. Flip the Y axis: `pdf_y = page_height_pts - (pixel_y / dpi * 72)`.

Parse the HOCR with a SAX or DOM HTML parser. Extract `ocr_word` elements, reading `bbox`, `x_wconf` (confidence), and text content. Map each to a `Span` in the same schema used for vector-extracted content, setting the `source` and `confidence` fields accordingly.

Group `ocr_word` spans into lines using the `ocr_line` parent, then into blocks using `ocr_carea`. This mirrors the block/line/span hierarchy produced by vector extraction.

---

## 6. Multi-Language OCR

Before invoking Tesseract with a language pack, detect the dominant script per page region:

- Sample character glyph bitmaps and classify by Unicode block after a first-pass OCR with `osd` (orientation and script detection) mode (`PSM_OSD_ONLY`). Tesseract's OSD returns a script name string.
- Split the page into regions with differing scripts (e.g., a header in Latin, body in Arabic) and process each with the appropriate language pack.

For **mixed-script pages**, segment regions by script first using OSD on sub-regions, then pass each region with its own `Tesseract` instance initialized to the correct language.

**CJK vertical text** requires `PSM_SINGLE_BLOCK_VERT_TEXT` (5) and the `chi_sim_vert`, `chi_tra_vert`, `jpn_vert`, or `kor_vert` language packs. Vertical glyph metrics differ from horizontal; do not reuse a horizontal-mode session.

---

## 7. JBIG2 and CCITT Encoded Scans

Scanned PDFs predominantly use two image compression formats for bitonal (black-and-white) rasters:

**CCITT Group 4** (T.6 fax compression): lossless, row-by-row 2D encoding. Decoding is exact; the raster recovered is pixel-identical to the original scan. No quality loss affects OCR. Most PDF rasterization backends decode CCITT natively.

**JBIG2**: an adaptive dictionary-based bitonal compressor. Standard JBIG2 (lossless mode) is also exact. However, **lossy JBIG2** substitutes visually similar symbol bitmaps from a shared dictionary — a glyph that "looks like" another is silently replaced. This is a known issue that can cause OCR character substitutions that are invisible to visual inspection but corrupt extraction. When the PDF stream dictionary has `/Filter /JBIG2Decode` and the JBIG2 global segments contain a lossy-mode marker, log a warning and consider elevating OCR confidence thresholds or flagging output as potentially degraded. Use `jbig2dec` or equivalent for decoding.

---

## 8. Performance Considerations

OCR throughput is limited by CPU and, secondarily, by rasterization cost.

**Caching.** Cache rasterized page images keyed by `(pdf_hash, page_index, dpi)`. If the same document is processed repeatedly (e.g., during development or re-extraction), rasterization is the dominant cost and can be eliminated on repeat runs.

**Parallelism.** OCR pages in parallel using a thread pool (`rayon` is appropriate). Each `Tesseract` instance is not `Send`; initialize one instance per thread using thread-local storage. A pool of 4–8 threads is typical; beyond that, memory pressure from holding multiple full-page rasters simultaneously may become the bottleneck.

**GPU acceleration.** Tesseract supports CUDA via its LSTM implementation when compiled with `--with-cuda`. GPU acceleration yields 3–5× throughput improvement for LSTM OCR. However, CUDA adds a large build dependency; expose it as an optional Cargo feature (`ocr-gpu`) that links against `libtesseract` compiled with CUDA support.

**DPI/accuracy tradeoff.** For documents known to have large font sizes (e.g., presentation slides), 200 DPI is sufficient and halves raster memory. For documents with mixed font sizes, use 300 DPI and accept the overhead.

**Skip conditions.** Skip OCR entirely for: (a) PDF pages with `Encrypt` dictionaries that restrict content copying if the restriction is enforced; (b) pages the user has explicitly marked as skip via configuration; (c) pages where the page area is below a minimum threshold (e.g., < 1 cm²), which are likely decorative or separator elements.

---

## 9. Confidence and Provenance Tagging

Every span in `pdftract`'s output model carries a `source` field. OCR-derived spans must be tagged:

```rust
pub struct OcrProvenance {
    pub source: &'static str,        // "ocr"
    pub engine: String,              // "tesseract-5.3.1"
    pub dpi: u32,                    // rasterization DPI used
    pub word_confidence: f32,        // 0.0–1.0, from Tesseract per-word Confidence()
    pub page_confidence: f32,        // 0.0–1.0, from GetMeanTextConf()
    pub preprocessing: Vec<String>,  // ["deskew", "sauvola", "median_filter"]
}
```

Distinguish OCR spans from vector spans at the consumer level: downstream tools (chunkers, classifiers) may apply different trust weights. Never silently merge OCR and vector text without recording which source each span came from. When assisted OCR (section 4) resolves a conflict by preferring OCR over a vector candidate, record both the original vector text and the OCR text in the span's `alternatives` field so the consumer can audit the decision.
