# Color Management, ICC Profiles, and Color Space Conversions in PDFs

## Overview

PDF color is richer than extraction requires, but understanding its structure is necessary to implement one extraction-critical feature: luminance estimation for text visibility detection. pdftract does not perform color-managed rendering — it never converts pixel values through ICC profiles or simulates halftone screens. Its goal is narrower: for each text span, estimate approximate luminance well enough to classify the span as visible or color-hidden. This document maps the full PDF color space hierarchy onto that goal, identifying which constructs require active handling and which can be ignored.

## PDF Color Space Hierarchy

PDF organizes color spaces into three tiers with different implications for text extraction.

**Device-dependent spaces** — DeviceGray, DeviceRGB, and DeviceCMYK — map numeric values directly to output device primaries without calibration. These are the most common spaces for text coloring in practice. Because they carry no embedded profile, converting to luminance requires only a formula, not profile evaluation.

**Device-independent (calibrated) spaces** — CalGray, CalRGB, and Lab — embed a white point and gamma. CalGray is a single-channel calibrated space; CalRGB extends that to three channels with a matrix to XYZ; Lab encodes color in CIE L\*a\*b\* where L\* is perceptual lightness on a 0–100 scale. These spaces are fully resolvable to luminance without a renderer.

**Special spaces** — ICCBased, Indexed, Pattern, Separation, and DeviceN — add indirection. An ICCBased space wraps an embedded ICC profile with a declared alternate space as fallback. Indexed maps a single integer through a byte table to entries in a base space. Pattern replaces numeric color with a pattern dictionary. Separation and DeviceN represent spot inks with alternate-space approximations.

For text visibility detection, the relevant spaces are those that can appear as the current color when a text-showing operator executes. Pattern-filled text is extractable without special handling — character codes are decoded before any paint step. The color-relevant spaces for luminance estimation are: DeviceGray, DeviceRGB, DeviceCMYK, CalGray, CalRGB, Lab, ICCBased, Separation, and DeviceN. Halftone dictionaries, transfer functions, rendering intents, and color rendering dictionaries affect rasterized pixel output only and can be ignored entirely.

## ICC Profile Streams and the ICCBased Color Space

An ICCBased color space is declared as `[/ICCBased stream]` where the stream dictionary carries `/N` (number of components: 1, 3, or 4) and an `/Alternate` fallback space. The profile encodes a full colorimetric transform including a rendering intent — perceptual, relative colorimetric, saturation, or absolute colorimetric — that controls out-of-gamut mapping during device rendering.

For pdftract, rendering intent is irrelevant: it controls a rendering step that pdftract does not perform. What matters is recognizing the component count and the alternate space, then using the alternate for luminance estimation. If the alternate is DeviceRGB, treat the paint values as DeviceRGB. If the alternate is DeviceCMYK, apply the CMYK approximation. If no alternate is declared, infer from N (1 = gray, 3 = RGB, 4 = CMYK). Full ICC evaluation — loading the profile, building a transform chain — is not required for the luminance classification task.

## CMYK to Luminance Conversion

DeviceCMYK is common in professionally typeset documents. A full ICC transform from CMYK to Lab requires the printer profile, which is unavailable at extraction time. The practical approximation used by pdftract is:

```
L ≈ 1.0 - (C + M + Y + K * 0.25)   clamped to [0, 1]
```

The K weighting at 0.25 rather than 1.0 prevents all-K compositions (rich black with no CMY contribution) from being miscalculated when CMY channels are contributing lightness. This formula is not colorimetrically accurate, but it reliably identifies high-luminance CMYK values that would be nearly invisible on a white page versus low-luminance values with strong contrast. The use case is binary classification — visible versus color-hidden — not perceptual color reproduction, and the formula's accuracy for that binary task is sufficient.

## Lab Color Space: Direct Luminance Extraction

The CIE L\*a\*b\* space is the one case where luminance requires no conversion formula. L\* is perceptual lightness normalized to 0–100: 0 is perceptual black, 100 is the reference white. When text is painted in a Lab color space, the luminance estimate is directly `L* / 100`. Text with L\* above approximately 87 on a white background produces a contrast ratio below 1.5 and is classified as color-hidden. This makes Lab the lowest-cost space for pdftract's luminance pipeline — no transform required.

## Separation Color Spaces and Spot Color Luminance

