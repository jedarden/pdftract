# Mathematical Expression Handling in pdftract

## Overview

Mathematical notation in PDFs does not follow a single encoding scheme. Depending on the authoring tool and font stack, the same rendered equation may be stored as structured XML, as a sequence of Unicode code points from specialized fonts, as legacy symbol-mapped glyphs, as a raster image, or as procedural vector drawing instructions. A robust extraction library must detect which encoding is present, apply the appropriate recovery path, and produce a normalized structured output. This document specifies each encoding case, the algorithms for handling it, and the output representation.

---

## 1. How Math Is Encoded in PDF

### (a) MathML in Tagged PDF StructTree

PDF/UA-compliant documents and Word-exported PDFs with the "Save as PDF" accessibility option may embed MathML directly in the logical structure tree. The `StructTree` dictionary contains `Formula` structure elements whose `ActualText` or associated file attachment holds a well-formed MathML fragment. The extraction path is unambiguous: walk the `StructTree`, locate `Formula` nodes, extract the `ActualText` string or the associated `AF` file stream, and validate the XML. No font decoding is needed.

### (b) OpenType Math Fonts with Correct ToUnicode

Authoring tools that target Unicode-native math (MathType, LibreOffice, newer LaTeX engines with `lualatex`/`xelatex`) embed OpenType fonts such as STIX Two Math, Latin Modern Math, or Cambria Math and include correct `ToUnicode` CMap entries. Glyphs map directly to Unicode Mathematical Alphanumeric Symbols (U+1D400–U+1D7FF) and operator blocks. The PDF content stream is legible at the character level; the challenge is spatial reconstruction — determining which glyphs form a numerator, denominator, superscript, or radical argument.

### (c) Legacy TeX/LaTeX Output with Computer Modern or AMS Fonts

`pdflatex` and `dvips`-produced PDFs embed Type1 or Type2 fonts in legacy TeX encodings. These fonts carry no `ToUnicode` entries, or carry entries that map to PUA code points. The CM family uses OT1 encoding for text, OML for math italic, OMS for math symbols, and OMX for large operators and delimiters. Recovery requires consulting the encoding vector, which maps slot numbers to glyph names, then resolving those glyph names to Unicode via the Adobe Glyph List extended with TeX-specific names.

### (d) Math as Embedded Raster Images

Word processors and equation editors sometimes render complex expressions to a bitmap and embed it as an image XObject (`/Subtype /Image`) in the content stream. EPS figures included via `\includegraphics` appear as Form XObjects. In these cases no character data is recoverable from the content stream. Detection relies on aspect ratio, position within a text block, and the absence of any text operators in the surrounding XObject. The extraction fallback is to crop the rendered page image at the object's bounding box and encode it as base64.

### (e) Math as Type 3 Fonts with Arbitrary Glyph Procedures

Type 3 fonts define each glyph as a PDF content stream of drawing commands. Some equation editors and older TeX backends embed math characters this way. The glyph streams contain no semantic information — only `m`, `l`, `c`, and fill operators. Recovery is strictly visual: render each glyph to a small bitmap and run it through a shape classifier. Given the cost, Type 3 math is best treated as a raster fallback after an attempt to match glyph bitmaps against a reference atlas of common math symbols.

---

## 2. The OpenType MATH Table

The `MATH` table (introduced in OpenType 1.8) is the canonical source of math layout metadata for fonts like Cambria Math and STIX Two Math. It contains three subtables.

**MathConstants** holds 51 scalar values (in font design units) that govern layout: `ScriptPercentScaleDown` and `ScriptScriptPercentScaleDown` give the em-size ratios for script and script-script levels; `FractionNumeratorDisplayStyleShiftUp` and `FractionDenominatorGapMin` control fraction layout; `RadicalVerticalGap` and `RadicalRuleThickness` describe radical construction; `UpperLimitGapMin` and `LowerLimitGapMin` cover large operator limits.

**MathGlyphInfo** associates per-glyph metadata with specific glyph IDs: italic correction (the horizontal overhang of an italic glyph, used for correct accent placement), top accent attachment points (the x-coordinate at which a combining accent centers itself over the base glyph), and extended shape flags (marking glyphs that require special italic correction behavior).

