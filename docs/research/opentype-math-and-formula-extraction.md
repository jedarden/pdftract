# OpenType MATH Table, Mathematical Formula Layout, and Formula Text Extraction

Mathematical formulas in PDFs occupy a unique position in the extraction problem space. Unlike prose text, which flows left-to-right and top-to-bottom in a predictable spatial sequence, formulas are two-dimensional structures where vertical position encodes meaning: a glyph shifted upward may be a superscript, a numerator, or an overscript, and distinguishing between them requires knowledge of the formula's internal geometry. This document covers the full path from the OpenType MATH table that describes how a font's mathematical characters are meant to be assembled, through the spatial heuristics needed to reconstruct formula structure, to the MathML output that captures that structure in a standard, machine-readable form.

---

## 1. The OpenType MATH Table

The `MATH` table, introduced in OpenType 1.8, is a mandatory component of any font designed for mathematical composition. Fonts such as Cambria Math, STIX Two Math, Latin Modern Math, and XITS Math all include it. The table is organized into four subtables that pdftract must parse whenever it encounters a math-capable font.

**MathConstants** is a record of 51 scalar metric values, each stored in design units or as a MathValueRecord (value plus device-adjustment hint). These constants define the global geometry of formula layout for the font:

- `AxisHeight` is the distance from the baseline to the math axis — the vertical center of operators, fraction bars, and most binary relations. This is approximately the height of the minus sign and equals one-half the x-height in typical text fonts.
- `ScriptPercentScaleDown` and `ScriptScriptPercentScaleDown` give the percentage of the base em-size to use for first-level and second-level scripts respectively — typically 71% and 58%.
- `FractionNumeratorDisplayStyleShiftUp`, `FractionNumeratorShiftUp`, `FractionDenominatorDisplayStyleShiftDown`, and `FractionDenominatorShiftDown` define how far numerator and denominator baselines are displaced from the math axis in display and inline styles.
- `FractionRuleThickness` gives the thickness of the fraction bar (vinculum). `FractionNumeratorGapMin` and `FractionDenominatorGapMin` define the minimum clearance between the vinculum and the adjacent glyph clusters.
- `SuperscriptShiftUp`, `SuperscriptShiftUpCramped`, `SubscriptShiftDown`, `SuperscriptBaselineDropMax`, and `SubscriptBaselineDropMin` govern script attachment — how far a superscript rises above or a subscript drops below the base glyph.
- `RadicalVerticalGap`, `RadicalDisplayStyleVerticalGap`, and `RadicalRuleThickness` describe the radical sign geometry.
- `UpperLimitGapMin`, `LowerLimitGapMin`, `UpperLimitBaselineRiseMin`, and `LowerLimitBaselineDropMin` control limit placement for large operators (∑, ∫, ∏).
- `OverbarVerticalGap`, `UnderbarVerticalGap`, `OverbarRuleThickness`, and `UnderbarRuleThickness` define overline and underline clearances.

**MathGlyphInfo** associates three kinds of per-glyph data with individual glyph IDs. The italic correction table records, for each slanted glyph, the horizontal distance by which its visual right edge overshoots the advance width — this correction must be applied when placing an accent or superscript immediately after the glyph. The top accent attachment table gives the x-coordinate, in design units, at which a combining accent should center itself above the base glyph, which may differ significantly from the glyph midpoint for asymmetric forms like integral signs. The extended shapes table flags glyphs that require special italic correction behavior when they appear as the base of a scripted expression.

**MathVariants** provides size escalation data for glyphs that must stretch to match the content they enclose: brackets, braces, parentheses, radical signs, integral signs, and extensible arrows. For each such base glyph, the table lists a sequence of prebuilt size variants (larger versions drawn at successively greater heights or widths) followed by a GlyphAssembly record describing how to construct an arbitrarily large version from component parts — a top piece, one or more extender pieces that tile to fill the required length, and a bottom piece. When pdftract encounters a sequence of adjacent glyphs in the content stream that are all identified as parts of a GlyphAssembly, those glyphs must be collapsed into a single logical delimiter rather than emitted as separate characters.

