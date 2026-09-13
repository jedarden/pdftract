# no-mapping.pdf Fixture

## Purpose

Worst-case Unicode recovery fixture: a Type1 font whose `/Encoding /Differences`
array assigns ten glyph names to character codes 0–9. Seven names are unmapped —
no `/ToUnicode` exists (Level 1), the names are absent from the Adobe Glyph List
and match neither algorithmic convention `uniXXXX`/`uXXXXXX` (Level 2), the font
is not embedded so there is nothing to fingerprint (Level 3), and no shape record
exists (Level 4). Three names (`/A`, `/B`, `/space`) are standard AGL names kept
as the success-path control group.

The glyph table is `NO_MAPPING_GLYPHS` in `tools/generate_encoding_fixtures.py`;
its `unmapped` boolean column is the generator-side equivalent of the reader's
`unmapped_glyph_names` configuration set.

Expected extraction: seven U+FFFD replacement characters followed by `AB `
(GLYPH_UNMAPPED diagnostics for codes 0–6).

## Glyph Table

| Code | Glyph Name  | Unmapped | Expected |
|------|-------------|----------|----------|
| 0    | /g001       | YES      | U+FFFD   |
| 1    | /g002       | YES      | U+FFFD   |
| 2    | /g003       | YES      | U+FFFD   |
| 3    | /CustomA    | YES      | U+FFFD   |
| 4    | /CustomB    | YES      | U+FFFD   |
| 5    | /NotAGlyph  | YES      | U+FFFD   |
| 6    | /glyph_0041 | YES      | U+FFFD   |
| 7    | /A          | no       | `A`      |
| 8    | /B          | no       | `B`      |
| 9    | /space      | no       | ` `      |

## Structure

### PDF Properties
- **PDF Version:** 1.4
- **Pages:** 1
- **Page Size:** 612 x 792 pts (Letter)
- **File Size:** 735 bytes
- **SHA256:** `4e35868ca3c5a66b46e38f4fe52c3bea00bb4bc77a5a5532992f4fa11da46d6d`
- **Encrypted:** No
- **Tagged:** No

### Font Details
```
Name: UnmappedTestFont
Type: Type 1
Encoding: Custom (Differences array)
Embedded: No
ToUnicode CMap: No (the key is absent from the font dictionary)
CMap stream: No (no beginbfchar/beginbfrange/begincmap anywhere in the file)
```

**Font Object (4 0 obj):**
```
<<
/Type /Font
/Subtype /Type1
/BaseFont /UnmappedTestFont
/Encoding <<
/Type /Encoding
/Differences [0 /g001 /g002 /g003 /CustomA /CustomB /NotAGlyph /glyph_0041 /A /B /space]
>>
>>
```

### Content Stream (5 0 obj)

```
BT
/F1 12 Tf
50 700 Td
<000102> Tj
<03040506> Tj
<070809> Tj
ET
```

Show operators reference glyphs by **character code**; the `/Differences` array
resolves each code to its glyph name. PDF content streams never spell glyph
names inside show strings:

| Show op        | Codes       | Glyph names resolved via /Differences          |
|----------------|-------------|------------------------------------------------|
| `<000102> Tj`  | 0, 1, 2     | g001, g002, g003                               |
| `<03040506> Tj`| 3, 4, 5, 6  | CustomA, CustomB, NotAGlyph, glyph_0041        |
| `<070809> Tj`  | 7, 8, 9     | A, B, space                                    |

## Unmapped-Glyph Invariants (bf-5oytjz)

The unmapped glyphs are skipped by the reader's CMAP/ToUnicode entry creation
(`DifferencesOverlay::parse` in `crates/pdftract-core/src/font/encoding.rs`,
`ToUnicodeMap::add_mapping_for_glyph` in `crates/pdftract-core/src/font/cmap.rs`)
but must **remain present and referenceable through the font's encoding
dictionary**:

1. Every glyph name — unmapped included — appears in `/Encoding /Differences`,
   so content-stream codes keep selecting those glyphs.
2. The PDF contains **no CMap entries** for the unmapped glyphs: no CMap stream
   exists, and the reader creates no Differences-overlay entries for names in a
   configured unmapped set.
3. The PDF contains **no `/ToUnicode` entries** for them: the key is absent
   entirely, and `ToUnicodeMap` creates no entry for a configured unmapped name.
4. Extraction therefore surfaces codes 0–6 as GLYPH_UNMAPPED / U+FFFD while the
   control group recovers through Level 2 AGL lookup.

These are pinned by `crates/pdftract-core/tests/no_mapping_fixture_structure.rs`
(`fixture_has_all_glyph_names_in_differences_array`,
`fixture_declares_no_tounicode_cmap`,
`fixture_content_stream_shows_all_ten_character_codes`, …) and the
`font::encoding` / `font::cmap` unit tests for the configured-set skip.

## Inspection Commands