**MathVariants** maps base glyph IDs to size variants and glyph construction recipes. Each extensible glyph — a bracket, brace, radical sign, integral, or arrow — has a list of prebuilt size variants followed by a `GlyphAssembly` that describes how to assemble an arbitrary-height or arbitrary-width version from parts (a start piece, one or more extender pieces that repeat, and an end piece). Parsing `MathVariants` allows pdftract to recognize that a sequence of component glyphs in the content stream constitutes a single large delimiter or radical, rather than treating each piece as an independent character.

Inference from the MATH table: if a glyph's bounding box places it above the current baseline by more than `SuperscriptShiftUp` and its font size is within `ScriptPercentScaleDown` of the enclosing font size, classify it as a superscript argument. Similar logic applies to subscripts, fractions, and radicals.

---

## 3. Symbol Font Encoding Recovery

Legacy TeX fonts use four encoding vectors relevant to math:

- **OT1** — 128 slots, mostly Latin; slot 0x0B is `\beta` in math mode due to glyph sharing.
- **OML** — 128 slots of math italic: lowercase and uppercase Latin italic, Greek lowercase, and special math glyphs.
- **OMS** — 128 slots of math symbols: operators, relations, arrows.
- **OMX** — 128 slots of large operators and extensible delimiters; many glyphs are halves or extenders.

Recovery procedure: (1) extract the font's encoding array from the Type1 `Encoding` dictionary or the `cmap` subtable; (2) map slot numbers to glyph names using the encoding vector; (3) look up glyph names in an augmented glyph-name-to-Unicode table that covers TeX-specific names (`arrowlefttophalf`, `bracketleftbt`, etc.) and the AGLFN; (4) for slots that resolve to PUA or remain unmapped, cross-reference the font's `CharStrings` dictionary name against a compiled symbol atlas.

Unicode Mathematical Alphanumeric Symbols (U+1D400–U+1D7FF) provide distinct code points for mathematical italic, bold, script, fraktur, double-struck, monospace, and sans-serif variants of Latin and Greek letters. When a glyph name resolves to a plain Latin letter but the enclosing font is identified as a math italic font (via the font name containing `MathItalic`, `CMMI`, or `OML`), remap to the corresponding U+1D4xx italic code point.

Font identification heuristics: a font is a math font if its name matches known math font families (`cmsy`, `cmex`, `cmmi`, `msam`, `msbm`, `esint`, `stmary`, `txsy`, `pxsy`), or if its `FontDescriptor` `Flags` field has bit 6 (Symbolic) set alongside an encoding with more than 20% glyph-name matches against the math symbol glyph atlas.

---

## 4. Spatial Heuristics for Expression Detection

**Inline vs. display math.** Display math is centered on the page (horizontal center within 5% of page width) and surrounded by inter-paragraph vertical gaps larger than the prevailing line spacing. Inline math shares the baseline grid of surrounding text runs and has no exceptional vertical gap.

**Superscript and subscript detection.** A glyph run is a superscript if its baseline offset from the enclosing line's baseline is positive and falls within the range `[SuperscriptShiftUp * 0.5, SuperscriptShiftUp * 1.5]` (in scaled font units). Subscripts shift negative by an analogous range. A secondary check compares the font size: script-level glyphs are typically 60–71% of the base size.

**Grouping into expression trees.** After baseline classification, group glyphs using a modified connected-components pass: two glyphs belong to the same expression if their bounding boxes overlap on the horizontal axis or are separated by less than one em-width, and they share a common ancestor in the script-level hierarchy. Radical constructs are detected by locating an OMX radical glyph followed by a horizontal bar (`radicalex`) and grouping all glyphs under the bar into the radicand argument. Fraction structures are detected by a horizontal rule glyph with two vertically separated glyph groups straddling it.

**Bracket matching.** Opening delimiter glyphs from OMX or MathVariants are matched to their closing counterparts by tracking a depth counter. Assembled delimiters (multi-part from GlyphAssembly) are collapsed to a single logical delimiter before matching.

---

## 5. MathML in Tagged PDFs

The extraction path for tagged PDFs proceeds as follows. Parse the `StructTreeRoot` from the document catalog. Traverse the structure tree depth-first, collecting nodes with `/S /Formula`. For each `Formula` node, inspect the `A` (attribute) dictionary and the `AF` (associated files) array. MathML may appear as:

- A UTF-16BE string in `ActualText` — decode to UTF-8 and parse as XML.
- A file specification in `AF` with `AFRelationship /Supplement` and a MIME type of `application/mathml+xml` — decompress the embedded stream and parse.