**MathKernInfo** provides per-glyph kerning tables specifically for script attachment positions. Where standard horizontal kerning adjusts spacing between adjacent glyphs on the same baseline, MathKernInfo allows fine control of the horizontal offset between a base glyph and its superscript or subscript, depending on the vertical position of the attachment point. Parsing MathKernInfo allows pdftract to confirm script associations that would otherwise be inferred purely from spatial position.

---

## 2. The Mathematical Formula Layout Model

Mathematical composition is governed by a set of rules first codified by Donald Knuth's TeX typesetting system and now formalized in the OpenType MATH specification. Understanding these rules is prerequisite for correct formula reconstruction.

The **math axis** is a horizontal reference line at height `AxisHeight` above the text baseline. Binary operators (=, +, −, ×) and fraction bars are vertically centered on the math axis. This means that the baseline of a formula sits lower than the visual center of its operators. When identifying formula regions on a page, the math axis is the appropriate alignment reference, not the glyph baseline.

**Fractions** are composed by placing a numerator cluster above the math axis and a denominator cluster below, separated by the vinculum — a horizontal rule of thickness `FractionRuleThickness` drawn at the math axis height. The numerator baseline is shifted up from the math axis by `FractionNumeratorShiftUp` (inline) or `FractionNumeratorDisplayStyleShiftUp` (display); the denominator baseline shifts down symmetrically. Detection in a PDF content stream proceeds by locating a horizontal rule (a filled rectangle or a `l` path operator with vertical extent near zero) at a height consistent with the math axis, then classifying all glyphs above it that horizontally overlap it as numerator content, and all glyphs below as denominator content.

**Subscripts and superscripts** are attached glyphs rendered at a reduced font size — `ScriptPercentScaleDown` percent of the base size — with a shifted baseline. Superscripts have a positive text rise (Ts) of approximately `SuperscriptShiftUp` in scaled font units; subscripts have a negative text rise of approximately `SubscriptShiftDown`. The base glyph and its attached scripts together form a scripted expression: the base determines horizontal extent and the scripts are positioned relative to its right edge, with the x-offset refined by MathKernInfo if available.

**Under- and overscripts** attach limits to large operators. In display style, the summation limit `n=1` renders directly below the ∑ glyph and `∞` directly above it, horizontally centered. In inline style the same limits appear as subscript and superscript. The distinction between the two attachment modes is determined by display context (presence of surrounding vertical space) and by the `MoveLimits` flag on the glyph.

**Radical structures** consist of the radical sign glyph (or its assembled multi-part form from MathVariants), a horizontal bar extending from the top of the radical sign to cover the radicand, and the radicand glyph cluster itself positioned beneath the bar. An optional index argument (the `n` in ⁿ√) appears above and to the left of the radical sign, shifted up by `RadicalDegreeBottomRaisePercent` of the radical height.

---

## 3. Text Extraction from Formula Glyphs

Formula glyphs are regular PDF glyph instructions — they appear in the content stream as Tj, TJ, or similar text-showing operators, drawn with the same graphics state machinery as prose text. For fonts with correct ToUnicode CMaps (as produced by XeLaTeX, LuaLaTeX with unicode-math, or MathType), the characters decode to Unicode Mathematical Alphanumeric Symbols and operator code points in the standard way.

The distinctive challenge is **reading order**. A PDF content stream records glyphs in the order the layout engine placed them, which for formulas is not the logical expression order. TeX, for instance, places base glyphs in left-to-right baseline order, then goes back and places the subscripts and superscripts for each base. An integral expression like `∫₀¹ f(x) dx` might appear in the stream as: ∫, then 0 (subscript), then 1 (superscript), then f, (, x, ), d, x — not as a simple left-to-right sequence. Passing these code points through naively produces character soup.

