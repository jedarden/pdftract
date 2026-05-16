# Font Descriptors, Font Metrics, and Embedded Font Data

## Overview

PDF's font system is layered: a font dictionary in the content stream references a FontDescriptor, which in turn may reference an embedded font stream. For text extraction, the goal is not to render glyphs but to recover the correct Unicode string for each character code and to compute accurate bounding boxes. This document describes what pdftract must read from each layer of that structure, and why.

---

## 1. The FontDescriptor Dictionary

Every font other than the 14 standard Type 1 fonts should have a FontDescriptor dictionary, referenced by the `/FontDescriptor` key in the font dictionary. The descriptor consolidates the typographic properties of a font in one place, independent of the specific glyph outlines.

The key fields and their relevance to pdftract:

- **`/FontName`** (name): The PostScript name of the font, possibly prefixed with a six-character subset tag (e.g., `ABCDEF+Helvetica`). Used for font identification in output metadata and for triggering fallback metric lookups when the font is not embedded.
- **`/Flags`** (integer): A 32-bit bitfield encoding font classification. Discussed in detail in the next section. Critical for encoding selection logic.
- **`/FontBBox`** (rectangle: `[llx lly urx ury]`): The bounding box of the full glyph set in glyph space (1/1000 of a text unit for most fonts). Provides a conservative bounding rectangle when per-glyph metrics are unavailable.
- **`/ItalicAngle`** (number): The angle of italic slant in degrees counterclockwise from vertical. Useful for flagging italic text in output annotations, not for extraction itself.
- **`/Ascent`** (number): The maximum height above the baseline for Latin uppercase letters, in glyph space units. Together with `/Descent`, defines the typographic line box. pdftract uses these to compute the vertical extent of text runs when constructing bounding boxes for extracted spans.
- **`/Descent`** (number): The maximum depth below the baseline, typically a negative number. Paired with `/Ascent` to compute line height.
- **`/CapHeight`** (number): The height of a flat capital letter (e.g., H) above the baseline. Used as a tighter ascent estimate for all-capital text and for normalizing font size comparisons across font families.
- **`/XHeight`** (number): The height of a lowercase x above the baseline. Useful for distinguishing small-cap or lowercase text in vertical clustering during paragraph assembly.
- **`/StemV`** (number): The dominant vertical stem thickness. Not needed for extraction; present for rendering hints.
- **`/StemH`** (number): The dominant horizontal stem thickness. Not needed for extraction.

For pdftract, the operationally important fields are `/Ascent`, `/Descent`, `/CapHeight`, and `/FontBBox` for bounding box computation, and `/FontName` and `/Flags` for identification and encoding recovery.

---

## 2. Font Flags Bitfield

The `/Flags` integer encodes font classification as a bitfield (bit 1 is the least significant bit). Each bit signals a typographic category:

| Bit | Name | Meaning |
|-----|------|---------|
| 1 | FixedPitch | Monospaced font; all advance widths are equal |
| 2 | Serif | Glyphs have serifs |
| 3 | Symbolic | Font uses its own encoding or glyph set not in the Adobe standard set |
| 4 | Script | Glyphs resemble cursive/handwritten forms |
| 6 | Nonsymbolic | Font uses the standard Latin character set |
| 7 | Italic | Glyphs are slanted |
| 17 | AllCap | Font contains only uppercase glyphs |
| 18 | SmallCap | Font uses small capitals for lowercase letters |
| 19 | ForceBold | Bold strokes should be synthesized if weight is bold |

The most consequential pair for extraction is **Symbolic (bit 3) vs. Nonsymbolic (bit 6)**. These bits are mutually exclusive by spec. Their value governs which encoding pdftract applies when no explicit `/Encoding` entry or `/ToUnicode` map is present in the font dictionary:

- **Nonsymbolic**: The font uses Standard Latin encoding. Character codes outside an explicit `/Encoding` array fall back to the Adobe Standard Latin set, from which Unicode values can be inferred by glyph name.
- **Symbolic**: The font has its own private encoding. Without a `/ToUnicode` CMap or an explicit `/Encoding` array, character codes must be interpreted through the font's built-in encoding, which may only be recoverable by parsing the embedded font program itself.

