# Font Subsetting and Extraction

## Overview

Font subsetting is among the most consequential sources of text extraction failure in practice, yet it receives less attention than encoding tables or CMap parsing. The failure mode is subtle: a font can carry a valid ToUnicode CMap, a well-formed glyph table, and still produce incorrect or missing text because the subset was constructed in a way that breaks the assumptions the extractor relies on. This document covers the mechanics of subsetting, the naming conventions that identify subset fonts, the specific failure modes at each stage of the extraction pipeline, and the recovery strategies a Rust extractor should implement.

---

## 1. What Font Subsetting Is

Embedding an entire font in a PDF is rarely practical. A full OpenType CJK font routinely occupies 15–25 MB. A full Latin font with all OpenType features is 200–800 KB. Most documents use a fraction of those glyphs: a business letter uses roughly 70–120 distinct characters; a CJK document with 500 unique characters may draw on 0.5–2% of a full font's glyph repertoire.

Authoring tools solve this with **subsetting**: only the glyph programs actually referenced in the document's content streams are embedded. The authoring tool collects every character code appearing in `Tj`, `TJ`, and related text operators, resolves each to a glyph index in the source font, then extracts only those glyph programs into the embedded font. Additional glyphs may be included if the font's shaping rules require them — composite glyphs in TrueType (a glyph that references component glyphs via `glyf` entries), or ligature alternates that the layout engine applied during composition.

Subsetting ratios vary widely:

- **CJK, small document:** 0.5–2% of full font (200 CIDs from 20,000+)
- **Latin, typical document:** 15–50% of full font (80–300 glyphs from ~600)
- **Latin, near-exhaustive use:** 70–95% of full font (a typeset book using most of the character set)

The practical consequence for extraction: any glyph not in the subset is inaccessible. Attempting to render or name it yields `.notdef`. Knowing that a font is subsetted tells the extractor that absent glyph entries are not bugs — they are by design — and that only the embedded population can be relied upon.

---

## 2. Subset Font Naming

The PDF specification (ISO 32000-2, §9.6.4) mandates a specific naming convention for subset fonts. The `/BaseFont` value in the font dictionary and the `/FontName` value in the `/FontDescriptor` dictionary must both carry a **six-uppercase-letter prefix followed by a plus sign**, e.g.:

```
ABCDEF+Helvetica
XYZQRT+NotoSansCJK-Regular
TMWVPK+CMR10
```

The six letters are chosen arbitrarily by the authoring tool; they carry no semantic content. They are not reproducible across invocations or tool versions — the same document saved twice may produce different prefixes. Their only function is to distinguish this subset instance from the full font and from other subsets of the same font within the same document.

In a Rust extractor, detecting subset fonts reduces to a pattern match against the `/BaseFont` or `/FontName` name object:

```rust
fn is_subset_font(name: &str) -> bool {
    let bytes = name.as_bytes();
    bytes.len() > 7
        && bytes[6] == b'+'
        && bytes[..6].iter().all(|b| b.is_ascii_uppercase())
}

fn extract_subset_prefix(name: &str) -> Option<&str> {
    is_subset_font(name).then(|| &name[..6])
}
```

When both `/BaseFont` and `/FontDescriptor /FontName` are present, they should carry the same prefix. A mismatch indicates a malformed font dictionary; the extractor should prefer the `/FontDescriptor` value for identification purposes and log a warning.

---

## 3. Glyph Re-encoding in Subsets

Subsetting tools frequently re-assign character codes. In the source font, glyph `A` occupies code point 0x41 in the font's encoding or cmap. In the subset, the tool may compact the code space, assigning the glyphs sequential codes starting at 0x01 or 0x20. This is valid: the content stream uses whatever codes the authoring tool wrote, and the font's encoding machinery maps those codes to glyph indices. The critical link is the **ToUnicode CMap** (§9.10.3): it maps the reassigned in-PDF character codes back to Unicode scalar values. If the ToUnicode CMap is present and covers all codes used in the content stream, re-encoding is fully transparent to the extractor.