pdftract must therefore reconstruct reading order from spatial position, not stream position. The algorithm is: identify the math axis height for the expression, cluster glyphs by their horizontal proximity and vertical script level, then serialize the clusters in the order: base glyph, then any subscript, then any superscript — mirroring the logical order expected by a formula parser.

Invisible glyphs (those with zero width, used to convey semantic grouping in some TeX-produced PDFs) must be suppressed before spatial analysis, as they introduce false cluster boundaries.

---

## 4. Fraction Detection in Vector Text

A fraction consists of three geometric components: a numerator cluster, a vinculum, and a denominator cluster. The vinculum may be rendered as a filled rectangle (a `re` path operator followed by `f` or `B`), as a horizontal lineto sequence (`m x0 y0 l x1 y0 S`), or as an extended glyph from OMX/MathVariants (the `fraction` or `radicalex` glyph drawn at a large width).

Detection proceeds as follows. After extracting all path operations on the page, identify horizontal rules: path segments or filled rectangles whose height is less than two points and whose width exceeds their height by a factor of at least ten. For each candidate rule, check whether its y-coordinate falls within `FractionRuleThickness * 2` of the local math axis estimate (derived from nearby text runs). Collect all text glyphs whose bounding boxes horizontally overlap the rule and whose baselines are above the rule — these form the numerator. Collect those below — these form the denominator.

The logical output of a detected fraction is either a flat Unicode approximation `numerator/denominator` for prose contexts, or an `mfrac` element in MathML:

```xml
<mfrac>
  <mrow><!-- numerator glyphs --></mrow>
  <mrow><!-- denominator glyphs --></mrow>
</mfrac>
```

When the numerator or denominator itself contains a fraction, the structure is recursively nested.

---

## 5. Subscript and Superscript Reconstruction

The primary signal for script classification is the Tm (text matrix) component that establishes glyph baseline position. In PDF, the text rise parameter Ts shifts the glyph vertically relative to the current text line without advancing the baseline. A positive Ts combined with a font size that is `ScriptPercentScaleDown` percent of the surrounding body text strongly indicates a superscript; a negative Ts with the same size ratio indicates a subscript.

Secondary signals: (1) if the glyph's rendered y-position departs from the dominant baseline of the current text run by more than half an x-height, it is a script; (2) the script font name may include a variant identifier (e.g., `cmmi7` versus `cmmi10` in pdfLaTeX output), where the size suffix directly encodes the design size.

Reconstruction produces MathML scripted elements. A base glyph with a superscript becomes `msup`, a base with a subscript becomes `msub`, and a base with both becomes `msubsup`. Under- and overscripts use `munder`, `mover`, and `munderover`. The base token element (`mi`, `mn`, `mo`) is determined by the Unicode category of the base glyph: letter-class code points produce `mi`, digit code points produce `mn`, and operator-class code points produce `mo`.

When multiple script levels are present — a superscript to a superscript — the nesting must be reconstructed from the glyph position hierarchy. Each level of superscript reduces the font size by `ScriptPercentScaleDown`; second-level scripts are therefore approximately `(0.71)² ≈ 0.50` of the body size.

---

## 6. TeX Math Encoding Issues

pdfLaTeX embeds Computer Modern Math fonts using three legacy TeX encodings — OML (math italic, `cmmi` fonts), OMS (math symbols, `cmsy` fonts), and OMX (math extension, `cmex` fonts) — without generating ToUnicode CMaps. This is the dominant encoding challenge in scientific PDF extraction, as the majority of academic papers are still produced by pdfLaTeX.

The recovery path is described in detail in `latex-and-scientific-pdf-patterns.md`. Additional symbols requiring explicit mapping that are not covered there:

