# Audit: CMAP and ToUnicode Entry Creation in the Generator

**Bead**: bf-2hc8pt (audit) → feeds bf-2nob5 (implement skip logic; deferred)
**Re-audit of**: `notes/bf-2nob5-child-1.md` (2026-07-05) — line numbers re-derived at HEAD, generator side added
**HEAD at audit time**: `5c64f38d` (identical at `a1feb210` where the reads
began — no audited file changed in between; shared checkout, concurrent workers)

> **Re-dispatch update, same day (2026-09-13):** after this audit was written,
> commits `5a327211` (bf-1yupei) and `d9493485` (bf-3yzopn) landed the
> reader-side ToUnicode skip and CMAP skip trace logging, and `567e3072`
> (bf-5v0bgu) moved resolver line numbers. §2's line numbers and §2.1's
> "no glyph-name-based skip possible" claim are superseded — **§7 re-derives
> everything at HEAD `9a809efe`**. §1 (generator side) is unchanged and
> remains valid.

## Executive summary

**The generator creates zero CMAP and zero ToUnicode entries today — by design.**
All PDF-writing scripts in this repo emit fonts with no `/ToUnicode` key and no
CMap stream; unmapped-ness is achieved by *omission* (fixture
`no-mapping.pdf` is pinned by a test asserting `/ToUnicode` is absent). The
"entry creation" points relevant to bf-2nob5 are therefore:

1. **Generator side (the actual target of bf-2nob5)** — the font-dict assembly
   blocks in `tools/generate_encoding_fixtures.py` and three sibling scripts,
   where a `/ToUnicode` CMap stream *would* be emitted. Any skip logic lands in
   the glyph-table loop that would feed that stream. See §1.
2. **Reader side (parsing)** — the sites where mapping entries are created
   while *reading* a PDF. These already carry `MARKER:` comments from the
   earlier audit chain, and one of them (`DifferencesOverlay::parse`,
   `font/encoding.rs`) **already implements the exact skip pattern bf-2nob5
   wants** — it is the reference implementation to mirror. See §2.
3. **Config access** — `unmapped_glyph_names` is plumbed
   `build/unmapped-glyph-names.json` → `build.rs` codegen →
   `font/unmapped.rs::UNMAPPED_GLYPH_NAMES` → `DifferencesOverlay`. The JSON
   file does not exist (never committed); the build silently falls back to a
   `.notdef`-only set. See §3.

---

## 1. Generator side — where CMAP/ToUnicode entries would be created

### 1.1 Canonical generator: `tools/generate_encoding_fixtures.py`

(Declared canonical in the bf-84xr8 close; commit `1617a42c`.)

**Glyph config data structure** — `NO_MAPPING_GLYPHS`, lines 30–42:

```python
NO_MAPPING_GLYPHS = [
    (0, "g001", True, "�"),   # (char_code, glyph_name, unmapped, expected_extraction)
    ...
    (7, "A", False, "A"),          # AGL control group (mapped)
]
```

This table is the single source of truth for which glyphs are unmapped in the
fixture. The `unmapped` boolean column is the generator-side equivalent of the
`unmapped_glyph_names` config.

**Font dictionary assembly** — `create_no_mapping_pdf()`, lines 58–124:

| What | Lines | Notes |
|---|---|---|
| `/Differences` assembly | 70–76 | `base_code` + every name from the table, in order |
| Content-stream show ops | 78–88 | hex-string `Tj` per line (`NO_MAPPING_LINES` 46–50) |
| Object list (5 objects) | 91–105 | Catalog, Pages, Page, **Font (obj 4, lines 96–101)**, Content |
| Font dict | 96–101 | `/Type1 /UnmappedTestFont /Encoding <</Differences [...]>>` — **no `/ToUnicode` key** |