If the ToUnicode CMap is absent or incomplete, the extractor cannot recover Unicode values by examining the embedded font's cmap table alone, because that cmap reflects the subset's internal code assignments, not Unicode. The embedded cmap is useful for **cross-validating** ToUnicode entries but cannot substitute for it when codes have been reassigned.

---

## 4. CIDFont Subsetting

Type 0 (composite) fonts wrap a CIDFont. The CIDFont embeds glyph data indexed by **CID** (character identifier). For Identity-H and Identity-V CMaps, the CID equals the two-byte character code in the content stream. For other predefined CMaps, the CID is looked up via the CMap's code space ranges.

When a CIDFont is subsetted, the embedded font data contains only the CIDs that were used. The `/CIDToGIDMap` stream (when present) maps CIDs to glyph indices within the embedded font file; for a subset, only entries for included CIDs are meaningful. CIDs outside the subset either have no entry in the CIDToGIDMap or map to GID 0 (`.notdef`).

The ToUnicode CMap for a Type 0 font maps CIDs (or character codes) to Unicode. For subsetted CIDFonts, the ToUnicode CMap should cover exactly the CIDs present in the subset. A ToUnicode entry for a CID not in the subset is harmless but unreachable. A CID present in the content stream but absent from both ToUnicode and the embedded font is an unmapped extraction failure.

---

## 5. OpenType CFF and Type 1 Glyph Table Subsetting

CFF-based fonts (Type 1 fonts and OpenType fonts with a `CFF ` table) store glyph programs as **charstrings** in a `CharStrings` dictionary keyed by glyph name. In a subset, only the charstring entries for included glyphs are present. The extractor can enumerate present glyph names by iterating the CharStrings dict.

This property is useful: even in a heavily subsetted CFF font, the glyph names remain available (e.g., `A`, `fi`, `uni0041`, `uniE001`). For AGL (Adobe Glyph List) lookup, the glyph name is sufficient to recover Unicode without consulting ToUnicode. For shape fingerprinting (rendering the charstring to an outline and matching against a glyph database), only present charstrings can be rendered — the extractor must skip absent glyphs rather than treating their absence as an error.

---

## 6. TrueType Glyph Table Subsetting

TrueType fonts (embedded as `/FontFile2` streams) store glyph outlines in the `glyf` table, with an index in the `loca` table mapping each GID to its offset and length. After subsetting:

- **`loca` entries for excluded GIDs** point to zero-length regions (the GID is present in the index but has no glyph data).
- **`maxp.numGlyphs`** reflects the total GID range in the subset, not the full font.
- **`cmap` table** may be present and contains character-to-GID mappings for the subsetted characters only; non-subsetted characters either have no entry or map to GID 0.

The subset's `cmap` is a useful validation tool: for each code in the ToUnicode CMap, the extractor can verify that the Unicode scalar maps back to a GID with a non-empty `glyf` entry. Discrepancies surface authoring tool bugs or intentional re-encoding.

Composite TrueType glyphs (those with the `COMPOSITE` flag in the `glyf` header) reference component GIDs. If a component GID was not included in the subset, the composite glyph's rendering breaks. Well-behaved subsetting tools always include required components, but the extractor should treat a composite glyph with a missing component as a rendering failure, not a parsing error, and fall back to ToUnicode for the character identity.

---

## 7. Incomplete ToUnicode CMaps

The most operationally significant failure mode is a ToUnicode CMap that covers only a subset of the codes actually used in the document. This happens when:

- The authoring tool generates ToUnicode incrementally and stops before covering all codes.
- The document was assembled from multiple sources with inconsistent encoding tables.
- The font was substituted late in the rendering pipeline without regenerating ToUnicode.

From the extractor's perspective, a code not present in the ToUnicode CMap is **unmapped**: the lookup returns nothing. This is distinct from a code that explicitly maps to U+FFFD (replacement character) or U+0000, both of which are valid (if uninformative) mappings. The extractor must distinguish:

1. `lookup(code) == None` — code absent from CMap; attempt fallback
2. `lookup(code) == Some(0xFFFD)` — explicit no-mapping; still attempt fallback
3. `lookup(code) == Some(c)` where `c` is a valid Unicode scalar — accept

For unmapped codes in a subset font, the recovery path is:

