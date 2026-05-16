# Shading, Pattern Fills, and Their Interaction with Text Visibility

## Overview

PDF color is a layered system: color spaces define how numeric values map to perceptual colors, paint operators apply those values to the current path or glyph, and compositing operators blend painted content onto the page. For a text extraction library, the critical distinction is between color attributes that affect *what characters exist in the content stream* and those that affect only *how those characters are rendered*. Character codes, Unicode mappings, and glyph positions survive every color operation; what can genuinely suppress extraction value is text that was intentionally painted to be invisible — matching or nearly matching its background — as a watermark defense or layout artifact. pdftract must navigate this distinction without discarding valid content and without treating deliberately hidden text as a false negative.

## PDF Color Space Taxonomy and Text Readability

PDF defines color spaces across three tiers. Device spaces — DeviceGray, DeviceRGB, and DeviceCMYK — map values directly to output device primaries with no calibration curve. Calibrated spaces — CalGray, CalRGB, and Lab — embed a white point and gamma, producing device-independent color. Special spaces — ICC-based, Indexed, Pattern, Separation, and DeviceN — add indirection through profiles, lookup tables, or alternate spaces.

From a text-extraction standpoint, device and calibrated spaces are straightforward: a single numeric tuple fully describes the painted color, and pdftract can compute its luminance for contrast analysis. ICC-based spaces reduce to an alternate color space when the embedded profile is unavailable, which is the common case at extraction time; pdftract should treat them as their declared alternate. Indexed spaces map a single integer through a lookup table to entries in a base space, so the effective color is always resolvable given the lookup table embedded in the PDF.

Pattern and Separation spaces require separate treatment and are addressed in later sections. DeviceN is a generalization of Separation that covers multi-ink systems; it includes an alternate space and a tinting function that approximates the multi-ink blend in a device-independent space, and pdftract uses that alternate for luminance estimation.

## Pattern Color Spaces: Tiling and Shading

When the current color space is set to `/Pattern`, paint operations use a pattern dictionary rather than a numeric tuple. Type 1 patterns (tiling) define a cell that is replicated across the painted region; Type 2 patterns (shading) compute color from a shading function and paint it across the bounding box.

A critical architectural point: when text glyphs are painted with a pattern fill, the character codes are already present in the content stream, bound to their Unicode mappings through the font's ToUnicode CMap or encoding vector. The pattern fill is a rendering instruction applied *after* the character is decoded. pdftract reads character codes during content stream parsing, before any paint step, so pattern-filled text requires no special extraction handling. The text is fully extractable regardless of the pattern type.

The only practical implication is metadata: pdftract may annotate a span with `fill_type: pattern` to signal that visual rendering requires pattern evaluation, but this annotation carries no effect on Unicode recovery or position confidence.

## Shading Types and Gradient-Rendered Text

PDF defines seven shading types under `/ShadingType`: function-based (type 1), axial/linear (type 2), radial (type 3), free-form Gouraud-shaded triangle mesh (type 4), lattice-form triangle mesh (type 5), Coons patch mesh (type 6), and tensor-product patch mesh (type 7). Types 2 and 3 are the gradient forms most commonly seen in practice; types 4–7 appear in complex illustration work.

When text itself is painted with a shading fill — an uncommon but valid PDF construction that requires a special graphics state sequence involving a clip to the glyph outlines followed by a shading paint — extraction is entirely unaffected. The character codes, positions, and font metadata were already parsed. pdftract records the presence of a shading fill as span metadata for downstream rendering systems, but takes no special extraction action.

## Gradient Backgrounds and Contrast Detection

The more common interaction between shading and text is a background shading rectangle painted before the text. An axial or radial gradient is drawn across the page or column region, and then text is painted on top in a fixed color. Here, the text color is deterministic but the background is spatially varying.

pdftract must estimate the effective luminance of the background at each span's bounding region to assess whether sufficient contrast exists. For axial gradients, this means evaluating the shading function at the x- and y-coordinates corresponding to the span's center point or, for spans spanning a wide gradient region, evaluating at both endpoints and taking the minimum contrast. The shading function — whether a sampled function (type 0), exponential (type 2), stitching (type 3), or PostScript calculator (type 4) — is embedded in the shading dictionary and can be evaluated at arbitrary coordinates.

For mesh shadins (types 4–7), evaluating the function at an arbitrary point is computationally involved. pdftract should fall back to sampling the declared color space bounds of the shading dictionary (the `/BBox` combined with the `/Function` domain endpoints) to compute a worst-case luminance range. If the entire luminance range produces contrast ratios above the threshold, the text is confidently visible; if any part of the range falls below threshold, pdftract applies the `low_contrast` confidence penalty and records the computed range in span metadata.

## White-on-White and Low-Contrast Text Detection

Text painted in a color that matches or nearly matches the page background is the primary class of intentionally invisible text. Detection requires computing the WCAG 2.1 relative luminance contrast ratio between the text color and the background color at that span's location.

Relative luminance L is computed from linearized RGB: each sRGB channel c is linearized as `c/12.92` when `c <= 0.04045` and `((c + 0.055)/1.055)^2.4` otherwise, then combined as `L = 0.2126*R + 0.7152*G + 0.0722*B`. The contrast ratio is `(L_lighter + 0.05) / (L_darker + 0.05)`. WCAG 2.1 AA requires a ratio of 4.5:1 for normal text; pdftract uses a lower internal threshold of 1.5:1 as the cutoff below which text is classified as color-hidden, since even heavily degraded low-contrast text (ratio between 1.5 and 4.5) may be intentionally visible in specialized contexts such as watermarks or light-colored metadata annotations that the document author intended to display against a custom background.