Key insertion point for bf-2nob5: lines 96–101. Adding a ToUnicode CMap here
means (a) a new stream object appended to `objects` (~line 105) containing
`beginbfchar`/`beginbfrange` entries, and (b) a `/ToUnicode N 0 R` entry in the
font dict. The skip logic belongs in the loop that *emits* those bfchar
entries: iterate `NO_MAPPING_GLYPHS` and emit an entry only for rows with
`unmapped == False` (codes 7–9), while every name stays in `/Differences`
(lines 70–76) so the glyphs remain present-but-unmappable, exactly as the
parent bead requires.

Other fixtures in the same file, all with **no `/ToUnicode`, no CMap**:
`create_agl_only_pdf()` font dict lines 161–166;
`create_fingerprint_match_pdf()` font dict 237–243 (+ FontDescriptor 257–269,
FontFile3 271–283); `create_shape_match_pdf()` font dict 341–347.

### 1.2 Sibling generator scripts (same shape, no ToUnicode anywhere)

| Script | Font-dict function | `/Differences` | ToUnicode? |
|---|---|---|---|
| `tools/generate_unmapped_glyphs.py` | `create_unmapped_glyph_pdf()` (line 62) | line 123 | absent (doc line 32 says so) |
| `tools/create_unmapped_comprehensive.py` | inline obj 5 | line 67 | absent |
| `gen_unmapped_comprehensive.rs` (repo root, lopdf) | `create_unmapped_comprehensive_pdf()` | lines 52–60 | absent |

A repo-wide search for `"/ToUnicode"` as a written literal matches exactly one
non-test file — and it is a *negative* assertion (see §1.3). No script in the
repo ever writes a ToUnicode or CMap stream.

### 1.3 The absence is test-pinned

`crates/pdftract-core/tests/no_mapping_fixture_structure.rs:65`:

```rust
!text.contains("/ToUnicode"),
"Fixture declares a /ToUnicode CMap. ..."
```

**Consequence for bf-2nob5**: if the skip logic is implemented by *adding* a
ToUnicode stream for the mapped control glyphs, this pin must be updated from
"absent entirely" to "present but contains no entry for codes 0–6".

---

## 2. Reader side — entry-creation points at HEAD (MARKER sites)

These are where mapping entries are created while **parsing** a PDF. Line
numbers re-verified at HEAD `5c64f38d`; drift from the 2026-07-05 note is
listed in §5.

### 2.1 ToUnicode entries — `crates/pdftract-core/src/font/cmap.rs`

Storage (lines 66–71):

```rust
pub struct ToUnicodeMap {
    mappings: HashMap<Vec<u8>, Vec<char>>,  // src byte sequence -> Unicode codepoints
}
```

**The insertion point** — `ToUnicodeMap::add_mapping()`, lines 83–88 (doc
MARKER at 83–85):

```rust
pub fn add_mapping(&mut self, src: Vec<u8>, dst: Vec<char>) {
    self.mappings.insert(src, dst);
}
```

Call sites (all funnel through `add_mapping`):
- `CMapParser::parse()` line 145 — dispatches `beginbfchar` (153) / `beginbfrange` (160)
- `parse_beginbfchar()` (fn line 199) → `add_mapping` at **line 218**
- `parse_beginbfrange()` (fn line 232) → `add_mapping` at **line 288** (explicit-array form) and **line 301** (contiguous form)

Note: there is **no glyph-name-based skip possible here** — ToUnicode maps raw
bytes to codepoints and never sees glyph names. Bad entries (empty or U+FFFD
dst) are instead filtered at *lookup*: `resolver.rs:428–431` in `resolve_level1()`.

### 2.2 CMAP entries from /Differences — `crates/pdftract-core/src/font/encoding.rs` — **skip logic already exists here**

`DifferencesOverlay` (lines 128–137):

```rust
pub struct DifferencesOverlay {
    entries: Vec<(u8, Arc<str>)>,                        // sparse (code, glyph_name)
    unmapped_glyph_names: std::collections::HashSet<String>,  // skip set
}
```

