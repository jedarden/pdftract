# Glyph Recognition and Unicode Recovery in PDF Text Extraction

## Overview

PDF text extraction depends on the font's encoding machinery to map raw glyph identifiers — character codes in a content stream — to Unicode codepoints. When that machinery is absent, broken, or intentionally obscured, a robust extractor must fall back through a layered series of heuristics. This document specifies the **four-level Unicode recovery cascade** implemented in `pdftract` Phase 2.2, the Type 3 fallback in Phase 2.4, and the shape database design from Phase 2.5.

---

## The Four-Level Recovery Cascade

Each level attempts to recover Unicode from character codes, proceeding from highest-confidence to lowest-confidence. The first non-empty result wins. All levels emit a `confidence` score in `[0, 1]` and a `unicode_source` tag for diagnostics.

### Level 1: ToUnicode CMap (confidence = 1.0)

**Happy path** — the font explicitly declares the mapping.

Parse the `/ToUnicode` stream as a PDF CMap program. The CMap syntax to implement:

- `beginbfchar` / `endbfchar`: Single-character mappings. Format: `<srcCode> <dstHex>` where `<dstHex>` may be a UTF-16BE multi-codepoint sequence (e.g., `fi` ligature → `<00660069>`).
- `beginbfrange` / `endbfrange`: Range mappings. Two forms:
  - `<lo> <hi> <dst>`: Contiguous range where each code maps to an incrementing Unicode codepoint.
  - `<lo> <hi> [<d0> <d1> ...]`: Explicit array for non-contiguous targets.
- `usecmap`: Inherit from a named CMap (e.g., `Adobe-Japan1-UCS2`).
- Comments (`%`) stripped.

**Result**: `unicode_source = "to_unicode"`, `confidence = 1.0`.  
**Fall-through**: If the CMap maps a code to U+FFFD or U+0000, treat as missing and proceed to Level 2.

**Entry point**: `pdftract-core::font::resolve_to_unicode()` (Phase 2.2).

### Level 2: Encoding Vector + AGL (confidence = 0.9)

**Second-most-common path** — glyph names are available via the font's `/Encoding`.

Map character code → glyph name via the font's `/Encoding` dictionary:

1. **Named encodings** (hardcoded tables):
   - `WinAnsiEncoding` — Windows ANSI (superset of ISO-8859-1)
   - `MacRomanEncoding` — classic Mac OS Roman
   - `MacExpertEncoding` — expert set for old-style figures
   - `StandardEncoding` — PDF standard encoding
   - `SymbolEncoding` — Symbol font (see note below)
   - `ZapfDingbatsEncoding` — ZapfDingbats font (see note below)

2. **`/Differences` array**: Sparse overlay on base encoding. Format: `[n /GlyphName1 /GlyphName2 ...]` where `n` is the starting code position.

Map glyph name → Unicode via the **Adobe Glyph List (AGL 1.4)** algorithm:

1. Direct AGL table lookup (~4,400 entries, compiled as a static `phf::Map`).
2. If name is `uniXXXX` (exactly four uppercase hex digits), return U+XXXX. Multiple consecutive `uniXXXX` segments encode a sequence (ligatures).
3. If name is `uXXXXXX` (four to six uppercase hex digits), return U+XXXXXX, provided the codepoint is valid (not surrogate, not above U+10FFFF).
4. If name contains a period (`.`), strip suffix and retry on base name.
5. Otherwise, unrecognized → fall through to Level 3.

**Special cases**: ZapfDingbats and Symbol have their own mappings defined in ISO 32000-2 Section 9.10.2. Do NOT apply AGL to these fonts.

**Result**: `unicode_source = "agl"`, `confidence = 0.9`.  
**Entry point**: `pdftract-core::font::resolve_agl()` (Phase 2.2).

### Level 3: Font Fingerprint Cache (confidence = 0.85)

**Known-font database** — identify the embedded font and use its standard mapping.

Hash the embedded font program: SHA-256 of the raw font program stream bytes (the decoded `/FontFile`, `/FontFile2`, or `/FontFile3` stream). Look up in a bundled database of known font checksums → per-glyph Unicode mapping tables.

**Database spec**:

- Compile-time `phf::Map<[u8; 32], &'static [(u16, char)]>`
- Key: 32-byte SHA-256 of raw font program bytes
- Value: Slice of `(glyph_id, unicode_char)` pairs covering every mapped glyph
- Generated from `build/font-fingerprints.json` via `build.rs` using `phf_codegen`
- **Binary footprint**: ~500 KB (approved allocation within 4 MB budget)

**Curation pipeline**: See OQ-02 (Open Questions) and `docs/research/font-fingerprinting.md`. Initially populated with ~200 common commercial fonts from Adobe and Google Fonts metric data.

**Guard**: If the font has no embedded program (Standard-14 fonts or no `/FontFile*`), skip Level 3 and proceed to Level 4.

**Result**: `unicode_source = "fingerprint"`, `confidence = 0.85`.  
**Entry point**: `pdftract-core::font::resolve_fingerprint()` (Phase 2.2).

### Level 4: Glyph Shape Recognition (confidence = 0.7)

**Fallback** — match the rendered glyph shape against a pre-computed database.

Render the glyph to a 32×32 grayscale bitmap and compute a perceptual hash. Look up in a bundled shape→Unicode database.

**Perceptual hash algorithm (pHash)**:

1. Rasterize glyph to 32×32 grayscale bitmap:
   - TrueType/OpenType: use `fontdue` rasterizer
   - Type 3 glyphs: use Type 3 content stream renderer (Phase 2.4)
