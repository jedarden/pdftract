# Redaction Detection and Recovery

## Overview

PDF redaction is the process of permanently removing sensitive content from a document before publication. The operative word is "permanently" — proper redaction destroys the underlying data. In practice, a significant fraction of published "redacted" documents fail this requirement: the content is visually obscured but remains fully accessible in the content stream. `pdftract` must handle both cases correctly, surfacing recoverable text while accurately representing the extraction state to the caller.

---

## 1. Proper vs. Improper Redaction

**Proper redaction** modifies the content stream itself. The text operators covering the redacted region are removed and replaced with an opaque fill (typically black). The original characters are gone; no amount of content-stream inspection will recover them.

**Improper redaction** leaves the original text operators intact in the content stream and merely paints a covering graphic on top — a black rectangle, a dark raster image, or an opaque layer element. The text is fully present and extractable without any special technique; it simply is not rendered visibly.

The prevalence of improper redaction in government and legal documents is well-documented. Entire classified passages, witness names, and financial figures have been recovered from "redacted" PDFs produced by government agencies, law firms, and courts — the producing party drew a black box in Word or Acrobat without invoking the actual redaction workflow. `pdftract` must distinguish which case it is in, both to recover text where possible and to label that text with appropriate provenance warnings.

---

## 2. PDF Redaction Annotations (`/Redact`)

PDF 1.7 (ISO 32000-1) introduced the `/Redact` annotation subtype. A redaction annotation marks a region for removal and carries metadata about the intended replacement appearance. Key dictionary entries:

| Key | Type | Description |
|---|---|---|
| `/Subtype` | name | Must be `/Redact` |
| `/QuadPoints` | array of numbers | Pairs of x,y coordinates defining the covered quadrilaterals |
| `/IC` | array | Interior fill color (DeviceRGB), typically `[0 0 0]` (black) |
| `/OverlayText` | text string | Replacement text rendered after apply (often empty or `"[REDACTED]"`) |
| `/Repeat` | boolean | Tile overlay text to fill the region |
| `/DA` | string | Default appearance string for overlay text (font, size, color) |
| `/RO` | stream | Rollover appearance XObject |

The critical distinction for `pdftract` is **applied vs. unapplied**:

- **Unapplied**: The annotation exists in the page `Annots` array but "Apply Redactions" has never been invoked. The content stream is unmodified. The text under `QuadPoints` is fully present and extractable.
- **Applied**: The application consumed the annotation (removed it from `Annots`), deleted the covered text operators from the content stream, and rendered the fill rectangle and overlay text directly into the stream. The annotation no longer exists. The text is genuinely absent.

When a `/Redact` annotation is still present in `Annots`, the document was not properly redacted. This is a detection opportunity.

---

## 3. Detecting Unapplied Redaction Annotations

During page object parsing, after collecting the `Annots` array, filter for entries where `/Subtype` equals `/Redact`. Each such entry represents intended but unapplied redaction.

**Algorithm:**

1. Resolve the `Annots` indirect references for the page.
2. For each annotation dictionary, check `/Subtype /Redact`.
3. Extract the `QuadPoints` array. Each group of eight values `[x1 y1 x2 y2 x3 y3 x4 y4]` defines one quadrilateral in page space (bottom-left origin).
4. Compute the axis-aligned bounding box of each quadrilateral.
5. After content-stream extraction, intersect these bounding boxes with the extracted text spans using the overlap test from Section 4.
6. Collect all spans whose bounding boxes overlap significantly with any redaction quadrilateral.

**Output fields** for each discovered unapplied annotation:

```rust
RedactionEvent {
    event_type: RedactionType::UnappliedAnnotation,
    bbox: Rect,              // from QuadPoints
    annotation_ref: ObjRef, // indirect reference to the annotation dict
    recovered_text: Option<String>,
    warning: "unapplied_redaction_detected",
}
```

The recovered text must be included in page output with `zone: "redacted_content"` and `redaction_warning: true`. The caller can suppress it with `include_redacted_content: false`, but the `redaction_events` entry is always emitted regardless of that flag.

---

## 4. Detecting Improper Redaction via Black Rectangle Overlap

The most common improper redaction draws a filled black path over text using the PDF graphics operators `f`, `F`, `f*`, or `B` (fill or fill-and-stroke).

**Detection algorithm:**

1. During graphics state tracking, maintain a list of closed filled paths with their current fill color.
2. A path qualifies as a candidate redaction rectangle when:
   - The current fill colorspace is DeviceGray, DeviceRGB, or a DeviceCMYK equivalent resolving to near-black (luminance < 0.05 after conversion to linear sRGB).
   - The path's axis-aligned bounding box has area > 100 square points (roughly 1.4 cm², filtering out hairlines and thin rules).
   - The path is convex (or is literally a rectangle: four straight segments forming a closed loop).
3. After both path collection and text span extraction are complete, test each text span against each candidate rectangle.
4. Overlap test: compute the Intersection over Union (IoU) of the span's bounding box and the rectangle's bounding box. An IoU > 0.5 indicates the span is substantially covered.

Painting order matters. A black rectangle drawn **after** the text (later in the content stream) visually covers it but leaves the text operators intact. A rectangle drawn **before** the text would be painted over by the text, not covering it. Track the stream position index of each element to enforce the ordering requirement: the covering rectangle must have a higher stream position than the text spans it overlaps.

**Output:** `RedactionEvent { event_type: RedactionType::CoveringRectangle, covering_element: CoveringElement::Rectangle, ... }`.

---

## 5. Detecting Improper Redaction via Image Overlay

A raster image XObject (placed with the `Do` operator) can serve as a covering black patch. This is common when screen-captured redaction tools export to PDF.

**Detection algorithm:**