Creation point — inside `DifferencesOverlay::parse()` (fn line 190), MARKER at
221, **guarded push at 227–228**:

```rust
// MARKER: CMAP entry creation point - Type1 font encoding differences.
if cursor <= 255 {
    if !overlay.is_unmapped_glyph_name(&name) {          // <-- THE SKIP CHECK
        overlay.entries.push((cursor as u8, Arc::clone(name)));
    }
}
```

This is the reference pattern for bf-2nob5: config-fed `HashSet<String>`,
slash-tolerant name check (`is_unmapped_glyph_name`, line 274), guard wrapped
around the entry push, defaults from `UNMAPPED_GLYPH_NAMES`
(`default_unmapped_glyph_names()`, line 163). Configurators:
`with_unmapped_glyph_names()` (153), `set_unmapped_glyph_names()` (294).

### 2.3 Codespace range entries

- `crates/pdftract-core/src/cmap/codespace.rs` — `CodespaceRange::new()`
  (MARKER doc 53–55, fn ~60; struct `{lo: [u8;4], hi: [u8;4], width: u8}`);
  parser call `ranges.push(CodespaceRange::new(...))` at **line 371** inside
  `parse_codespace_block()`.
- `crates/pdftract-core/src/font/codespace.rs` — MARKER at 274,
  `ranges.add(range)` at **line 277**.

Codespace ranges carry no glyph names, so they are **not** a skip-logic target;
listed for completeness.

### 2.4 ToUnicode resolution entry — `crates/pdftract-core/src/font/resolver.rs`

`resolve_level1()` (fn line 419), MARKER at 434–437: a Level-1 hit becomes

```rust
ResolvedGlyph::new(SmallVec::from_slice(chars), UnicodeSource::ToUnicode)
```

Consumption, not creation — included because the 2026-07-05 note cites it and
because the empty/U+FFFD fall-through at 428–431 is the *reader-side* skip.

---

## 3. `unmapped_glyph_names` config — where it is accessed

Plumbing chain:

```
build/unmapped-glyph-names.json            (workspace root build/ dir)  [MISSING — see WARN]
  └─ crates/pdftract-core/build.rs
       UnmappedGlyphNamesConfig            line 31  (unmapped_glyph_names: Vec<String>)
       generate_unmapped_glyph_names()     line 1003 (called from line 89)
       resolves workspace_root/build/...   lines ~1006–1011
       writes OUT_DIR/unmapped_glyph_names.rs
  └─ crates/pdftract-core/src/font/unmapped.rs
       include!(OUT_DIR/...)               line 9
       UNMAPPED_GLYPH_NAMES: LazyLock<HashSet<&'static str>>
       is_unmapped_glyph_name()            line 61 (slash-tolerant)
  └─ crates/pdftract-core/src/font/encoding.rs
       DifferencesOverlay.unmapped_glyph_names   line 136
       default_unmapped_glyph_names()            line 163
```

**WARN — config file does not exist.** `ls build/` has no
`unmapped-glyph-names.json`; it was never committed (`git log` for the path is
empty). `generate_unmapped_glyph_names()` therefore takes the fallback branch
(build.rs ~1013–1041): a `cargo:warning` and a set containing **only
`.notdef`**. Known since the bf-84xr8 close (breaks 5/7
`cmap_unmapped_glyphs` tests). Any bf-2nob5 implementation that reads this
config must either create the file or treat the effective set as
`{".notdef"}` — and note the *generator's* unmapped names (`g001`, `CustomA`,
…) are **not** in the runtime set; in the generator they come from the
`unmapped` column of `NO_MAPPING_GLYPHS` instead.

## 4. How to add the conditional skip logic (for bf-2nob5)

