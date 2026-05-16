# Watermark and Background Separation

## Purpose

Watermarks, background images, decorative graphics, and repeating patterns degrade text extraction in two distinct ways: they inject unwanted strings into the text stream when rendered as PDF text operators, and they reduce OCR accuracy on scanned pages by overlapping with real characters. This document describes how each mechanism manifests in the PDF specification, how to detect each variant, and what suppression policy to apply.

---

## 1. How Watermarks Appear in PDFs

Five distinct mechanisms produce watermark or background content:

**(a) Semi-transparent text via ExtGState `ca`.**
A graphics state dictionary in the page's `ExtGState` resource can set `ca` (fill alpha) to a value between 0 and 1. The content stream loads this state with `gs`, then renders text normally with `BT`/`ET` operators. The rendered text appears faded on screen but is fully present in the content stream. Detection requires tracking the current alpha during parsing, not inspecting the visual output.

**(b) Large image XObject behind page content.**
The content stream places a full-page or near-full-page image using `Do` before any text operators appear. The image is an indirect reference to an XObject of subtype `Image` in the page's `Resources` dictionary. Background images placed this way are ordering-dependent: the `Do` precedes `BT`, which is the positional signal.

**(c) Form XObject repeated via Resources.**
A single Form XObject (XObject subtype `Form`) defined once in the PDF and referenced from the `Resources` of multiple pages. On each page the content stream invokes it with `Do` as one of the first operations. Because the form is defined once and shared, its content stream is parsed independently of each page's content stream. Detection requires cross-referencing which XObjects appear in the Resources of many pages and which are invoked early in each content stream.

**(d) OCG layer marked as background.**
An Optional Content Group with a `Name` of "Background", "Watermark", or similar, referenced via a `Marked Content` sequence (`/OC BMC ... EMC` or `BDC ... EMC`). The OCG's `Intent` array or the `Usage` dictionary `View` entry may have `PrintState` or `ViewState` set to `OFF`. Content inside this marked region is background by declaration. The OCG name and intent are the primary signals; see the optional-content-groups research document for the full OCG traversal algorithm.

**(e) Low-contrast color text.**
Text rendered in light gray (e.g., RGB `0.85 0.85 0.85`) against a white background, or very light tint of any hue. No alpha involved; the graphics state fill color set by `rg` or `g` operators carries the signal. The contrast ratio between the text color and the background estimate determines whether the text is decorative.

---

## 2. Transparency-Based Detection

During content stream parsing, maintain a graphics state stack mirroring what `q`/`Q` operators push and pop. Each stack frame carries:

```
struct GState {
    fill_alpha: f32,    // ca, default 1.0
    stroke_alpha: f32,  // CA, default 1.0
    blend_mode: BlendMode,
    ctm: Matrix3x3,
    fill_color: Color,
}
```

When a `gs` operator references an ExtGState dictionary, extract `ca`, `CA`, and `BM` from that dictionary and update the current frame. When a text span or image `Do` is encountered, annotate it with the current `fill_alpha`.

**Alpha threshold:** spans or images with `fill_alpha < 0.5` are watermark candidates. The threshold accounts for watermarks typically rendered between 0.1 and 0.4 alpha.

**Blend mode signal:** blend modes `Multiply`, `Screen`, `Overlay`, and `Luminosity` are structurally typical for watermarks. A span with alpha between 0.5 and 0.8 but a non-Normal blend mode should be escalated to a watermark candidate. Normal blend mode at alpha = 1.0 is never a watermark by this signal alone.

**Area weighting:** a single character at low alpha is not a watermark. A text element whose bounding box covers more than 30% of the page area at low alpha is a strong watermark candidate.

---

## 3. Positional Repetition Detection

Some watermarks are rendered at full opacity (alpha = 1.0) but appear at a fixed position on every page. Detection requires a cross-page pass.

Build a normalized position inventory during the first parse pass. For each text span and image `Do`, record:

```
(normalized_x, normalized_y, width_fraction, height_fraction, content_hash)
```

where coordinates are divided by the page's `MediaBox` dimensions. After parsing all pages, count how often each `(normalized_x, normalized_y, content_hash)` tuple appears. Elements present on more than 80% of pages at the same normalized position are watermark candidates regardless of alpha.

**Diagonal watermarks:** a diagonal "CONFIDENTIAL" or "DRAFT" watermark is typically centered on the page with a rotated CTM. The CTM rotation angle (extracted from the `cm` operator or inherited via Form XObject) of ±45° combined with a bounding box centered near (0.5, 0.5) normalized is a diagnostic pattern. The positional repetition check applies equally — the normalized center position and rotation angle form the key.

**Recto/verso patterns:** for duplex-printed documents, a watermark may appear only on odd or even pages. The 80% threshold handles this naturally if the document has more than ten pages; for shorter documents, run the check separately on odd and even page sets.

---

## 4. Form XObject Reuse as Background

A Form XObject used as a background is parsed once but `Do`-invoked on every page. The detection algorithm:

1. For each page, record the order in which XObjects are invoked relative to `BT` operators. An XObject invoked before any `BT` on the page gets a `pre_text = true` flag.
2. Count how many pages invoke each XObject with `pre_text = true`.
3. Any XObject invoked with `pre_text = true` on more than 80% of pages is a background Form XObject candidate.
4. Parse the Form XObject's own content stream. If it contains `BT`/`ET` sequences, the background carries text (common in letterhead watermarks). If it contains only path operators (`m`, `l`, `c`, `re`, `f`, `S`) or image `Do` operators, it is a purely graphic background.

This classification determines suppression: text-carrying Form XObjects need text-level filtering; graphic Form XObjects are suppressed at the render level.

---