**OMS supplementary operators** (slots 0x10–0x3F): 0x10 = U+2261 (≡ identical to), 0x11 = U+2264 (≤), 0x12 = U+2265 (≥), 0x13 = U+221C (∜ fourth root, use U+221A fallback), 0x14 = U+2234 (∴ therefore), 0x15 = U+2235 (∵ because), 0x20 = U+2213 (∓ minus-or-plus), 0x21 = U+2295 (⊕), 0x22 = U+2296 (⊖), 0x23 = U+2297 (⊗), 0x24 = U+2298 (⊘), 0x25 = U+2299 (⊙), 0x40 = U+2022 (• bullet), 0x7E = U+2243 (≃ asymptotically equal).

**OMX large operator slots**: 0x00 = U+0028 large `(`, 0x01 = U+0029 large `)`, 0x02 = U+005B large `[`, 0x03 = U+005D large `]`, 0x04 = U+230A ⌊, 0x05 = U+230B ⌋, 0x06 = U+2308 ⌈, 0x07 = U+2309 ⌉, 0x08 = U+007B large `{`, 0x09 = U+007D large `}`, 0x0A = U+2329 ⟨ (or U+27E8), 0x0B = U+232A ⟩ (or U+27E9), 0x0C–0x0F = extensible bar parts (discard as structure), 0x10 = U+222B (∫), 0x11 = U+222E (∮), 0x50 = U+2211 (∑), 0x51 = U+220F (∏), 0x58 = U+222B display-size (treat as U+222B), 0x59 = U+222E display-size.

OMX extender glyphs (top/middle/bottom pieces of brackets and radicals) should be consumed during GlyphAssembly reconstruction and not emitted as individual characters. Any OMX glyph in slots 0x20–0x4F that is not matched as part of an assembly is a standalone large symbol; consult the cmex glyph name list to determine the Unicode fallback.

---

## 7. MathML as the Target for Structured Formulas

MathML 3.0 is the standard output format for formula structure recovered by pdftract. It separates token elements (carrying character data) from layout elements (describing spatial relationships).

Token elements map to glyph clusters by Unicode category and context: `mi` for identifiers (single letters in math italic, named functions like `sin`, `log`), `mn` for numeric literals (digit sequences, possibly with decimal point), `mo` for operators (characters in Unicode category Sm or Po when in operator position), and `mtext` for upright text appearing within a formula (units, labels).

Layout elements correspond to the spatial structures identified during reconstruction:

- `mfrac` wraps the numerator and denominator of a detected fraction.
- `msqrt` wraps the radicand of a square root; `mroot` wraps both the radicand and the index argument of an nth root.
- `msub`, `msup`, `msubsup` wrap base and script pairs.
- `munder`, `mover`, `munderover` wrap large operator bases with their limits.
- `mrow` groups any sequence of logically associated tokens (the content of brackets, the body of a radical, numerator or denominator).
- `mo` with the `stretchy` attribute represents delimiters that scale to match their content; pdftract sets `stretchy="true"` on any delimiter identified as assembled from GlyphAssembly parts.

The root element is always `<math xmlns="http://www.w3.org/1998/Math/MathML" display="block">` for display formulas or `display="inline"` for inline. The document's formula output should be valid against the MathML 3.0 schema to allow downstream consumers — screen readers, search indexers, computer algebra systems — to parse it without modification.

---

## 8. Alternative Text for Formulas in Tagged PDFs

Tagged PDFs generated by accessible authoring workflows may include formula descriptions that render all spatial reconstruction unnecessary. The PDF structure tree `/Formula` element may carry:

- `/Alt` — a plain-text alternative, often a spoken description or a simplified linearization (e.g., "x squared plus y squared equals r squared").
- `ActualText` — a string intended to be read in place of the rendered content, sometimes containing LaTeX source or a Unicode linearization.
- An `/AF` (associated file) entry pointing to an embedded file stream with MIME type `application/mathml+xml`, containing well-formed MathML.