2. Apply 32×32 Discrete Cosine Transform (DCT)
3. Retain top-left 8×8 AC coefficients (64 values)
4. Threshold against median of those 64 values → 64-bit integer hash

This yields a scale-invariant hash robust to minor rendering differences.

**Database format**:

- Compile-time `&'static [(u64, char)]` — sorted slice of `(pHash, char)` pairs
- Generated from `build/glyph-shapes.json` via `build.rs` (emitted as `static` array)
- NOT `phf::Map` because we need nearest-neighbor scan, not exact lookup
- **Binary footprint**: ~300 KB for ~5,000 common glyphs (Latin, Greek, Cyrillic, extended Latin)

**Query algorithm**:

1. Linear scan over all entries computing `(query_hash XOR entry_hash).count_ones()`
2. Collect entries with Hamming distance ≤ 8
3. Select entry with smallest distance
4. **Tie-break**: Use Unicode frequency rank from companion table (`&'static [(u64, u32)]` sorted by pHash)
5. If no entry within threshold, fall through to failure

**Performance**: 5,000 entries × ~8 ns per XOR+popcount ≈ 40 µs worst-case scan.

**Tie-break rules for visually similar glyphs**:

When pHash distance is tied or ambiguous (e.g., `l` vs `I` vs `|`, `O` vs `0`):
- Prefer digits if previous span resolved to digits (monospaced font context)
- Prefer lowercase letters if surrounding text is mostly lowercase
- Use Unicode frequency rank as final tie-breaker (common chars preferred)

**Result**: `unicode_source = "shape_match"`, `confidence = 0.7`.  
**Entry point**: `pdftract-core::font::resolve_shape_match()` (Phase 2.2) and Type 3 fallback (Phase 2.4).

---

## Confidence Scoring Formula

Each level emits a confidence score:

| Level | Source | Confidence |
|-------|--------|------------|
| 1 | ToUnicode CMap | 1.0 |
| 2 | AGL lookup | 0.9 |
| 3 | Font fingerprint | 0.85 |
| 4 | Shape match | 0.7 |

**Cascade behavior**: The first non-empty result wins. Confidence is emitted in diagnostic metadata but does NOT override cascade priority.

**Post-cascade context rescoring** (optional): For results below 0.8 confidence, apply character n-gram or dictionary-based validation using surrounding high-confidence characters. This can downgrade ambiguous matches to U+FFFD with a `SHAPE_AMBIGUOUS` diagnostic.

---

## Type 3 Font Handling

Type 3 fonts define each glyph as a PDF content stream in `/CharProcs`. The same four-level cascade applies:

1. Check `/ToUnicode` first (Level 1)
2. If absent, attempt `/Encoding` glyph name lookup (Level 2)
3. If glyph name is non-standard (arbitrary user name), rasterize the content stream to 32×32 bitmap and apply shape recognition (Level 4)

**Type 3 rasterization** (Phase 2.4):

- Execute the glyph's content stream as a constrained sub-content-stream
- Track graphics state (CTM, fill/stroke state) using the Phase 3 graphics state machine
- Record stroke/fill operations to a 32×32 grayscale bitmap
- Support operators: `m l c v y` (path construction), `h S s f F B b f* B* b*` (stroke/fill), `q Q cm` (state), `Do` (form XObject, recursive), `re` (rectangle)
- Stack depth limit: 20 levels (same as form XObject limit)

**Entry point**: `pdftract-core::font::rasterize_type3_glyph()` (Phase 2.4).

---

## Database Licensing and Provenance

**Font fingerprint database** (`build/font-fingerprints.json`):

- Source: Adobe's public font databases, Google Fonts `cmap` metric exports
- License: Fonts used are SIL Open Font License or similar permissive licenses
- Curation: Maintainer-owned; see OQ-02 and `docs/research/font-fingerprinting.md`

**Glyph shape database** (`build/glyph-shapes.json`):

- Source: Glyph bitmaps rendered from open-source fonts (Google Fonts corpus, SIL OFL fonts)
- License: SIL Open Font License fonts are free of Unicode licensing entanglements
- Curation: Offline hash pipeline; JSON is the authoritative artifact

**Reprocibility**: Same glyph shape MUST always hash to the same bucket and yield the same Unicode value. No float nondeterminism, no random seeds.

---

## Failure Mode

If all four levels fail:

- Emit U+FFFD (REPLACEMENT CHARACTER)
- Set `unicode_source = "unknown"`, `confidence = 0.0`
- Log `GLYPH_UNMAPPED` diagnostic with font ID and character code

---

## Cross-References

- **Phase 2.2** (font recognition coordinator): Entry point for the four-level cascade
- **Phase 2.4** (Type 3 charstring renderer): Type 3 fallback to Level 4
- **Phase 2.5** (shape database bundling): Build artifact generation
- **OQ-02** (plan line 513): Font-fingerprint database curation pipeline ownership

---

## References

- ISO 32000-2:2020 (PDF 2.0), Section 9 (Text) and Annex D (Character Sets and Encodings)
- Adobe Glyph List Specification, version 1.7 — `adobe-type-tools/agl-specification`
- Adobe Glyph List for New Fonts (aglfn) — `adobe-type-tools/agl-aglfn`
- Adobe Type 1 Font Format specification (Black Book), Chapter 6 (Charstrings)
- Apple TrueType Reference Manual — `glyf` table specification
- OpenType Specification 1.9, Microsoft Typography (CFF / CFF2 charstring formats)
- Unicode Standard Annex #29 (Unicode Text Segmentation)
- `pdftract` plan: Phase 2.2 (line 1340), Phase 2.4 (line 1416), Phase 2.5 (line 1434)