pdftract evaluates these bits after exhausting higher-priority sources (ToUnicode, then Encoding), using them to select the correct fallback decoding path.

---

## 3. Embedded Font Streams

The FontDescriptor may reference one of three font stream keys, each covering a different font format:

- **`/FontFile`**: Contains a Type 1 (PostScript) font program in PFB or PFA format. The stream contains the font's encoding vector, Private dict, and charstrings.
- **`/FontFile2`**: Contains a TrueType font program in the sfnt binary format.
- **`/FontFile3`**: Contains either an OpenType/CFF, CFF-only (Type1C), or CIDFont Type 0C font. The `/Subtype` entry inside the stream dictionary identifies the exact format: `/Type1C` for CFF-wrapped Type 1, `/CIDFontType0C` for CFF-based CID fonts, and `/OpenType` for a full OpenType wrapper.

pdftract detects which key is present to determine the parsing path. The presence of a font stream does not change the ToUnicode priority, but it provides a fallback encoding source when `/ToUnicode` is absent and the `/Encoding` array is incomplete or missing.

---

## 4. Type 1 Font Programs

A PFB file consists of two segments: an ASCII section containing the font header and `/Encoding` array, and a binary section containing the Private dict and charstrings. A PFA file is entirely ASCII, with the binary segment hex-encoded.

For text extraction, pdftract need not decode Type 1 charstrings (which would require executing a stack-based virtual machine to recover glyph outlines). The encoding vector in the ASCII segment — an array of 256 glyph names indexed by character code — is sufficient. Each glyph name can be mapped to a Unicode code point via the Adobe Glyph List. This encoding vector supplements or overrides the PDF-level `/Encoding` array if the latter is incomplete.

The `/Encoding` array in the PDF font dictionary takes precedence; the embedded font's own encoding is a secondary source and only consulted when the PDF-level encoding has gaps.

---

## 5. TrueType Font Programs

A TrueType font is an sfnt container: a binary file with a table directory followed by named binary tables. The tables relevant to pdftract are:

- **`cmap`**: Maps character codes to glyph IDs. Multiple subtables may be present; pdftract prefers the Platform 3 / Encoding 1 (Windows Unicode BMP) or Platform 0 (Unicode) subtables. For a TrueType font embedded in a non-CID context, the cmap supplements the PDF `/Encoding` array and provides a Unicode fallback path when glyph names are available.
- **`hmtx`**: Horizontal metrics table. Contains `advanceWidth` and `leftSideBearing` for each glyph ID. pdftract uses these to cross-validate the PDF-level `/Widths` array. When the `/Widths` array is absent or malformed, `hmtx` provides the authoritative advance widths in font units (normalized by `unitsPerEm` from the `head` table).
- **`OS/2`**: Contains `sTypoAscender`, `sTypoDescender`, `sCapHeight`, and `sxHeight` — the typographic equivalents of the FontDescriptor fields. When the PDF FontDescriptor is absent or its ascent/descent values are zero (a known authoring bug in some PDF producers), pdftract falls back to these OS/2 values for bounding box computation. `usWeightClass` provides weight information for output metadata.

---

## 6. CFF (Compact Font Format / Type1C)

CFF encodes one or more Type 1 fonts in a compact binary format. It appears as the glyph data in both `/FontFile3` with `/Subtype /Type1C` and inside OpenType CFF fonts.

The CFF structure relevant to pdftract:

- **Top DICT**: Contains the font's encoding, charset, and offsets to other data structures. For non-CID CFF fonts, the charset maps glyph indices to SIDs (string IDs), from which glyph names are recovered. Glyph names then map to Unicode via the Adobe Glyph List.
- **FDArray** (for CID fonts): CID-keyed CFF fonts organize glyphs into font dictionaries (FD), each with its own Private DICT. There is no charset-to-name mapping; glyph indices are CIDs. For these fonts, the `/ToUnicode` CMap is the only reliable Unicode source — pdftract does not attempt to derive Unicode from raw CIDs without a ToUnicode map.
- **CharStrings index**: Contains the charstring (glyph program) for each glyph. pdftract does not execute charstrings; their presence only confirms that the font is fully embedded and not a subset with missing glyphs.