1. During content stream processing, when `Do` is encountered with an XObject of `/Subtype /Image`, record the image's position and dimensions in page space (derived from the current transformation matrix at the time of `Do`).
2. Decode the image into grayscale (or convert from its native colorspace). Compute the mean pixel luminance.
3. A covering image candidate satisfies:
   - Mean luminance < 30/255 (approximately 12% brightness).
   - Rendered area > 100 square points (same threshold as rectangles).
4. Apply the same IoU > 0.5 overlap test against text spans, with the same stream-position ordering requirement (image rendered after text).

For inline images (`BI`/`EI`), apply identical criteria.

**Output:** `covering_element: CoveringElement::Image`.

---

## 6. Layer-Based Redaction

Optional Content Groups (OCGs, PDF 1.5+) can implement redaction by placing covering graphics on a visible layer above a text layer. The default OCG configuration (`/D` dictionary in the `/OCProperties` dictionary) controls which layers are visible on open.

**Detection algorithm:**

1. Parse the `OCProperties` dictionary from the document catalog.
2. Enumerate all OCGs and their default visibility (`/ON` vs. `/OFF` in the `/D/OFF` and `/D/ON` arrays).
3. For each content stream element, note its associated OCG (from enclosing `BDC` marked-content sequences with `/OC` property or from `/OC` entries on XObjects).
4. Identify OCGs that consist entirely of near-black filled rectangles or dark images (using the criteria from Sections 4 and 5). Call these "redaction layers."
5. Identify OCGs that contain text spans at the same page positions. Call these "content layers."
6. If a redaction layer is in the default-on set and a content layer at the same position is in the default-on set (both visible simultaneously), the text is covered but present.

Note that text on any layer — regardless of its visibility in the default configuration — is present in the content stream and extractable. The layer's visibility state is a rendering hint, not a data presence indicator.

**Output:** `covering_element: CoveringElement::Layer`, plus the OCG name in the event metadata.

---

## 7. Text Under Transparency

A translucent dark rectangle (fill color near black, but painted into an ExtGState with `ca` < 1.0, or using blend mode `Multiply`) obscures text visually but does not remove it from the content stream.

Detection follows the same bounding-box overlap logic as Section 4, with the additional criterion that the ExtGState's `ca` (non-stroking alpha) is less than 1.0. The luminance threshold may be relaxed slightly: a 50% opaque black rectangle has an effective luminance of ~0.5 against a white background, but the intent is still concealment. Apply a threshold of effective luminance (alpha × fill_luminance) < 0.3.

The text is fully extractable regardless. Emit the event with `event_type: RedactionType::TransparentOverlay`.

---

## 8. Color-Match Concealment in Redaction Context

White text on a white background (or any text whose fill color matches the page background) is covered in the invisible-text document; however, in a redaction context it takes on additional significance. When white-on-white text appears in a region that immediately follows a `/Redact` annotation in the annotation list, or where a same-color filled rectangle was drawn, this combination is a deliberate concealment pattern rather than an incidental rendering artifact.

Detect this by noting the position of white-on-white spans and correlating against: (a) nearby unapplied `/Redact` annotations, and (b) same-color background rectangles drawn at the same position. When the correlation fires, emit `event_type: RedactionType::ColorMatchConcealment` in addition to the standard invisible-text warning.

---

## 9. Properly Applied Redaction: What Remains

When redaction is correctly applied, the authoring tool modifies the content stream: text operators in the covered region are deleted, and a filled rectangle (in the redaction color) is inserted in their place. The `/Redact` annotation is consumed and removed from `Annots`. There is no annotation trail remaining in the live document.

Evidence of past redaction may appear in:

- **XMP metadata**: The `xmpMM:History` array may contain `stEvt:action = "saved"` entries with software like "Acrobat Redact," or a `pdfx:Marked` field indicating the document was reviewed.
- **Content stream gaps**: Regions of the page that contain only filled black rectangles with no surrounding text activity, especially when the surrounding text flow suggests missing words.
- **Structural gaps in tagged PDFs**: `/Artifact` tagged elements covering regions with no associated `ActualText` where surrounding structure implies content should be present.

`pdftract` cannot recover properly applied redaction — the data is gone. The extractor will encounter the black fill rectangle (a graphics element, not a covering graphic over text), produce no text spans for that region, and may optionally note the apparent gap as `event_type: RedactionType::AppliedRedaction` when heuristics are confident.

---

## 10. Output and Policy

All redaction events are gathered into a per-page `redaction_events: Vec<RedactionEvent>` field, always populated regardless of `include_redacted_content`.

```rust
pub struct RedactionEvent {
    pub event_type: RedactionType,
    pub bbox: Rect,
    pub covering_element: Option<CoveringElement>,
    pub recovered_text: Option<String>,
    pub redaction_warning: bool,
    pub annotation_ref: Option<ObjRef>,
}

pub enum RedactionType {
    UnappliedAnnotation,
    CoveringRectangle,
    CoveringImage,
    LayerBased,
    TransparentOverlay,
    ColorMatchConcealment,
    AppliedRedaction,
}

pub enum CoveringElement {
    Rectangle,
    Image,
    Layer,
}
```

Text spans recovered from improper redaction carry:

- `zone: "redacted_content"` for unapplied `/Redact` annotations.
- `zone: "covered_content"` for rectangle, image, or layer-based improper redaction.
- `redaction_warning: true` on the span.

When `include_redacted_content: false`, these spans are omitted from the text output but their `RedactionEvent` entries remain. This allows callers (e.g., a compliance tool) to detect and report improper redaction without inadvertently re-publishing the content.

The default is `include_redacted_content: true` — `pdftract`'s goal is maximum text recovery, and suppression is an explicit caller decision.
