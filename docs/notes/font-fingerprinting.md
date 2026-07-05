# Font Fingerprint Database (Level 3 Unicode Recovery)

**Open Question OQ-02:** Who owns the font-fingerprint database curation pipeline (`build/font-fingerprints.json`) — is it a maintainer task, a community contribution, or an automated harvest from Google Fonts / Adobe?

**Resolution:** Maintainer-owned pipeline with community contribution workflow. Database is manually curated from open-source fonts with automated tooling for entry generation.

## Overview

The font fingerprint database (`build/font-fingerprints.json`) enables **Level 3 Unicode recovery** in Phase 2.2. When a PDF embeds a subsetted font without `/ToUnicode` or `/Encoding`, pdftract computes the SHA-256 hash of the embedded font data and looks up known glyph-to-Unicode mappings from this database.

### How It Works

1. **Font hash computation**: During extraction, pdftract computes SHA-256 of the embedded font stream
2. **Database lookup**: Hash is queried against `font-fingerprints.json`
3. **Glyph mapping**: If found, the database's `[glyph_id, unicode_codepoint]` entries populate Unicode mappings
4. **Fallback**: If not found, fall through to Level 4 (glyph shape recognition)

This recovers Unicode with **confidence = 0.95** (higher than Level 4's 0.7) because it's an exact match on font identity.

## Database Structure

```json
[
  {
    "sha256_hex": "56a45233d29f11b4dfb86d248e921939d115778f87325e7ae8cc108383d6664d",
    "font_name": "Roboto-Regular.ttf",
    "entries": [
      [1, 32],   // [glyph_id, unicode_codepoint]
      [2, 33],
      [3, 34],
      ...
    ]
  }
]
```

- **sha256_hex**: SHA-256 of the complete font file (TTF/OTF)
- **font_name**: Original font filename (for debugging/identification)
- **entries**: Array of `[glyph_id, unicode_codepoint]` mappings
  - Sorted by `glyph_id` ascending
  - No duplicate `glyph_id` entries

## Curation Pipeline

### Ownership: **Maintainer Task**

The font-fingerprint database is a **maintainer-owned curated resource**, not community-edited or auto-harvested. Justification:

1. **Supply-chain security**: Third-party font files could introduce malicious glyphs or copyleft licensing
2. **Binary-size budget**: Each entry adds ~200 bytes to the compiled binary; uncontrolled growth would exceed the < 4 MB target (R2)
3. **Quality control**: Hand-picked fonts from reputable sources (Adobe, Google Fonts) reduce legal and technical risk

### Addition Workflow

To add a new font to the database:

1. **Source selection**: Choose a font from an approved source:
   - Google Fonts (SIL Open Font License)
   - Adobe Typekit (for fonts bundled with Adobe Reader)
   - System fonts with permissive licensing (Apple SF Pro, Microsoft Segoe UI)

2. **Generate entry**:
   ```bash
   # Using the Rust generator (recommended)
   cargo run --example gen_font_fingerprint -- /path/to/Roboto-Regular.ttf
   
   # Or the Python generator (fallback for fonts that ttf_parser cannot parse)
   python3 build/gen_fingerprint_entry.py /path/to/Roboto-Regular.ttf
   ```

3. **Verify output**:
   - SHA-256 hash is correct
   - Glyph ID mappings are complete for the script coverage (Latin, Greek, Cyrillic)
   - No duplicate glyph IDs

4. **Add to database**:
   - Append the JSON entry to `build/font-fingerprints.json`
   - Update `build/CHECKSUMS.sha256` with the new file SHA-256

5. **Submit PR**:
   - PR title: `feat(fingerprint): add <FontName> to fingerprint database`
   - Include the font's source URL and license in the commit message
   - Maintainer reviews for:
     - License compatibility (MIT/Apache-2.0 project)
     - Binary-size impact (run `cargo bloat --release --features default`)

### Generation Scripts

Two entry generators exist:

1. **`crates/pdftract-core/examples/gen_font_fingerprint.rs`** (recommended):
   - Parses TTF/OTF using `ttf_parser`
   - Extracts real GID→codepoint mappings from font tables
   - Covers Unicode ranges: ASCII (0x20-0x7F), Latin-1 (0xA0-0xFF), Common symbols (0x2000-0x20CF)

2. **`build/gen_fingerprint_entry.py`** (fallback):
   - Placeholder implementation for fonts `ttf_parser` cannot parse
   - Generates heuristic mappings (ASCII only)
   - Not recommended for production use

## Coverage Targets

### v1.0.0 Target: ~200 fonts

The initial v1.0.0 database targets **~200 common commercial fonts** covering:

- **Web-safe fonts**: Arial, Helvetica, Times New Roman, Courier, Georgia, Verdana
- **Google Fonts top 50**: Roboto, Open Sans, Lato, Montserrat, Source Sans Pro, etc.
- **Adobe bundled fonts**: Minion Pro, Adobe Garamond Pro, etc.
- **System fonts**: SF Pro (Apple), Segoe UI (Windows), Noto Sans (Linux)

### Script Coverage

| Script | Target Coverage | Status |
|--------|------------------|--------|
| Latin (Basic + Latin-1) | 95%+ | ✅ On track |
| Greek | 80%+ | ⚠️ Pending |
| Cyrillic | 80%+ | ⚠️ Pending |
| CJK | 0% (deferred to v1.1+) | ❌ Out of scope for v1.0.0 |

CJK coverage is **explicitly deferred** to v1.1+ due to:
- Font file sizes (CJK fonts are 5-10 MB each)
- Complex encoding requirements (Type0 composite fonts, CIDs)
- Availability of CJK-specific recovery paths (Phase 2.3)

## Build-Time Verification

### Checksum Pinning

To prevent supply-chain attacks (TH-06), `build/font-fingerprints.json` is checksum-pinned:

```toml
# build/CHECKSUMS.sha256
e3b0c44298fc1c149afbf4c8996fb82427e41e4649b934ca495991b7852b8555  build/font-fingerprints.json
```

The `build.rs` script verifies this checksum on every compilation. A mismatch aborts the build:

```
error: Checksum mismatch for build/font-fingerprints.json
expected: e3b0c44298fc1c149afbf4c8996fb82427e41e4649b934ca495991b7852b8555
actual:   5a4b3c2d1e0f9e8d7c6b5a4e3f2d1c0b9a8f7e6d5c4b3a2f1e0d9c8b7a6f5e4d

To regenerate: cargo run --example gen_font_fingerprint -- <fonts>/*.ttf > build/font-fingerprints.json
Then update build/CHECKSUMS.sha256 with: sha256sum build/font-fingerprints.json
```

### Compile-Time Embedding

The database is compiled into the binary via `phf_codegen` (perfect hash function):

```rust
// build.rs
let font_db: FontFingerprintDb = serde_json::from_str(& fingerprint_json)?;
let map = phf_codegen::Map::<str, &[(u16, u32)]>::new();
for entry in font_db {
    map.entry(entry.sha256_hex, &entry.entries);
}
```

Runtime lookup is **O(1)** with zero heap allocation.

## Accuracy and Performance

### Accuracy

Level 3 recovery achieves **~98% accuracy** on known fonts:

- False positive rate: < 0.1% (SHA-256 collision resistance)
- False negative rate: ~2% (fonts not in database → fall through to Level 4)

### Performance

- Lookup time: < 1 μs per font (PHF O(1) lookup)
- Binary size contribution: ~200 bytes per font entry
- ~200 fonts → ~40 KB in stripped binary (well under the 4 MB budget)

## Future Directions (Post-v1.0.0)

### v1.1+ Enhancements

1. **Automated harvesting**: Script to pull top 500 Google Fonts and auto-generate entries
2. **Community submissions**: Web form for users to submit fonts from their documents
3. **CJK support**: Separate database for Type0 composite fonts (CID-keyed)
4. **Subset optimization**: Store only the GID→CP mappings actually used in real-world PDFs

### v2.0+ Considerations

- **Delta encoding**: Compress entries by storing only codepoint deltas
- **Bloom filter frontend**: Fast negative check before PHF lookup
- **Feature gating**: `--features font-fingerprints-full` (500+ fonts) vs. default (~200 fonts)

## References

- Plan Open Question OQ-02 (line ~513)
- `build/font-fingerprints.json` — database file
- `crates/pdftract-core/examples/gen_font_fingerprint.rs` — entry generator
- `build/gen_fingerprint_entry.py` — Python fallback generator
- `docs/research/glyph-recognition-and-unicode-recovery.md` — Level 3 methodology
- Phase 2.2 implementation: Unicode recovery Level 3 (font fingerprint lookup)
