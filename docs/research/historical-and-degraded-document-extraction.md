# Historical and Degraded Document Extraction

## Overview

Scanned historical documents, microfilm reproductions, low-quality photocopies, and physically degraded originals sit at the difficult end of the OCR spectrum. Each degradation type triggers a different failure mode in the extraction pipeline. Treating them all with a single generic filter produces consistently poor results. This document defines the degradation taxonomy, the algorithms to address each category, and the confidence policy that prevents garbage text from propagating silently into structured output.

---

## 1. Degradation Categories

**Salt-and-pepper noise** — random isolated black or white pixels scattered across the image. Origin: CCD sensor noise during scanning, dirty scanner glass, or film grain on microfilm. These pixels disrupt connected-component analysis and produce spurious characters.

**Background bleed-through** — text printed on the reverse side of thin paper (newspaper stock, onion-skin) transmits light and appears as a faint, laterally-mirrored ghost image. The secondary ink signal overlaps character frequency bands, making simple threshold separation unreliable.

**Uneven illumination** — gradient luminance across the scan: darker corners from a flatbed lid that does not press fully, a bright hotspot at image center from a overhead copy stand, or a gradient from left to right caused by angled ambient light. Otsu-style global thresholding collapses under this condition.

**Physical distortion** — page curl at the binding margin, keystoning from a camera held off-axis, rotational skew up to several degrees, and binding shadow (a darkening gradient toward the spine). Each produces geometric errors that break word and line segmentation.

**Ink spread or fading** — over-inked originals produce strokes that bleed together and merge adjacent characters; under-inked or aged originals produce strokes that are too light or discontinuous. Both extremes harm connected-component character recognition.

**Staining and foxing** — brown ferrous oxidation spots (foxing), water tide-marks, and adhesive residue produce high-contrast blobs in the same intensity range as ink. A naive binarizer classifies them as characters.

**Resolution too low** — below approximately 150 DPI, a lowercase `e` is fewer than 10 pixels tall. Individual stroke features are not resolved; the pixel grid is the limiting factor, not the algorithm.

**Mixed degradation** — a single page may exhibit three or four of the above simultaneously. A 19th-century newspaper scan can have bleed-through, salt-and-pepper noise, and a binding shadow on the same column.

---

## 2. Noise Reduction

Gaussian blur attenuates high-frequency noise but smears edge information, degrading thin character strokes. The **median filter** is the standard choice for salt-and-pepper noise: for each pixel, replace its value with the median of an N×N neighborhood. A 3×3 kernel removes isolated single-pixel noise; 5×5 handles heavier speckle while still preserving stroke edges because the median operation is nonlinear and resists the influence of outlier pixels.

For images with noise density above roughly 20% of pixels, the standard median filter degrades because the median itself may be drawn from noise pixels. The **adaptive median filter** (AMF) solves this by dynamically expanding the kernel size until the local median falls in a plausible range, capping at a maximum window (typically 7×7 or 9×9) before accepting the result.

After median filtering, **morphological opening** (erosion followed by dilation with a small structuring element, typically a 2×2 or 3×3 square) removes any remaining isolated foreground blobs smaller than the structuring element. Because erosion removes thin protrusions and isolated pixels, and the subsequent dilation restores objects that survived erosion, object-sized structures survive while noise pixels do not.

Recommended sequence for a noisy scan:

```
grayscale → median filter (5×5) → Sauvola binarization → morphological opening (3×3)
```

---

## 3. Bleed-Through Removal

Bleed-through is detectable by computing the **normalized cross-correlation** of the grayscale image with its horizontally mirrored version. High correlation (empirically above 0.15–0.25 depending on paper thickness) indicates bleed-through is present.

Removal relies on the density difference: the primary text is darker than the bleed signal. A locally-adaptive binarization threshold computed on the primary text's ink-density distribution should be tuned to exclude the lighter bleed-through layer. In practice, Sauvola thresholding with `k` pushed toward 0.4–0.5 (higher than the default 0.2) biases the threshold upward and rejects the lighter bleed pixels.

For severe bleed-through, the **Wiener filter** simultaneously denoises and deblurs in the frequency domain. Given an estimate of the noise power spectrum (from a blank region of the scan) and an assumed point-spread function for bleed (a Gaussian with σ ≈ 1.5–2.0 px representing paper diffusion), the Wiener filter minimizes the mean-squared error between the restored signal and the true primary text image. This is computationally heavier but appropriate when the bleed is dense enough that Sauvola alone misclassifies it.