When the page background is white (the default), the computation simplifies: any text color whose luminance L satisfies `(1.05) / (L + 0.05) < 1.5` — meaning `L > 0.65` approximately — is treated as color-hidden. This covers light gray, near-white, and white text.

The background color at any span position is resolved by walking the graphics state stack backward through the rendering sequence to find the most recently painted opaque background intersecting the span bounding box. For complex pages with layered content, pdftract performs this analysis during a two-pass content stream parse: first pass builds a background color map by recording all filled rectangles and their colors; second pass assigns background colors to each text span.

## Spot Colors and DeviceN Alternate Space Mapping

Spot colors — Separation color spaces referencing named inks such as Pantone values — are defined with an alternate color space and a tinting function that approximates the spot ink in the alternate space. When text is painted in a spot color, pdftract evaluates the tinting function at full coverage (tint value 1.0) in the alternate space to estimate the effective color. If the alternate space is DeviceRGB or DeviceCMYK, the resulting value is converted to luminance through the standard path.

When the alternate space maps to near-white at full tint — a luminance above 0.65 — pdftract applies the same `color_hidden` classification as for direct white-text cases. If the alternate space is unavailable or the tinting function is a PostScript type 4 function that pdftract cannot evaluate, the span is extracted with full confidence and annotated with `fill_type: spot_color_unknown_alternate`. Dropping text because the spot color alternate could not be resolved would produce false negatives; the conservative policy is to extract and annotate.

DeviceN follows the same logic. pdftract evaluates the DeviceN tinting function at the colorant values specified in the text paint operation, maps the result through the alternate space, and applies luminance-based contrast analysis. If evaluation fails, the span is extracted with reduced confidence rather than suppressed.

## Pattern-Filled Backgrounds and OCR Fallback Policy

When a tiling pattern creates a textured background — hatching, stippling, or repeating imagery — underneath vector text, the vector extraction path is entirely unaffected: character codes come from the text content stream, not from rendered pixels. pdftract's vector extraction reads the font and text operators directly and requires no image processing. OCR is a fallback mechanism for pages or regions where vector text is absent — scanned pages, image-only XObjects, or rasterized text.

The policy is: attempt vector extraction first; if the content stream contains text operators for a given region, accept those results regardless of the visual complexity of the background. OCR is only invoked when the vector extraction pass returns no text for a region that contains rasterized content. A background tiling pattern does not demote the page to OCR status.

## Transparency Groups and Reduced-Opacity Text

Form XObjects may declare themselves as transparency groups, and their contents can be painted onto the page with a reduced alpha value via the `ca` (fill opacity) and `CA` (stroke opacity) graphics state parameters. When text inside a transparency group is painted at reduced opacity — for instance, a watermark group at `ca 0.3` — the character codes, font references, and positions within the XObject content stream are fully parsed by pdftract's content stream reader during XObject traversal. Opacity is a compositing parameter applied at rendering time; it does not remove characters from the stream.

pdftract's content stream parser recursively descends into Form XObjects, inheriting the graphics state at the invocation point. Opacity values from the invoking graphics state are recorded in span metadata as `opacity: 0.3` but have no bearing on whether the span is extracted. A text span at any nonzero opacity is extractable.

## Blend Modes and Unicode Recovery

The `/BM` graphics state key sets the blend mode used when painting onto the page. Non-Normal blend modes — Multiply, Screen, Overlay, Darken, Lighten, ColorDodge, ColorBurn, HardLight, SoftLight, Difference, Exclusion, Hue, Saturation, Color, Luminosity — affect the composited pixel output but have no effect on character decoding. pdftract records the active blend mode in span metadata as `blend_mode: Multiply` (for example) to allow downstream consumers to reason about visual appearance, but Unicode recovery is identical for all blend modes.

The only extraction-relevant consequence of a non-Normal blend mode is the contrast analysis step. Multiply blend mode applied to black text on a white background yields black, which is visible; applied to light gray text on white, it deepens the text toward the background color. If contrast analysis is triggered, pdftract must account for the blend mode when computing the effective composited color. For Normal, this is a direct substitution; for other modes, pdftract computes the composited result using the standard blend mode equations before applying the luminance threshold check.

## Extraction Policy for Color-Hidden Text

Spans that fall below the contrast threshold receive a `color_hidden: true` flag in the extraction output. This flag does not suppress the span from the results. The character data, position, and font metadata are included in full; the flag is advisory, informing the caller that the text was likely not intended to be read by a human viewer of the rendered document. Extraction confidence is reduced proportionally: spans at contrast ratio below 1.1 (near-invisible) receive a confidence penalty of 0.4; spans between 1.1 and 1.5 receive a penalty of 0.2.

The rationale for extracting rather than suppressing is that extraction consumers — search indexers, accessibility tools, content pipelines — derive value from the text regardless of its visual presentation. Invisible text in a PDF may represent hidden metadata, copy-protection watermarks, or template artifacts; all of these are legitimate extraction targets. Suppression would be a silent false negative. The `color_hidden` flag gives callers the information they need to apply their own policy.

When reporting extraction output in structured form, pdftract groups `color_hidden` spans in a dedicated section of the output manifest, alongside their contrast ratios and the color values resolved for both text and background. This audit trail allows callers to verify the classification and override it if domain knowledge suggests the text was intentionally visible in a specialty printing context that pdftract's sRGB-based luminance model does not capture.