Separation color spaces represent a single named colorant — a spot ink such as a Pantone value — defined by an alternate color space and a tinting function. The tinting function takes a single tint input (0 to 1) and returns a value in the alternate space.

For luminance estimation, pdftract evaluates the tinting function at the tint value specified in the paint operator and converts the result through the alternate-space formula. The most common alternate is DeviceCMYK; DeviceRGB alternates also occur. When the tinting function is a PostScript type 4 calculator function that cannot be evaluated at extraction time, pdftract extracts the span at full confidence and annotates it with `fill_type: spot_color_tint_unevaluated` rather than suppressing it. Dropping text because the spot alternate could not be resolved would produce false negatives.

## DeviceN: Multi-Ink Luminance Approximation

DeviceN generalizes Separation to N named colorants. Its tinting function takes N inputs (one per colorant) and returns a value in the alternate space. N can be as small as 1 (equivalent to Separation) or larger for multi-ink systems.

For luminance estimation, pdftract reads the N component values from the paint operator, evaluates the tinting function, and converts the alternate-space result to luminance. The same evaluation-failure fallback applies: if the function cannot be evaluated, extract with reduced confidence and annotate rather than suppress.

## Overprint Mode

Overprint is controlled by three graphics state parameters: `op` (overprint for non-stroking operations), `OP` (overprint for stroking), and `OPM` (overprint mode 0 or 1). Overprint mode 1, meaningful only in CMYK and DeviceN, suppresses painting for colorant channels whose value is zero so that underlying ink shows through.

For text extraction, overprint has no effect on character decoding. The practical implication for luminance estimation is narrow: a CMYK value of all zeros under OPM=1 would show underlying content in a renderer, but pdftract sees the tuple and computes luminance of 1.0 (white), correctly classifying the span as a potential color-hidden case. The overprint state is recorded in span metadata but does not alter extraction behavior.

## Halftone and Transfer Functions

Halftone screens and transfer functions are legacy print-production features. Halftone dictionaries specify screen angle, frequency, and spot function for each colorant, controlling how continuous-tone values convert to dot patterns on a physical print device. Transfer functions apply per-channel tone curves to compensate for press gain.

Neither affects digital display or text extraction. Character codes, positions, and color values are read from the content stream before any halftone or transfer processing. pdftract ignores both constructs entirely.

## Color in Type 3 Fonts

Type 3 fonts define glyphs through arbitrary PDF content streams. A glyph procedure can include color operators, painting sub-paths in different colors or using pattern fills within a single glyph. This makes Type 3 the one case where a single character may render in multiple colors.

Character codes and Unicode mappings are recovered from Type 3 fonts through the font's Encoding and ToUnicode entries exactly as with other font types. The colored rendering inside a glyph procedure does not affect character identity. However, luminance estimation cannot rely on the graphics state color when the glyph is a colored Type 3 (signaled by the `d0` operator at the glyph procedure start rather than `d1`). For these glyphs, pdftract records `fill_type: type3_colored` and does not apply the contrast check — executing the glyph procedure to sample its internal colors is beyond extraction scope. The character is extracted at full confidence.

## Extraction Implications: Implementing the color_hidden Flag

pdftract's luminance estimation resolves the current color space through a chain of fallbacks: ICCBased → alternate space → formula; Indexed → base space → formula; Separation → alternate via tinting function → formula; DeviceN → alternate via tinting function → formula; CalGray/CalRGB/Lab → direct or near-direct formula; DeviceGray/DeviceRGB/DeviceCMYK → direct formula.

When any step in the chain fails — missing alternate, unevaluable function, malformed profile — the fallback is to extract without a luminance estimate, annotate the span with the failure reason, and assign neutral confidence. The conservative direction is always to include rather than suppress.

The `color_hidden` flag is set when the computed contrast ratio between the estimated text luminance and the background luminance at the span's position falls below 1.5. This threshold covers white-on-white, light-gray-on-white, and analogous near-invisible cases. The flag does not suppress the span — character data, position, and font metadata are included in full. The goal throughout is not colorimetric accuracy but reliable binary classification: clearly visible versus potentially invisible. The approximations used — CMYK simplified formula, ICCBased alternate substitution, CalRGB treated as DeviceRGB — are calibrated for that binary task, not for reproducing the visual appearance of the document.