When any of these alternatives is present, pdftract must prefer it over geometric reconstruction. The extraction pipeline should check for `ActualText` first (authoritative by the PDF specification), then `/AF` with MathML content, then `/Alt` as a text fallback. Only in the absence of all three should the spatial reconstruction path activate. This ordering avoids wasting computation on formulas that are already fully described.

---

## 9. LaTeX Source Recovery

Some PDF generators embed the formula source in machine-readable metadata, enabling exact recovery without reconstruction.

**hyperref bookmarks**: The hyperref package encodes section headings in the PDF outline using UTF-16BE strings. For mathematical section titles, the string contains the LaTeX representation of the formula (e.g., `Proof of the \( O(n \log n) \) Bound`). These strings appear in the `/Outlines` dictionary and are available without font decoding.

**MathJax-rendered PDFs**: PDFs produced by printing a MathJax-rendered HTML page may carry XMP metadata in the `/Metadata` stream. The `x:xmpmeta` block in some MathJax configurations includes `<mml:math>` fragments corresponding to the formulas rendered on the page. Parsing the raw XMP XML allows recovery of the original MathML before any PDF rendering loss.

**OpenDocument-based PDFs**: LibreOffice's PDF export embeds an ODF content stream in the `/EmbeddedFiles` array for some configurations. ODF formulas are stored in OpenDocument Formula (ODF Formula) format, which is a subset of MathML. When an embedded ODF file is present, extract formula elements from the `content.xml` member of the ODF ZIP archive.

**XMP Dublin Core `description` field**: Some equation editor plugins (MathType for Word) write a LaTeX or AsciiMath representation of each formula into the XMP `dc:description` field of the associated image or annotation object. Inspect XMP streams on Form XObjects that are identified as formula regions.

These recovery paths are opportunistic — not all PDFs will have any of them — but each should be attempted before engaging the computational reconstruction pipeline. When source recovery succeeds, the confidence score for the formula is set to 1.0 regardless of whether the formula could have been reconstructed geometrically.

---

## 10. Confidence and Fallback

Formula extraction confidence is a composite score reflecting the proportion of glyphs with clean Unicode mappings, the availability and parsability of MATH table data, the geometric clarity of the expression structure (absence of overlapping bounding boxes, consistent size ratios), and the number of unresolved GlyphAssembly components. The score is computed per formula and drives the output strategy.

Formulas that cannot be reliably reconstructed must be emitted, not silently dropped. A formula that produces no output creates a semantic gap in the extracted text that is invisible to downstream consumers and unrecoverable. The fallback output format is a `kind: formula` block with the raw Unicode glyph sequence in stream order, the bounding box, and `confidence` below the reconstruction threshold:

```json
{
  "kind": "formula",
  "glyphs": "∫₀¹ f ( x ) dx",
  "mathml": null,
  "confidence": 0.42,
  "bbox": { "page": 2, "x0": 180.0, "y0": 430.0, "x1": 295.0, "y1": 450.0 }
}
```

The `glyphs` field preserves every decoded character in stream order so that downstream tools — computer algebra systems, LLM post-processors, or human reviewers — can attempt further parsing with full information. When even glyph-level decoding fails (Type 3 fonts with no Unicode recovery, OMX extender glyphs with no Unicode equivalent), the field is populated with Unicode REPLACEMENT CHARACTER (U+FFFD) placeholders in glyph count, preserving the character count for layout correlation.

The minimum requirement is that every formula region identified on the page produces an output block. Formula content is never silently omitted.

---

## 11. Formula-Region Detection Algorithm

Formula-region detection operates on spans flagged during font recognition as using a math-capable font (any font with a MATH table). The algorithm aggregates these spans into contiguous formula regions using proximity and geometric consistency.

### Pseudo-code

