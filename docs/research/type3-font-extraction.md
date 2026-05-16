# Type 3 Font Extraction

Type 3 fonts are the most specification-compliant yet practically difficult font type in the PDF format. Unlike Type 1, TrueType, or CFF fonts — which encode glyph outlines in standardized binary formats — Type 3 fonts define each glyph as an arbitrary PDF content stream. This makes them maximally flexible but maximally opaque to text extraction. A Rust implementation must treat Type 3 handling as its own sub-pipeline.

## 1. Type 3 Font Dictionary Structure

A Type 3 font dictionary (PDF spec §9.6.5) contains the following mandatory and commonly present entries:

- **`FontBBox`**: A rectangle (in glyph space) that encompasses all glyphs in the font. Used for rasterization clipping.
- **`FontMatrix`**: A six-element transformation matrix mapping glyph space to text space. For Type 3, this is typically `[0.001 0 0 0.001 0 0]` (same as Type 1) but is frequently used for scaling in TeX-generated fonts (e.g., `[1 0 0 1 0 0]` when the glyph streams work directly in text units).
- **`CharProcs`**: A dictionary whose keys are glyph names (e.g., `/A`, `/uni0041`, `/cmr10-a`) and whose values are indirect references to content stream objects. Each stream is a self-contained glyph program.
- **`Encoding`**: Either a predefined encoding name or an Encoding dictionary with a `Differences` array. Maps 1-byte character codes (0–255) to glyph names. This is the first hop in code resolution.
- **`FirstChar`** / **`LastChar`**: Integer bounds of the character code range covered by the `Widths` array.
- **`Widths`**: Array of advance widths in glyph space units for character codes `FirstChar` through `LastChar`. A code outside this range or with a width of zero is not encoded.
- **`Resources`**: A resource dictionary shared by all CharProcs streams in the font. Can contain sub-fonts, XObjects, color spaces, and graphics state parameters.

**Character code resolution chain:**

```
character code (u8)
  → Encoding dictionary → glyph name (e.g., "/hyphen")
  → CharProcs dictionary → content stream (indirect ref)
```

Missing any link in this chain means the character is not renderable via the font's own mechanism. Record which link broke for downstream fallback routing.

## 2. What Type 3 Glyph Streams Contain

Each CharProcs value is a content stream parsed identically to a page content stream, but with two additional operators:

- **`d0 wx wy`**: Declares the advance width `(wx, wy)` in glyph space. No bounding box is declared; caching is disabled. The glyph appearance may be empty (whitespace glyph) or rendered without cache.
- **`d1 wx wy llx lly urx ury`**: Declares advance width and glyph bounding box. The viewer may cache the rendered result. This is the standard form for non-whitespace glyphs.

`d0` or `d1` must be the first operator in every CharProcs stream. After it, the stream may contain:

- **Path construction and painting**: `m`, `l`, `c`, `h`, `f`, `S`, `B`, etc. for vector glyph shapes. Most Type 3 fonts used for math symbols or decorative purposes are vector-only.
- **Image XObjects**: `Do` referencing an image XObject in the font's `Resources`. Common in scanned-font Type 3 fonts or bitmap glyph sets.
- **Text operators**: `BT`/`ET` blocks with `Tf`/`Tj`/`TJ` — a CharProcs stream can itself paint text using another font, including another Type 3 font. This is the nested Type 3 scenario.
- **Graphics state changes**: `q`/`Q`, `cm`, `w`, `J`, color operators. These affect only the glyph's internal coordinate system and should not escape it.

**The core text-extraction problem**: the content stream encodes appearance, not identity. There is no intrinsic Unicode codepoint stored in the stream. Identity must be recovered through external mappings.

## 3. Unicode Recovery: Priority Chain

Implement Unicode recovery in this strict priority order:

### (a) ToUnicode CMap

If the Type 3 font dictionary includes a `ToUnicode` entry referencing a CMap stream, parse it exactly as for any other font type. This is authoritative and should short-circuit all other recovery paths. It is rare in hand-crafted Type 3 fonts but appears in PDF generators that auto-embed it.

### (b) Glyph Name via Adobe Glyph List (AGL)

The glyph name from `CharProcs` is the primary recovery path in practice. Apply the AGL algorithm (Adobe Glyph List for New Fonts, specification version 1.7):

1. If the name is in the AGL table directly, map it.
2. If the name starts with `uni`, parse the hex suffix as one or more UTF-16BE codepoints.
3. If the name starts with `u` followed by 4–6 hex digits, parse as a single codepoint.
4. If the name contains a period (e.g., `A.sc`, `hyphen.alt`), use only the base component before the period for lookup.
5. Otherwise, the name is unrecognized — proceed to the next fallback.

Store the AGL as a static sorted array of `(&'static str, u32)` pairs and binary-search by name at runtime.