---

## 7. OpenType Fonts

OpenType is a superset of sfnt that wraps either TrueType outlines or CFF outlines. The two flavors are:

- **CFF-flavored OpenType** (`.otf`): Contains a `CFF ` table instead of `glyf`/`loca`. The sfnt structure, cmap, hmtx, and OS/2 tables are present as in TrueType. pdftract reads the `CFF ` table for glyph names when cmap lookups are insufficient.
- **TTF-flavored OpenType** (`.ttf`): Identical to TrueType from an extraction standpoint.

The **GSUB** (Glyph Substitution) table is relevant for ligature validation. GSUB lookup type 4 (Ligature Substitution) records which sequences of glyph IDs are contracted into a single ligature glyph. When a ToUnicode map assigns a multi-character string to a single character code, pdftract can cross-reference GSUB to confirm that the sequence is a known ligature, strengthening confidence in the ToUnicode mapping.

---

## 8. Multiple Master and Variable Fonts

Multiple Master fonts define a parameter space (axes such as weight and width) from which instances are interpolated. Variable fonts (OpenType 1.8+) use the `fvar`, `gvar`, and related tables. Both are rare in PDFs; when encountered, pdftract treats the font using the default or normalized instance. No axis interpolation is required — the character-to-Unicode mapping is invariant across instances, and metric values from the FontDescriptor or OS/2 table apply to the default instance without further computation.

---

## 9. Font Substitution Artifacts

When a font is not embedded and the PDF viewer substitutes a visually similar font at render time, the substitution affects only the rendered glyph shapes — not the character codes in the content stream. The ToUnicode CMap and the Encoding array remain tied to the original font's character codes. pdftract's extraction pipeline reads character codes from the content stream and resolves them through ToUnicode or Encoding, entirely bypassing the render-time substitution. Extracted text is therefore unaffected by font substitution, though extracted bounding boxes may be slightly inaccurate if the PDF's `/Widths` array was generated for the original unembedded font and the substitute has different metrics.

---

## 10. Font Name Normalization

Many embedded fonts carry a subset prefix: a six-character uppercase tag followed by a `+` separator, as in `ABCDEF+TimesNewRomanPS-BoldMT`. pdftract strips this prefix using a regex match on `/^[A-Z]{6}\+/` before using the base name.

The base name is then normalized for font identification in output:

1. **PostScript name parsing**: Hyphens separate the family name from style modifiers (e.g., `Helvetica-BoldOblique` → family `Helvetica`, style `Bold Oblique`).
2. **Family equivalence mapping**: A lookup table maps common alias pairs to canonical families — `Arial` → `Helvetica`, `TimesNewRoman` → `Times`, `CourierNew` → `Courier`. This mapping is used for fallback metric lookup only; pdftract preserves the original `/FontName` in its output metadata.
3. **Standard font fallback**: If no font stream is present and the normalized base name matches one of the 14 standard Type 1 fonts, pdftract uses the corresponding built-in metric table (ascent, descent, cap height, per-glyph widths) rather than returning zero values.

The normalized font name is emitted in pdftract's per-span output as `font_family`, alongside `font_size`, `is_bold`, `is_italic`, and `is_monospace` (derived from the FixedPitch flag). This gives downstream consumers enough information to reconstruct basic typography without access to the PDF itself.

---

## Summary of pdftract Reading Priorities

| Purpose | Primary Source | Secondary Source | Tertiary Source |
|---------|---------------|-----------------|----------------|
| Unicode mapping | ToUnicode CMap | /Encoding + Glyph List | Embedded font encoding vector |
| Advance widths | /Widths array | hmtx (TrueType/OT) | FontDescriptor /FontBBox width |
| Ascent/Descent | FontDescriptor /Ascent, /Descent | OS/2 sTypoAscender/Descender | /FontBBox [lly, ury] |
| Font identification | /FontName (stripped) | /BaseFont | Embedded font name record |
| Encoding fallback | /Flags Symbolic/Nonsymbolic | Embedded font /Encoding | Standard Latin defaults |