1. **AGL glyph name lookup**: if the embedded font (CFF or Type 1) has a glyph name for the GID, look it up in the Adobe Glyph List. Names like `A`, `fi`, `uni0041`, `uniE001` resolve to Unicode directly.
2. **Shape fingerprinting**: render the glyph outline from the embedded font (charstring execution for CFF, `glyf` parsing for TrueType) and match the normalized path against a reference glyph database. This is computationally expensive and reserved for high-value recovery scenarios.
3. **Unextractable**: if both fail, report the span as unextractable with the raw character code preserved for inspection.

AGL lookup and shape fingerprinting are always worth attempting for partially unmapped subset fonts, even if the majority of codes are mapped via ToUnicode. Partial coverage is common enough that implementing the fallback path yields meaningful improvements in extraction completeness.

---

## 8. Synthetic Glyphs and Outlines

Some subset tools — particularly those processing DRM-restricted fonts — cannot legally embed glyph outlines. Instead, they substitute **synthetic glyphs**: a charstring or `glyf` entry that traces a blank path or a simple rectangle matching the advance width of the original glyph. The glyph occupies the correct horizontal space and the ToUnicode CMap may map the code to the correct Unicode value, but the outline contains no recoverable character identity.

Detection in CFF charstrings: after executing the charstring, if the path operation list is empty (the glyph is `endchar` with no prior drawing operators), or contains only a single rectangular path (four `lineto` calls forming a closed rectangle), the glyph is synthetic. In TrueType, a `glyf` entry consisting solely of a single contour with four on-curve points arranged as a rectangle at the advance-width boundary is the equivalent indicator.

When a synthetic glyph is detected, the extractor should:

- Use the ToUnicode mapping if present (the Unicode value is likely correct even if the outline is not).
- Flag the span as `synthetic_glyph: true` in the extraction output.
- Report character identity confidence as lower, since the Unicode mapping was placed by the DRM tool and may be incorrect.

---

## 9. Re-subsetting and Incremental Update Interactions

A PDF incremental update appends a new cross-reference table and body section without rewriting earlier content. When a page is added that uses an already-subsetted font, the update may extend the subset by appending new glyph data and a revised ToUnicode CMap. The extended font object (with a new object number or an updated generation) replaces the original in the cross-reference resolution order — the **last definition of an object in the file wins**.

The extractor must parse the cross-reference chain from tail to head, ensuring that the most recent font dictionary and ToUnicode CMap are used for each font reference. A common mistake is merging ToUnicode CMaps additively across incremental updates; the correct behavior is to use only the latest CMap for the given font object, which should already incorporate all prior mappings.

---

## 10. Detection and Reporting

Every font processed by the extractor should carry structured subsetting metadata:

```rust
pub struct FontSubsetInfo {
    pub is_subset: bool,
    pub subset_prefix: Option<String>,   // e.g. "ABCDEF"
    pub glyphs_embedded: usize,          // from maxp.numGlyphs or CharStrings len
}
```

Every extracted span should reference the font's subsetting state and record the source of its Unicode mapping:

```rust
pub enum UnicodeSource {
    ToUnicode,
    AglGlyphName,
    ShapeFingerprint,
    Unmapped,
    SyntheticGlyph,
}

pub struct SpanMetadata {
    pub font_is_subset: bool,
    pub unicode_source: UnicodeSource,
    // ... other fields
}
```

Confidence assessment:

- `ToUnicode` from a subset font with verified complete CMap coverage: **high confidence**.
- `ToUnicode` from a subset font with partial CMap coverage, this span's codes all mapped: **high confidence**.
- `AglGlyphName`: **medium confidence** (glyph name may be generic or incorrect in the subset).
- `ShapeFingerprint`: **medium confidence** (outline matching has false-positive risk).
- `Unmapped` or `SyntheticGlyph`: **low confidence / unextractable**; preserve raw code for downstream inspection.

Reporting per-span confidence rather than per-font allows the caller to make document-level decisions: a document that is 98% `ToUnicode`-sourced with 2% `Unmapped` spans in footnotes is far more usable than one with 40% unmapped spans in body text.