```
function detect_formula_regions(spans: Vec<Span>, page_metrics: PageMetrics) -> Vec<FormulaRegion> {
    let mut regions: Vec<FormulaRegion> = vec![];
    let mut candidate_spans: Vec<Span> = spans
        .iter()
        .filter(|s| s.font.has_math_table())
        .cloned()
        .collect();

    // Sort by y (top to bottom), then by x (left to right)
    candidate_spans.sort_by(|a, b| {
        a.bbox.y0.partial_cmp(&b.bbox.y0)
            .unwrap()
            .then(a.bbox.x0.partial_cmp(&b.bbox.x0).unwrap())
    });

    let mut current_region: Option<FormulaRegion> = None;
    let math_axis = estimate_math_axis(&candidate_spans, page_metrics);

    for span in candidate_spans {
        if let Some(ref region) = current_region {
            // Check if span belongs to current region
            let vertical_gap = span.bbox.y0 - region.bbox.y1;
            let horizontal_overlap = spans_horizontally_overlap(&span.bbox, &region.bbox);
            let baseline_aligned = (span.baseline - math_axis).abs() < (page_metrics.x_height * 0.3);

            if vertical_gap < page_metrics.line_height * 0.5
                && (horizontal_overlap || baseline_aligned)
            {
                // Extend current region
                region.add_span(span);
            } else {
                // Finalize current region and start new one
                regions.push(current_region.take().unwrap());
                current_region = Some(FormulaRegion::new(span));
            }
        } else {
            current_region = Some(FormulaRegion::new(span));
        }
    }

    if let Some(region) = current_region {
        regions.push(region);
    }

    regions
}

function estimate_math_axis(spans: &[Span], page_metrics: PageMetrics) -> f32 {
    // Compute median baseline of math-font spans as math axis estimate
    let mut baselines: Vec<f32> = spans.iter().map(|s| s.baseline).collect();
    baselines.sort_by(|a, b| a.partial_cmp(b).unwrap());
    baselines[baselines.len() / 2]
}
```

### Region properties

Each `FormulaRegion` contains:
- `spans`: All constituent text spans
- `bbox`: Union of all span bounding boxes
- `math_axis`: Computed axis height for this region
- `has_display_style`: True if region is centered and offset from surrounding text (see Section 12)
- `confidence`: Composite score based on glyph coverage, MATH table parsability, and geometric clarity

Formula regions are atomic for XY-cut purposes: the reading-order algorithm treats an entire formula as a single block and does not split it across columns.

---

## 12. Inline vs Display Formula Classification

The distinction between inline and display formulas determines the output block kind and affects reading-order placement.

### Display formula detection

A formula region is classified as **display** if ANY of the following conditions hold:

1. **Centering**: The region's horizontal center is within 5% of the page's horizontal center AND the region's width is < 80% of the column width.
2. **Vertical spacing**: The vertical gap above AND below the region exceeds `1.5 * line_height` of the surrounding body text.
3. **Font size**: The median font size of the region exceeds `1.2 * body_font_size` (display-style operators are larger).

### Inline formula detection

A formula region is classified as **inline** if:

1. The region appears within a paragraph (vertical gap < `0.5 * line_height` on both sides).
2. The region's horizontal extent does not exceed the column width by more than 10%.
3. The font size is within ±20% of the surrounding body text.

### Output representation

- **Display formulas**: Emitted as `kind: formula` with `display: "block"` in MathML output. Surrounded by blank lines in plain text output.
- **Inline formulas**: Emitted as `kind: formula` with `display: "inline"` in MathML output. Integrated into paragraph text flow in plain text output.

### Edge cases

- **Display formulas within multi-column layouts**: If a formula spans across column boundaries (width > column width), it is classified as display and assigned to the dominant column based on horizontal center position.
- **Offset inline formulas**: Some academic journals offset inline formulas by 0.5em for emphasis. These are NOT display formulas if the vertical spacing condition fails. Detection uses the font-size signal as the primary discriminator.

---

## 13. LaTeX-Like Reconstruction (Best-Effort)