No poppler tools are required; every check below is byte-level:

```bash
# /Differences carries all ten glyph names, unmapped included
grep -a "/Differences" tests/fixtures/encoding/no-mapping.pdf

# No ToUnicode / CMap entries anywhere in the file (all must print 0)
grep -ac "/ToUnicode"       tests/fixtures/encoding/no-mapping.pdf
grep -ac "beginbfchar"      tests/fixtures/encoding/no-mapping.pdf
grep -ac "beginbfrange"     tests/fixtures/encoding/no-mapping.pdf
grep -ac "begincmap"        tests/fixtures/encoding/no-mapping.pdf

# Content stream show ops reference the ten codes
grep -a "Tj" tests/fixtures/encoding/no-mapping.pdf

# Ground truth: 7 x U+FFFD + "AB "
od -c tests/fixtures/encoding/no-mapping.txt
```

## Regeneration

Canonical generator: `tools/generate_encoding_fixtures.py` (declared canonical
in the bf-84xr8 close, commit `1617a42c`). Object offsets and the content-stream
`/Length` are computed, not hard-coded, so editing `NO_MAPPING_GLYPHS` or
`NO_MAPPING_LINES` cannot desynchronize the xref table.

```bash
python3 tools/generate_encoding_fixtures.py
sha256sum tests/fixtures/encoding/no-mapping.pdf
# 4e35868ca3c5a66b46e38f4fe52c3bea00bb4bc77a5a5532992f4fa11da46d6d
```

Verified byte-reproducible at HEAD `5a327211` (2026-09-13): regenerating
yields the committed file bit-for-bit.

**Caution:** the script regenerates all four encoding fixtures in one run. As
of 2026-09-13 the committed `agl-only.pdf` and `shape-match.pdf` differ from
current generator output (pre-existing drift, out of scope of the bf-5oytjz
verification) — restore them with
`git checkout -- tests/fixtures/encoding/{agl-only,shape-match}.{pdf,txt}`
if you only need `no-mapping.pdf`.

## Ground Truth

`tests/fixtures/encoding/no-mapping.txt` contains 7 × U+FFFD followed by
`AB ` (10 characters, one per code), derived from the glyph table's
`expected_extraction` column by `no_mapping_ground_truth()`.

## Test References

- `crates/pdftract-core/tests/no_mapping_fixture_structure.rs` — byte-level
  structure pins (7 tests, all green)
- `crates/pdftract-core/tests/encoding_recovery.rs` — end-to-end recovery
  metrics (currently failing for all four fixtures: the extractor rejects
  these minimal hand-built PDFs with "empty or contains no content" /
  "No /Root reference in trailer"; the structure tests above are deliberately
  independent of that, per their module docs)
- `crates/pdftract-core/tests/cmap_unmapped_glyphs.rs` — CMAP/ToUnicode skip
  behavior (3/8 pass; 5 failures are the documented missing
  `build/unmapped-glyph-names.json` baseline, see notes/bf-2nob5-audit.md §3)

## Related Fixtures

- `agl-only.pdf` — standard Type1 font with AGL glyph names (Level 2 recovery)
- `fingerprint-match.pdf` — embedded font subset for fingerprint matching (Level 3)
- `shape-match.pdf` — custom glyph names with shape recognition (Level 4)
- `unmapped-glyphs.pdf`, `unmapped-comprehensive.pdf` — sibling unmapped-glyph
  fixtures from other generator scripts (see their own .md docs)

## History

- **Generated:** 2026-06-09
- **Regenerated:** 2026-07-02 (bf-512z1)
- **Regenerated:** 2026-07-03 (bf-f0xqd) — corrected ground truth from "ABC" to U+FFFD
- **Regenerated:** 2026-07-03 (bf-1m30m) — via `generate_encoding_fixtures.rs`
- **Regenerated:** 2026-09-05 (bf-84xr8, commit `1617a42c`) — 10-glyph table-driven
  generation via the canonical Python generator; control group `/A /B /space` added
- **Verified:** 2026-09-13 (bf-5oytjz) — byte-reproducible at HEAD `5a327211`;
  documentation rewritten to match the actual 10-glyph fixture (previous doc
  described the pre-bf-84xr8 3-glyph version)

## Notes

The key insight of this fixture is that it represents the **worst-case scenario**
for Unicode recovery while keeping the glyphs *reachable*:

1. No `/ToUnicode` CMap (nothing at Level 1)
2. Custom encoding with no StandardEncoding/WinAnsiEncoding/MacRomanEncoding
   fallback, so recovery depends entirely on the `/Differences` names
3. Seven glyph names outside the AGL and matching no algorithmic convention
4. No embedded font program for fingerprinting

The unmapped glyphs are excluded only from the *mapping* structures (CMAP /
ToUnicode entry creation); the font's `/Encoding` dictionary keeps every name,
which is what lets content streams reference them directly by code.