### (c) TeX Encoding Heuristics

When the font name matches a TeX Computer Modern pattern (see §4), use the known encoding vector for that font's TeX encoding scheme to resolve glyph names that AGL does not cover. TeX glyph names in Type 3 often do not follow AGL conventions and require a separate lookup table.

### (d) Shape Fingerprinting

Render the CharProcs content stream to a small raster and compare against a precomputed database of Unicode glyph hashes (see §5–6).

### (e) Context-Based Inference

In a sequence of resolved glyphs with one unknown, contextual n-gram analysis over the resolved neighbors can sometimes disambiguate with reasonable confidence. This is a last resort before emitting U+FFFD.

## 4. TeX/dvips Type 3 Fonts

TeX documents compiled via `dvips` or similar tools embed Type 3 fonts for Computer Modern and related math fonts. These fonts follow predictable conventions:

**Font name pattern**: TeX-generated Type 3 font names are typically a 6-character uppercase prefix (a subset checksum, e.g., `ABCDEF`) followed by a plus sign and the Metafont name: `ABCDEF+CMR10`, `GHIJKL+CMMI10`, `MNOPQR+CMSY10`, `STUVWX+CMEX10`.

**Detection heuristic**: if `BaseFont` matches `^[A-Z]{6}\+CM`, classify as TeX Type 3. Also check for `MSBM` (AMS blackboard bold), `EUFM` (Euler Fraktur), and `WASY` (Wasy symbol set) prefixes.

**Encoding vectors**: TeX uses non-standard 8-bit encodings. The relevant ones for glyph name resolution:

- **OT1** (original TeX text encoding): remaps standard glyph positions; `\quotedblleft` at 0x22, ligatures at positions standard fonts leave empty.
- **OML** (math italic): slots 0x00–0x7F hold lowercase Greek and math italic Latin.
- **OMS** (math symbol, CMSY): contains operators like `\cdot`, `\times`, `\ast`, `\pm` at known positions.
- **OMX** (math extension, CMEX): large delimiters, integral signs, extensible arrows — stored as multi-part glyph sequences.

Embed these encoding vectors as static lookup tables keyed on `(encoding_name, glyph_position)` → `char`. When the font name identifies a TeX font family, cross-reference the CharProcs glyph names against these tables before falling through to shape matching.

## 5. Glyph Rendering for Shape Matching

When name-based recovery fails, implement a minimal PDF graphics interpreter to rasterize the CharProcs content stream:

1. **Coordinate system**: Apply `FontMatrix` to establish glyph-to-user space. Use `FontBBox` as the clip region.
2. **Operators to support**: path construction (`m l c v y h`), path painting (`f F S s B B* b b* n`), `cm` (CTM update), `q`/`Q` (graphics state stack), `Do` (image XObjects only — do not recurse into form XObjects for shape matching).
3. **Target raster**: 64×64 pixels is sufficient for shape fingerprinting. Use 8-bit grayscale. Rasterize filled paths as white-on-black.
4. **Normalization**:
   - Compute the center of mass of foreground pixels and translate so it aligns with the raster center.
   - Scale the bounding box of foreground pixels to fill ~80% of the raster extent.
   - Apply mild Gaussian blur (σ ≈ 1.0) to suppress sub-pixel sensitivity.
5. **Hash computation**: Compute a difference hash (dHash) over the 64×64 raster — downsample to 8×8, compare adjacent pixels left-to-right, produce a 64-bit integer. Store as `u64`.
6. **Matching**: Compare the query hash against all entries in the glyph hash database using Hamming distance. A distance ≤ 8 (out of 64 bits) is a confident match; 9–15 is a weak match worth flagging with reduced confidence; > 15 is a non-match.

## 6. Building the Unicode Glyph Hash Database

The hash database must be precomputed offline and bundled with the library as a binary asset.

**Reference fonts**: render glyphs from DejaVu Serif, DejaVu Sans, Liberation Serif, Liberation Sans, GNU FreeFont (FreeSerif, FreeSans, FreeMono). Use multiple point sizes (12pt, 24pt, 48pt) and average or union the hash sets to reduce size-sensitivity.

**Coverage targets**: Basic Latin (U+0020–U+007E), Latin-1 Supplement (U+00A0–U+00FF), Latin Extended-A/B for common accented forms, Greek (U+0370–U+03FF), Cyrillic (U+0400–U+04FF), General Punctuation (U+2000–U+206F), Mathematical Operators (U+2200–U+22FF), Letterlike Symbols (U+2100–U+214F), Arrows (U+2190–U+21FF).

**Collision handling**: Multiple codepoints may hash identically (e.g., `l` vs `I` in some fonts). Store collisions as a small `Vec<u32>` per hash bucket. When a query matches a collision bucket, emit the first codepoint with `confidence: 0.5` and annotate the span with `ambiguous: true`.