---

## 4. Uneven Illumination Correction

The standard approach is **background estimation by large-kernel Gaussian blur**: apply a Gaussian with radius 50–100 pixels to the grayscale image. At that radius, all text is blurred away; what remains is an estimate of the smoothly-varying background luminance field. Divide each pixel by its corresponding background estimate, then rescale to [0, 255]. This is the core of homomorphic filtering adapted for reflective (not transmissive) illumination.

An alternative for scans with abrupt luminance changes (such as a shadow edge from a warped page): sample background intensity at a grid of points identified as non-text by their local standard deviation (low σ indicates no texture), fit a polynomial surface (degree 2 or 3) through those sample points using least-squares, and use the polynomial surface as the background estimate.

Both methods must run before binarization. Applying Sauvola to the illumination-corrected image is markedly more reliable than applying Sauvola directly to the raw scan, even though Sauvola is itself local — Sauvola's window cannot span the scale of a full-page gradient.

---

## 5. Geometric Correction

**Deskew** removes rotational skew. Two reliable approaches:

- *Hough transform*: detect line segments in the binary image, cluster their angles, take the dominant angle as the skew, and rotate the image by its negation.
- *Projection profile maximization*: rotate the binarized image in 0.1° steps over ±5°, compute the horizontal projection (row-wise pixel sum), and take the angle that maximizes the variance of that projection. At the correct angle, text lines produce sharp peaks; at other angles, the distribution flattens.

**Page curl** causes text baselines to follow a curve rather than a line. Detect curved baselines by fitting a polynomial (degree 2 or 3) through the centroid positions of connected components in each text line. Warp the image using a mesh warp (bicubic interpolation on a control-point grid) to map the curved baselines to horizontal lines.

**Perspective correction** applies to camera captures. Detect the four corners of the document (Hough lines on the document boundary, or corner-specific feature detectors), compute the projective transform that maps those four corners to a rectangle, and apply the transform with bilinear or bicubic resampling.

**Binding shadow** manifests as a darkening gradient toward the spine. After illumination correction (Section 4), this gradient is largely removed. If residual darkening remains, detect the gradient direction from the background luminance field estimate and apply a compensating brightness ramp along that axis.

---

## 6. Adaptive Binarization for Degraded Images

Global Otsu thresholding computes a single intensity threshold for the entire image. It fails catastrophically under uneven illumination because the optimal threshold for a dark region differs from the optimal threshold for a bright region.

**Sauvola thresholding** computes a local threshold for each pixel:

```
T(x,y) = μ(x,y) · [1 - k · (1 - σ(x,y) / R)]
```

where `μ` and `σ` are the local mean and standard deviation in a window of size W×W, `R` is the dynamic range of the standard deviation (typically 128 for 8-bit images), and `k ∈ [0.2, 0.5]` is a sensitivity parameter. Lower `k` accepts more pixels as foreground; higher `k` rejects lighter pixels.

Window size W should be approximately 2–3× the height of a typical character stroke in pixels. At 300 DPI, a standard printed character stroke is 3–5 px wide, so W = 25–51 is appropriate. At 150 DPI, W = 15–25.

**Wolf-Jolion modification** extends Sauvola to handle documents where ink is very light across the entire page (e.g., faded typewriter output). It normalizes the standard deviation term to the maximum standard deviation observed in the image, preventing the threshold from collapsing when global contrast is low.

**Niblack thresholding** is the predecessor to Sauvola: `T = μ + k·σ`. It tends to introduce more noise in background regions and is generally superseded by Sauvola, but may be useful as a reference baseline.

---

## 7. Stroke Reconstruction for Faded Ink

Faded ink may produce pixel values in the range 180–220 (light gray on a 0–255 scale with 255 = white), well below what Sauvola classifies as foreground. Pre-processing with **CLAHE** (contrast-limited adaptive histogram equalization) redistributes the local intensity histogram, amplifying low-contrast regions while clipping the redistribution to avoid over-amplifying noise. Apply CLAHE with a tile size of 8×8 or 16×16 and a clip limit of 2.0–4.0 before binarization.