Reconstructing LaTeX source from formula glyphs is fundamentally lossy. PDF rendering discards semantic information: `\frac{a}{b}` and `\dfrac{a}{b}` produce identical glyph sequences, and `\left( \right)` vs `\bigl( \bigr)` are indistinguishable after font substitution. This section describes a best-effort reconstruction that produces plausible LaTeX when confidence is high, falling back to Unicode text otherwise.

### Reconstruction algorithm

The reconstruction proceeds in three passes:

1. **Structure detection**: Identify fractions, scripts, radicals, and large operators using the spatial heuristics in Sections 4–5.
2. **LaTeX template selection**: Map each detected structure to a LaTeX template:
   - Fraction → `\frac{<numerator>}{<denominator>}`
   - Superscript → `^{<superscript>}`
   - Subscript → `_{<subscript>}`
   - Radical → `\sqrt{<radicand>}`
   - Large operator with limits → `\sum_{<lower>}^{<upper>}` or `\sum\nolimits` depending on display context
3. **Token serialization**: Convert each glyph cluster to a LaTeX token:
   - Math italic letters → single letter (e.g., `x`, `A`)
   - Function names → backslash-prefixed name (e.g., `\sin`, `\log`)
   - Operators → operator command (e.g., `\times`, `\cdot`, `\leq`)
   - Greek letters → backslash-prefixed name (e.g., `\alpha`, `\Gamma`)

### Confidence scoring

LaTeX reconstruction confidence is computed as:

- `structure_confidence`: Fraction of glyphs successfully assigned to a structure (0.0–1.0)
- `unicode_coverage`: Fraction of glyphs with clean Unicode mappings (excluding U+FFFD)
- `math_table_quality`: 1.0 if MATH table present and parsable, 0.5 if present but malformed, 0.0 if absent

Overall `latex_confidence = 0.5 * structure_confidence + 0.3 * unicode_coverage + 0.2 * math_table_quality`

### Output format

- **High confidence** (`latex_confidence >= 0.7`): Emit `metadata.formula_latex` field with reconstructed LaTeX string.
- **Medium confidence** (`0.4 <= latex_confidence < 0.7`): Emit `metadata.formula_latex` with an `(experimental)` comment prefix and set `metadata.formula_latex_confidence`.
- **Low confidence** (`latex_confidence < 0.4`): Do NOT emit LaTeX. Emit Unicode text only in `text` field.

### Feature flag

The LaTeX reconstruction path is gated behind the `formula_latex` feature flag. When disabled (default for v1.0), the extractor skips structure detection and emits only the raw Unicode glyph sequence. This reduces binary size and eliminates the maintenance burden of LaTeX template mapping until the algorithm is validated against a large corpus.

### Fallback example

Input (glyphs with positions):
```
∫ at (100, 200), size 12
0 at (110, 190), size 8, Ts=-10 (subscript)
1 at (120, 215), size 8, Ts=+10 (superscript)
f at (135, 200), size 12
( at (145, 200), size 12
x at (152, 200), size 12
) at (160, 200), size 12
```

High-confidence LaTeX output:
```latex
\int_{0}^{1} f(x)
```

Low-confidence fallback (Unicode text):
```
∫₀¹ f(x)
```

---

## 14. Profile Classifier Signal: `structural.has_math`

The profile classifier uses the `structural.has_math` boolean signal to identify scientific papers and distinguish them from other document types (e.g., a book chapter with inline equations vs a research article).

### Signal definition

`structural.has_math = true` when ANY of the following conditions are met across the first 5 pages of the document:

1. **OpenType Math detection**: At least one span uses a font with a MATH table (detected during Phase 2 font recognition).
2. **Formula region count**: At least 3 formula regions are detected (using the algorithm in Section 11) with confidence >= 0.5.
3. **Math operator density**: The document contains >= 10 distinct mathematical operator Unicode code points from the set {∑, ∫, ∏, √, ±, ≠, ≤, ≥, ∈, ∉, ∪, ∩, ⊂, ⊃}.