Generator side (the bead's actual scope):

1. In `tools/generate_encoding_fixtures.py`, wherever a ToUnicode CMap stream
   comes to be emitted, build it from
   `[row for row in NO_MAPPING_GLYPHS if not row[2]]` — i.e. only mapped
   controls (codes 7–9) get `beginbfchar` entries; codes 0–6 get none.
2. Keep `/Differences` assembly (lines 70–76) untouched so every glyph name —
   unmapped included — still appears in the font's encoding (parent-bead
   criterion: glyphs "STILL included in the font's /CharProcs or /Encoding
   dictionary"; `/CharProcs` would apply only if the font becomes /Type3 —
   see resolver.rs:627+ for the Type3 read path).
3. Update the `no_mapping_fixture_structure.rs:65` pin from "no /ToUnicode
   anywhere" to "no entry for codes 0–6" if a stream is added.
4. Mirror the reader-side reference implementation at `encoding.rs:223–230`:
   a name-set membership check wrapping each entry emission.

Reader side: nothing to add — the /Differences path already skips
(`encoding.rs:227`), and the ToUnicode path cannot skip by glyph name (bytes
only, §2.1); its guard is the U+FFFD/empty filter at `resolver.rs:428–431`.

## 5. Line-number drift vs notes/bf-2nob5-child-1.md (2026-07-05)

| Site | 2026-07-05 note | HEAD (5c64f38d) |
|---|---|---|
| encoding.rs MARKER / push | 196–199 | 221 / 227–228 |
| encoding.rs skip guard | (not present) | 223–230 |
| resolver.rs MARKER | 398–402 | 434–437 |
| cmap.rs add_mapping | 86–88 | doc 83–85, fn 86–88 |
| cmap/codespace.rs ctor / call | 56–60 / 354 | 53–60 / 371 |

## 6. Verification

- All line numbers above read directly from the HEAD working tree
  (`5c64f38d`; `git diff a1feb210..5c64f38d -- <audited files>` is empty);
  marker sites cross-checked with
  `rg "MARKER: (CMAP|ToUnicode) entry creation"`.
- Generator ToUnicode absence verified three ways: literal search for
  `"/ToUnicode"` across `.py`/`.rs` writers (no positive hit), manual read of
  all four font-dict assemblies, and the pinning test at
  `no_mapping_fixture_structure.rs:65`.
- Read-only audit — no production code changed.

## 7. Re-derivation at HEAD `9a809efe` (2026-09-13, re-dispatch bf-2hc8pt)

`git diff 5c64f38d..9a809efe` touched exactly three audited files
(`font/cmap.rs` +342, `font/encoding.rs`, `font/resolver.rs`); every
generator-side and config-side claim in §1/§2.3/§3 was re-verified unchanged.

### 7.1 What landed since §2 was written

| Commit | Bead | Effect on this audit |
|---|---|---|
| `5a327211` | bf-1yupei | **The reader-side ToUnicode skip now exists.** `ToUnicodeMap` gained an `unmapped_glyph_names: HashSet<String>` field (cmap.rs:74), `with_unmapped_glyph_names` (:91), `default_unmapped_glyph_names` (:103), `is_unmapped_glyph_name` (:181), accessor (:188), `set_unmapped_glyph_names` (:197), and the glyph-name-aware creation point `add_mapping_for_glyph` (:129) |
| `d9493485` | bf-3yzopn | The /Differences skip branch in `DifferencesOverlay::parse` gained a structured `tracing::trace!` on the skip path (encoding.rs:227–236) |
| `567e3072` | bf-5v0bgu | Type3 rasterizer DocumentContext activation — part of the +46 resolver.rs diff |

### 7.2 Supersedes §2.1's "no glyph-name-based skip possible"

Still true of the **parser path**: a ToUnicode CMap stream carries no glyph
names, so `CMapParser::parse` still inserts unconditionally via plain
`add_mapping` (bfchar at cmap.rs:306; bfrange explicit-array :376, contiguous
:389). What changed is that a **font-level primitive now exists**:

```rust
// cmap.rs:129 — skips + trace-logs when the name is in the configured set,
// mirroring DifferencesOverlay::parse (see notes/bf-2nob5-child-1.md)
pub fn add_mapping_for_glyph(&mut self, src: Vec<u8>, glyph_name: &str, dst: Vec<char>)
```