**Database format**: a sorted `Vec<(u64, u32)>` (hash, codepoint) serialized with `bincode` or as a flat binary array. At query time, binary-search by hash; if not found exactly, scan neighbors within Hamming distance 8 using a BK-tree or linear scan over the sorted list.

**Stroke width variation**: vector glyphs in Type 3 fonts may be thicker or thinner than reference fonts. Normalize stroke width by morphologically thinning foreground pixels to 1-pixel skeletons before hashing both query and reference glyphs, or generate multiple reference hashes per codepoint at varying simulated stroke widths.

## 7. Nested Type 3 Fonts

A CharProcs stream may invoke another font via `BT ... Tf /FontName sz Tf ... Tj ... ET`. The nested font is resolved from the Type 3 font's own `Resources` dictionary, not the page's resource dictionary.

**Font stack tracking**: maintain a `Vec<FontRef>` during CharProcs stream execution. When `Tf` is encountered inside a CharProcs stream, push the new font onto the stack. When `ET` closes the text block, pop. Cap depth at 8 to prevent pathological recursion (though the PDF specification does not permit loops, malformed files may contain them).

**Nested encoding resolution**: resolve the nested font's character codes independently through its own encoding and CharProcs chain. Concatenate the resulting Unicode spans from the nested text into the parent glyph's output as if they were a single logical character sequence.

**Width accounting**: the outer glyph's advance width (from `d0`/`d1`) takes precedence over the sum of nested glyph widths for layout purposes.

## 8. Width-Only Glyphs (d0)

Glyphs declared with `d0` provide an advance width but no bounding box. Their appearance is never cached and may be blank (used for whitespace) or may produce visible ink that is still useful for shape matching.

Even when rendering fails entirely, the advance width is available. Use it for:

- **Whitespace detection**: if `wx` matches a known word-space width for the current font size, emit U+0020.
- **Width-profile matching**: build a width vector for a sequence of unknown glyphs and compare against frequency distributions of English letter widths. This is probabilistic but can disambiguate `i`/`l`/`1` or `m`/`w` when used with context.

Record width in the output span regardless of whether Unicode was recovered. Downstream layout reconstruction depends on it.

## 9. OCR Fallback

When all preceding methods fail to recover a Unicode mapping with acceptable confidence:

1. **Compute glyph bounds in page space**: use the text matrix, font size, and advance width to determine the bounding rectangle of the glyph on the page.
2. **Crop the rendered page**: if a rasterized page image is available (e.g., from a prior rasterization pass), extract the crop at the computed bounds, padded by 20% on each side.
3. **Run OCR**: pass the crop to a Tesseract instance (via `leptess` or a raw FFI binding) configured for single-character recognition (`--psm 10`). Limit the character whitelist to printable ASCII plus any script detected elsewhere on the page.
4. **Align OCR output**: Tesseract returns a string; for a single-character crop this should be 0–2 characters. Accept a single character result; reject multi-character results as likely noise.
5. **Confidence threshold**: Tesseract provides a mean confidence score (0–100). Accept results above 70; mark 50–70 as low confidence; reject below 50 and emit U+FFFD.

OCR on individual glyphs is expensive. Gate it behind a per-page budget (e.g., at most 50 OCR crops per page) to avoid pathological performance on pages that are entirely Type 3 text with no recoverable names.

## 10. Output Representation

Every span derived from Type 3 glyph extraction carries the following metadata fields:

- **`font_type: "type3"`**: always set for Type 3 derived spans.
- **`unicode_source`**: one of:
  - `"to_unicode_cmap"` — recovered from an explicit ToUnicode CMap entry.
  - `"glyph_name_agl"` — recovered via the Adobe Glyph List algorithm from the CharProcs key.
  - `"tex_encoding"` — recovered from a TeX OT1/OML/OMS/OMX encoding table.
  - `"shape_fingerprint"` — recovered by rasterizing the glyph and matching against the hash database.
  - `"ocr_fallback"` — recovered by OCR on the rendered page crop.
  - `"unknown"` — all methods exhausted without a confident match.
- **`confidence`**: a `f32` in `[0.0, 1.0]`. `to_unicode_cmap` and `glyph_name_agl` emit `1.0`. `tex_encoding` emits `0.95`. `shape_fingerprint` maps Hamming distance linearly: distance 0 → `1.0`, distance 8 → `0.75`. `ocr_fallback` maps Tesseract confidence divided by 100.
- **`readable: bool`**: `false` when `unicode_source == "unknown"`. Spans with `readable: false` emit U+FFFD (U+FFFD, `'\u{FFFD}'`) into the text output and are excluded from readability scoring.

This structure allows downstream consumers to filter by confidence, audit the recovery chain, and make informed decisions about whether to invoke additional post-processing (e.g., a full-page OCR pass) when `unknown` spans exceed a threshold fraction of the page.