### Rationale

The three-condition definition handles the major PDF production workflows:

- **Modern LaTeX (XeLaTeX, LuaLaTeX)**: Uses Unicode math fonts with MATH tables. Condition 1 fires.
- **Legacy pdfLaTeX**: Uses OML/OMS/OMX encoding without MATH tables but produces recognizable operators. Condition 2 or 3 fires.
- **Word/MathType**: Embeds equation objects with operator glyphs. Condition 3 fires.

### False positive mitigation

To avoid misclassifying a finance document with currency symbols as a math paper:

- Currency symbols ($, €, £, ¥) are excluded from the operator code point set.
- The formula region count threshold (3) filters out documents with incidental math notation (e.g., a single equation in an annual report).
- The page range limit (first 5 pages) prevents back-matter references from triggering the signal.

### Usage in built-in profiles

The `scientific_paper` profile (plan line 3060) uses `structural.has_math` as a strong positive signal. A document with `structural.has_math = true`, headings matching academic paper patterns (Abstract, Introduction, References), and PDF metadata indicating a journal source is classified as `scientific_paper` with confidence >= 0.8.

---

## 15. Validation Methodology

Formula extraction validation requires a fixture corpus of scientific papers with known equations. The validation methodology measures both geometric reconstruction accuracy and semantic correctness.

### Fixture corpus

- **Source**: arXiv preprints in PDF format with accompanying LaTeX source (available via arXiv's "source" download).
- **Size**: Minimum 50 papers spanning physics, mathematics, and computer science.
- **Selection criteria**: Each paper must contain at least 5 display equations and 10 inline equations.
- **Ground truth**: Extracted from LaTeX source by stripping non-math content and parsing equation environments (equation, align, gather, inline $...$).

### Metrics

1. **Formula-region detection**: Precision, recall, F1 for identifying formula regions (bounding box overlap > 0.8 IoU counts as detection).
2. **Structure reconstruction**: Percentage of formulas with correct structure (fraction, script, radical) compared to LaTeX AST.
3. **Unicode accuracy**: Character-level accuracy of the extracted Unicode text against the LaTeX-rendered Unicode.
4. **LaTeX reconstruction** (when `formula_latex` feature is enabled): Levenshtein distance between reconstructed LaTeX and source LaTeX, normalized by length.

### CI gate

A WER (Word Error Rate) delta gate is added to the `BrokenVector` fixture set (bead `pdftract-48ea`). The gate compares the current extraction's character error rate against the baseline. If the delta exceeds +0.5%, the CI fails and blocks the PR. This gate specifically catches regressions in formula extraction that corrupt non-formula text (e.g., OMX extender glyphs leaking into output).

### Test infrastructure

```rust
#[test]
fn test_formula_region_detection_arxiv_sample() {
    let pdf = std::fs::read("tests/fixtures/arxiv_sample.pdf").unwrap();
    let result = extract(&pdf).unwrap();
    assert!(result.metadata.structural.has_math);
    assert!(result.pages[0].blocks.iter().filter(|b| b.kind == BlockKind::Formula).count() >= 5);
}

#[test]
fn test_formula_mathml_output_validates() {
    let pdf = std::fs::read("tests/fixtures/arxiv_sample.pdf").unwrap();
    let result = extract(&pdf).unwrap();
    let formula_block = result.pages[0].blocks.iter()
        .find(|b| b.kind == BlockKind::Formula).unwrap();
    let mathml = formula_block.mathml.as_ref().unwrap();
    // Validate against MathML 3.0 schema
    assert!(validate_mathml(mathml).is_ok());
}
```

### Ongoing validation

As the fixture corpus grows, the validation metrics are tracked over time in the benchmark suite (plan lines 3127–3131). Each commit records the formula-region F1 and LaTeX reconstruction accuracy, allowing detection of regressions before they reach users.