Validate the extracted XML against the MathML 3 schema subset. Common defects in Word-exported MathML include missing namespace declarations, `mfenced` elements with non-standard separators, and empty `mrow` wrappers. Apply a normalization pass: add the `xmlns` attribute if absent, replace `mfenced` with explicit `mo` delimiters and `mrow`, and strip empty `mrow` elements.

---

## 6. LaTeX Reconstruction

When the source is glyph sequences (cases b and c) rather than embedded MathML, LaTeX reconstruction proceeds in two phases.

**Phase 1 — symbol mapping.** Map each Unicode math code point (after encoding recovery) to a LaTeX command string using a compiled lookup table covering the full Unicode math range: U+2200–U+22FF (mathematical operators), U+27C0–U+27EF (supplemental arrows), U+1D400–U+1D7FF (alphanumerics), and AMS extension blocks. Characters with multiple LaTeX representations (e.g., U+2212 `−` mapping to both `\minus` and `-`) prefer the representation appropriate to context (operator position).

**Phase 2 — structure reconstruction.** Apply the expression tree from the spatial heuristics pass: superscript groups become `^{...}`, subscripts become `_{...}`, fraction numerators and denominators become `\frac{num}{denom}`, radicands become `\sqrt{...}` (or `\sqrt[n]{...}` if an index argument is detected above the radical glyph), and integral glyphs with limit arguments become `\int_{...}^{...}`. Delimiter pairs from bracket matching become `\left( ... \right)` using the appropriate delimiter command.

Limitations: reconstruction is heuristic and degrades for deeply nested structures, for multi-line display environments (`align`, `cases`), and for any glyph that has no Unicode mapping. Confidence decreases with nesting depth and increases with the proportion of glyphs that resolve cleanly to Unicode.

---

## 7. Fallback Strategies

Fallbacks are selected based on a per-expression confidence score (0.0–1.0) computed from: fraction of glyphs with clean Unicode mappings, availability of MATH table data, presence of MathML in StructTree, and structural ambiguity (unmatched delimiters, zero-width gaps suggesting missing glyphs).

| Confidence | Output Strategy |
|---|---|
| ≥ 0.85 | Full LaTeX and/or MathML reconstruction |
| 0.60–0.84 | Unicode math string only (`unicode` field populated, `latex` omitted) |
| 0.30–0.59 | Placeholder `[MATH]` with bounding box; Unicode field if partially recoverable |
| < 0.30 | Base64 image crop of the rendered expression region; all text fields omitted |

Image crops are produced by rendering the page to a raster at 150 DPI (sufficient for readability) and cropping to the expression bounding box with 4-point padding on each side.

---

## 8. Output Representation

Math blocks appear as JSON objects in the extraction output with the following schema:

```json
{
  "kind": "math",
  "subtype": "inline",
  "latex": "\\frac{d}{dx}\\left(x^2\\right) = 2x",
  "mathml": "<math xmlns=\"http://www.w3.org/1998/Math/MathML\">...</math>",
  "unicode": "d/dx(x²) = 2x",
  "confidence": 0.91,
  "bbox": { "page": 3, "x0": 144.0, "y0": 612.5, "x1": 310.2, "y1": 628.0 },
  "image_b64": null
}
```

Field semantics:

- `kind`: always `"math"` for math blocks.
- `subtype`: `"inline"` for expressions within a text run; `"display"` for centered block equations.
- `latex`: LaTeX source string if confidence ≥ 0.85 and reconstruction succeeded; `null` otherwise.
- `mathml`: MathML 3 XML string if extracted from StructTree or reconstructed with high confidence; `null` otherwise.
- `unicode`: Best-effort Unicode rendering of the expression; populated when confidence ≥ 0.30.
- `confidence`: Float in [0.0, 1.0] reflecting the extraction reliability estimate.
- `bbox`: Page number (1-indexed) and coordinates in PDF user-space units (origin at bottom-left).
- `image_b64`: Base64-encoded PNG crop of the rendered expression; populated only when confidence < 0.30 and a raster render is available; `null` otherwise.

When both `latex` and `mathml` are present, they are independently derived (one from StructTree, one from reconstruction) and may differ in normalization. Consumers should prefer `mathml` when present, as it is either source-authoritative or structurally more complete than the heuristic LaTeX.