## 5. Color-Based Filtering

Track the current fill color during parsing. For `g` (grayscale), `rg` (RGB), `k` (CMYK), and their stroke equivalents, maintain the current color in the graphics state.

Compute the WCAG relative luminance for each text span's fill color:

```
L = 0.2126 * linearize(R) + 0.7152 * linearize(G) + 0.0722 * linearize(B)
// linearize(c) = c/12.92 if c <= 0.04045, else ((c+0.055)/1.055)^2.4
```

Assuming a white background (`L_bg = 1.0`), the contrast ratio is `(L_bg + 0.05) / (L_text + 0.05)`. Text with contrast ratio below 2.0 is likely decorative or a watermark. Text with contrast between 2.0 and 3.0 is ambiguous and should be labeled but not suppressed by default.

For non-white backgrounds, the background luminance must be estimated. If the page contains a background image, use the median luminance of the region beneath the text span. If no background image exists, assume white.

---

## 6. OCR Preprocessing: Raster Watermark Removal

For scanned PDFs, the watermark is baked into the raster image. Two detection approaches apply before passing the page image to Tesseract:

**Connected components approach:** binarize the page image (Otsu threshold). Run connected-component labeling. Very large connected components that span more than 20% of the page width or height, are not rectangular (i.e., shaped like text glyphs), and whose pixel color deviates from the local background by less than 30 gray levels are watermark region candidates. Inpaint these regions by replacing pixels with the local median background color (sampled from a 16-pixel border around the component's bounding box).

**Frequency domain approach:** periodic watermarks (repeating logos or patterns) appear as discrete peaks in the 2D discrete Fourier transform of the page image. Apply a notch filter centered on those peaks, then invert the DFT. This is effective for grid or tiling patterns but less targeted than connected-component inpainting for text watermarks.

Inpainting is applied regardless of the output suppression policy — the OCR input must be clean even if the caller has requested `include_watermarks: true`.

---

## 7. Diagonal Text Watermarks on Scans

"CONFIDENTIAL", "DRAFT", and "COPY" watermarks typically appear at 45° rotation, large font, spanning the page diagonally. Detection on the rasterized image:

1. **Hough line transform** on the binarized image restricted to angles 40°–50°. A strong response in this range with lines passing through the page center signals a diagonal text watermark.
2. **Large connected components at 45° orientation:** compute the principal axis of each large connected component (PCA on pixel coordinates). Components whose principal axis is within 5° of 45° and whose bounding box area exceeds 5% of the page are candidates.
3. **Confirmation by OCR in the rotated region:** rotate the candidate region by −45° and run Tesseract on the sub-image. If the recognized text matches a known watermark vocabulary ("CONFIDENTIAL", "DRAFT", "COPY", "SAMPLE", "VOID") with confidence > 0.7, the region is confirmed.
4. Mask the confirmed region with the local background estimate before the main OCR pass.

---

## 8. Background Images vs. Content Images

Both appear as XObjects of subtype `Image`, but their roles differ:

| Signal | Background image | Content figure |
|---|---|---|
| Rendered area / page area | > 80% | < 60% |
| Position in content stream | Before `BT` | After `BT` or between text blocks |
| Image content entropy | Low (solid color, gradient) | High (photograph, chart) |
| Proximity to text | Text overlaps the image | Text is adjacent, not overlapping |

Compute image entropy as the Shannon entropy of the pixel value histogram (8-bit grayscale, 256 bins). A solid-color image has entropy near 0; a photograph typically has entropy above 5 bits. Threshold at 3.0 bits: below is background, above is content.

The content stream ordering check is the highest-confidence signal and should gate the entropy check. An image placed after all text operators on a page cannot be a background by definition.

---

## 9. Suppression Policy

Three disposition options apply per detected watermark element:

**(a) Exclude from text output entirely.** Default for pure decorative elements (graphic Form XObjects, background images, transparent non-text spans). No representation in the output text stream.

**(b) Include with `zone: "watermark"` label.** The watermark text span is included in the main text stream but tagged so callers can filter it. Useful when the caller needs to be aware of what the document says (e.g., "DRAFT") without mistaking it for body text.

**(c) Include with `visible: false`.** The span is present in the structured output but excluded from any plain-text serialization. Callers querying the structured representation can access it; plain-text users cannot.

The caller controls behavior via:

```rust
pub struct ExtractionOptions {
    pub include_watermarks: bool,   // default: false
    pub watermark_zone_label: bool, // default: true (when include_watermarks = true)
}
```

For scanned pages, inpainting is unconditional — it happens before OCR regardless of the output policy.

---

## 10. Output Structure

Each page's output includes a `watermarks` array:

```rust
pub struct WatermarkRecord {
    pub kind: WatermarkKind,           // Text | Image | FormXObject
    pub text: Option<String>,          // populated for text watermarks
    pub bbox: Rect,
    pub alpha: Option<f32>,            // None if detected by repetition or color
    pub detection_method: DetectionMethod,
    pub page_indices: Vec<usize>,      // pages where this watermark was detected
}

pub enum DetectionMethod {
    Transparency,      // ca < 0.5
    Repetition,        // same position on > 80% of pages
    ColorContrast,     // WCAG contrast < 2.0
    OcgLayer,          // marked inside a background OCG
    RasterDetection,   // connected component or Hough on scan
}
```

Text spans that are included in the main stream despite being watermarks carry:

```rust
pub struct TextSpan {
    // ...
    pub zone: Option<ZoneLabel>,  // Some(ZoneLabel::Watermark) when applicable
    pub visible: bool,
}
```

The `watermarks` array is populated even when `include_watermarks: false` — callers can always inspect what was suppressed without requesting its inclusion in the text stream.