For strokes that are binarized but broken (gap pixels within a stroke due to uneven fading), **morphological closing** (dilation followed by erosion) reconnects gaps up to the size of the structuring element. A horizontal structuring element (1×3 or 1×5) closes horizontal stroke gaps without merging characters vertically.

For severe cases, **skeleton-based reconstruction** extracts the stroke skeleton (Zhang-Suen or Guo-Hall thinning), which reduces each stroke to a 1-px-wide centerline even if the original stroke was intermittent. The skeleton is then dilated to a standard stroke width, producing a normalized binary image suitable for OCR even if the original was patchy.

---

## 8. Low-Resolution Handling

At 150 DPI, a typical lowercase character is 15–20 px tall. At 100 DPI, it is 10–13 px. Tesseract's documented minimum for reliable recognition is 300 DPI; it ships with a `--dpi` flag that accepts an override, but the underlying character models are trained at 300 DPI and degrade sharply below 150 DPI.

**Bicubic upsampling** to 300 DPI before OCR is the minimum intervention — it does not recover lost detail but gives the recognizer familiar feature dimensions. For moderate quality gain, **ESRGAN-class super-resolution models** (Real-ESRGAN or a document-specific fine-tune) trained on document imagery can synthesize plausible high-frequency detail. These models are not appropriate for legal or archival use where fabrication of detail is unacceptable, but for readability-oriented extraction they can recover legible characters from 150 DPI inputs.

When the computed DPI is below 100 and the image shows no recoverable character features (assessed by measuring the variance of the horizontal projection profile — very low variance indicates characters are not distinguishable), the pipeline should emit a `low_quality_page` warning and still return the best-effort text, rather than silently inserting high-confidence garbled output.

---

## 9. Script and Typeface Detection for Historical Documents

Historical documents may be typeset in scripts and conventions no longer current:

- **Blackletter** (Fraktur, Schwabacher, Textura): dominant in German-language printing through the 1940s. Recognizable by the high angle of oblique strokes (typically 40–60° from horizontal, compared to 10–20° for Roman). A histogram of local gradient orientations in the binary image distinguishes blackletter from Roman with high reliability. Tesseract provides a `script/Fraktur` language pack trained on 19th-century German texts; recognition quality is significantly below Latin for degraded inputs and improves with pre-processing.
- **Long s** (`ſ`, U+017F): used in early modern printing for non-final `s`. OCR models trained on modern text misclassify it as `f`. Post-processing rules can correct `ſ→f` substitutions in known-context positions (not at word-final positions, not before another `s`).
- **Typewriter fonts**: monospaced, lighter ink density than letterpress, often on thin paper with higher bleed-through risk. The uniform character width is an asset for segmentation but the lighter ink requires lower Sauvola `k`.
- **Ligatures**: fi, fl, ffi, ffl, ct, st, and the long-s ligatures ſi, ſl are common in 18th–19th-century setting. These are single glyphs occupying the width of two characters; models that segment character-by-character before recognition will fail on them. Tesseract's LSTM engine handles ligatures at the word level and is preferred over the legacy mode for historical documents.

---

## 10. Confidence-Gated Fallback

Tesseract's C API exposes `ResultIterator::Confidence()`, which returns a per-word confidence in [0, 100]. Aggregate to the **block level** by taking the mean confidence across all words in a block, and to the **page level** by taking the mean across all blocks (weighted by block word count).

Output policy:

- **Page-level confidence ≥ 60**: emit text normally.
- **40 ≤ page-level confidence < 60**: emit text with a `degraded_quality` annotation in the extraction metadata. The text is usable but should be treated as approximate.
- **Page-level confidence < 40**: emit a `low_quality_page` warning in the structured output. Include the best-effort text — do not discard it — but mark it explicitly so that downstream consumers (e.g., LLM pipelines) can weight it appropriately or skip it.

Never silently emit garbled text without confidence metadata. A word recognition confidence below 20 should be individually flagged; the extraction output format should support per-word confidence annotation, not just per-page. This allows downstream consumers to apply their own threshold rather than receiving binary pass/fail decisions from the extractor.

The confidence gating applies after all pre-processing. Running the full degradation-correction pipeline before measuring confidence ensures that the confidence score reflects true unrecoverability rather than a correctable image quality issue.