**It has no production caller.** Verified repo-wide: call sites are tests only
(`crates/pdftract-core/tests/cmap_unmapped_glyphs.rs:702–754`; unit tests in
`cmap.rs:1216–1401`). Wiring this primitive into the font-loading path —
where `/Differences` names are known alongside the font's ToUnicode stream —
is the remaining reader-side work for bf-2nob5.

### 7.3 Corrected line numbers at `9a809efe` (replaces §2 / §5 values)

| Site | §2 (at `5c64f38d`) | HEAD `9a809efe` |
|---|---|---|
| cmap.rs `ToUnicodeMap` struct / skip-set field | 66–71 / — | 67–75 / 74 |
| cmap.rs `add_mapping` doc/fn | 83–85 / 86–88 | 112–115 / 116–119 |
| cmap.rs `add_mapping_for_glyph` | (absent) | doc 121–128, fn 129, skip+trace 130–143 |
| cmap.rs parse dispatch bfchar/bfrange | 145 / 153 / 160 | 241–246 / 248 |
| cmap.rs `parse_beginbfchar` fn → `add_mapping` | 199 → 218 | 287 → 306 |
| cmap.rs `parse_beginbfrange` fn → `add_mapping` | 232 → 288 / 301 | 320 → 376 (array) / 389 (contiguous) |
| encoding.rs `DifferencesOverlay` struct | 128–137 | 129–137 |
| encoding.rs `parse` fn / MARKER | 190 / 221 | 190 / 221–222 (unchanged) |
| encoding.rs guard / push | 227–228 | check :227, trace 231–235, push :237 |
| encoding.rs `is_unmapped_glyph_name` | 274 | 283 |
| encoding.rs with_/default_/set_ | 153 / 163 / 294 | 153 / 163 / 303 |
| resolver.rs `resolve_level1` fn | 419 | 423 |
| resolver.rs empty/U+FFFD filter | 428–431 | 432–435 |
| resolver.rs MARKER + `ResolvedGlyph::new` | 434–437 | 438–441 |

Unchanged and re-verified by direct read at `9a809efe`: §1 generator sites
(`generate_encoding_fixtures.py` `NO_MAPPING_GLYPHS` :30, font dict :96–101,
pin test `no_mapping_fixture_structure.rs:65`), §2.3 codespace
(`cmap/codespace.rs:371`, `font/codespace.rs` MARKER :274/add :277), §3
config chain (`build.rs` :31/:89/:1003, `unmapped.rs` :9/:61). §3's WARN
stands: `build/unmapped-glyph-names.json` is still absent (re-checked
2026-09-13), so the effective skip set is `{".notdef"}` everywhere.

### 7.4 Bottom line for bf-2nob5 (updated)

1. **Generator side (§1) — unchanged, still the primary target.** Skip logic =
   emit `beginbfchar` entries only for rows with `unmapped == False` (codes
   7–9) of `NO_MAPPING_GLYPHS`; keep `/Differences` complete; update the
   `no_mapping_fixture_structure.rs:65` pin if a ToUnicode stream is added.
2. **Reader /Differences skip — done** since the child-1 audit
   (encoding.rs:224–237, now with structured trace).
3. **Reader ToUnicode skip — primitive done (bf-1yupei), wiring open.**
   `add_mapping_for_glyph` exists and is tested but nothing in `src/` feeds it
   glyph names; the CMap parser path deliberately cannot.

## References

- Prior audits: `notes/bf-2nob5-child-1.md`, `notes/bf-e4uvb-child-{1..4}*.md`
- Design docs: `notes/bf-68f9i-design.md`, `notes/bf-68f9i-glyphs.md`,
  `tests/fixtures/encoding/unmapped-comprehensive.md`
- bf-84xr8 close (canonical-generator decision): commit `1617a42c`,
  `notes/bf-84xr8.md`
