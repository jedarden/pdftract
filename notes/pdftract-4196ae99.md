# pdftract-4196ae99 — multi-section xref resolution: /Root resolves on mislabelled-free tables, /Prev chains, and xref-stream documents

- Fix commit: `fix(pdftract-4196ae99)` — `crates/pdftract-core/src/parser/xref.rs`
  + `crates/pdftract-core/tests/realworld_extraction_baseline.rs`.
- Verified in a clean `git archive HEAD` extraction under `/var/tmp/bead-4196ae99`
  with a private `CARGO_TARGET_DIR` (never the shared working tree — the shared
  tree carries a concurrent worker's uncommitted xref.rs rewrite and does not
  compile as a union). HEAD at dispatch: `d7f7f834`.

## Fixture-structure findings (probed before assuming which hypothesis applied)

Both committed fixtures were inspected at the byte level (python over the raw
bytes: startxref values, `%%EOF` count, every `xref` keyword occurrence,
`^N G obj` header offsets, and a 20-byte field split of the full table):

- **`crates/pdftract-core/tests/fixtures/linearized-10.pdf`** (3,335 bytes):
  ONE classic xref section (subsection `0 26`, one `%%EOF`, one `startxref`
  2749), **no `/Prev`, no second section, no xref stream, no `/XRefStm`**.
  The `/Linearized` dict is object 1 at offset 36. **Every in-use entry is
  typed `f` instead of `n` (25 of 26 entries mislabelled)**, and many
  offsets are wrong: the entry for object 3 (/Root) says 390 — which holds
  `5 0 obj`; object 3 actually sits at 222. Entries 1, 2, 5 and 24 carry
  correct offsets; the rest are permuted but within ~1 KB of their true
  headers. A stray `1000 0 obj` (font) exists with NO table entry
  (Size 26 covers 0..25).
- **`crates/pdftract-core/tests/fixtures/multipage-100.pdf`** (1,024,504
  bytes): same shape — ONE classic section (`0 204`), no `/Prev`/stream.
  Every in-use entry typed `f` (203 of 204). Entries 1, 2 and 3 ALL record
  887 (which holds `3 0 obj`); the catalog (object 1) actually sits at 36,
  object 2 at 85; the remaining entries carry true offsets. /Root is
  `1 0 R`.

The bead's three hypotheses against multi-section resolution were evaluated
against HEAD code and **(a) and (b) are refuted**:

- (a) "the section walk stops after the first xref" — no;
  `load_xref_with_prev_chain`'s `walk_chain` follows `/Prev` with cycle
  detection (`HashSet<u64>`), a `MAX_PREV_DEPTH = 32` cap, and
  `STRUCT_INVALID_PREV_OFFSET` handling (xref.rs `walk_chain`).
- (b) "inverted merge order (earlier clobbering later)" — no; the walk
  inserts the current (newer) revision's entries OVER the recursive older
  result and keeps the newest trailer; `merge_linearized_xrefs` overlays the
  full section over the first-page section.
- (c) "offsets from earlier sections never consulted" — **confirmed, and it
  is the root cause** — but not for multi-section reasons: `/
  XrefEntry::Free` resolved to `NotFound` unconditionally; the entry's first
  field was never consulted.

Neither fixture is multi-section at all — the "multi-section" framing came
from the real-world classes the error signature was believed to match. The
fixtures are the **mislabelled-free single-section malformation**.

## Root cause

`XrefResolver::resolve_with_source` matched `XrefEntry::Free { .. }` with a
bare `Err(ResolveError::NotFound(obj_ref))`. Per spec the first field of a
free entry is a free-list LINK, not an offset — but these producers type
every in-use entry `f` with the entry's true byte offset in that field.
With /Root's own entry mislabelled free, `/Root` (and everything else)
returned "object N 0 R not found".

Before (HEAD `d7f7f834`, HEAD-built CLI):

```
$ pdftract extract --text - crates/pdftract-core/tests/fixtures/linearized-10.pdf
exit=1
Error: Failed to extract PDF: .../linearized-10.pdf
Caused by: Failed to parse catalog: Failed to resolve /Root: object 3 0 R not found
$ pdftract extract --text - crates/pdftract-core/tests/fixtures/multipage-100.pdf
exit=1
Error: Failed to extract PDF: .../multipage-100.pdf
Caused by: Failed to parse catalog: Failed to resolve /Root: object 1 0 R not found
```

## Fix (three parts, all in parser/xref.rs)

1. **Free-list coherence gate** (`free_list_is_coherent`): a section's Free
   entries are trusted as free iff they form the spec chain (head at free
   object 0, every free object visited exactly once, terminated at a 0
   next link). The verdict is computed lazily once per resolver
   (`OnceLock<bool>` on `XrefResolver`).
2. **Mislabelled-free recovery**: when the map's free fields are NOT a
   coherent free list and a Free entry's first field is nonzero, the
   resolver attempts the same verified parse as an in-use entry (strict
   "N G obj" header match at the offset, then the existing bounded
   `OBJECT_OFFSET_RECOVERY_WINDOW` rescan — factored into
   `resolve_object_at_offset`, shared with the InUse arm). A first field
   that really was a free-list link cannot resolve a wrong object: the
   parsed header must still match `obj_ref` exactly. Coherent free lists
   keep strict semantics (freed objects stay dead even though their
   superseded bytes remain in the file).
3. **Stream-object-first load in `load_single_xref`** (multi-section
   correctness for mixed chains): when `startxref` lands on an
   indirect-object header (`offset_lands_on_object_header`), the xref
   stream parser runs FIRST. Previously the traditional parser ran first
   and pdftract-0df07688's bounded keyword recovery could "recover" an
   OLDER revision's classic table sitting within
   `XREF_KEYWORD_RECOVERY_WINDOW` (1024 B) of a stream object's offset,
   silently dropping the newest revision's entries. Falls back to the
   traditional path when the object does not yield a usable xref stream.

`parser/catalog.rs` needed no change: `parse_catalog`'s /Root resolution is
correct; the failure was entirely upstream in entry resolution.

## Tests (written failing-first, then flipped)

- `crates/pdftract-core/tests/realworld_extraction_baseline.rs`: the two
  `pdftract-4196ae99` pin pairs flipped. Pre-fix run (evidence recorded
  pre-implementation): `core_linearized_10_resolves_root`,
  `desired_core_linearized_10_resolves_root`, `core_multipage_100_resolves_root`,
  `desired_core_multipage_100_resolves_root` all FAILED with exactly the
  two "Failed to resolve /Root" errors above; post-fix all four pass
  (linearized-10 → 10 pages; multipage-100 → 100 pages; PyMuPDF reports
  0 pp for both stubs — the pins assert pdftract resolution).
- `parser::xref` unit tests added (all pass):
  - `test_free_list_coherence_verdicts` — coherent / dangling-link /
    headless / unchained (the fixtures' shape) / cyclic verdicts.
  - `test_mislabelled_free_entry_recovers` — all-free section recovers
    catalog + mis-pointed entry via the window rescan; object 0 stays dead.
  - `test_coherent_free_list_keeps_freed_object_dead` — coherent list +
    freed object whose link value doubles as a byte offset landing exactly
    on its stale "5 0 obj" header: stays `NotFound` (strict semantics).
  - `test_prev_chain_mixed_classic_stream_later_overrides_earlier` — 3
    revisions classic → classic → xref stream: newest entry per object
    wins, newest /Root survives, bodies resolve from the newest offsets
    (depth ≥ 3 + classic/stream mix + override semantics).
  - `test_prev_chain_free_override_of_inuse_stays_strict` — a later
    revision's Free overrides an earlier InUse and stays dead under a
    coherent chain, even with a parseable stale header at the
    link-as-offset.
  - `test_merge_linearized_full_xref_overrides_first_page` — full section
    wins over first-page Free entries; full trailer is authoritative.

## After (fix, HEAD-built CLI)

```
$ pdftract extract --text - crates/pdftract-core/tests/fixtures/linearized-10.pdf
exit=0   (150 bytes: "Page 1 content…Page 10 content" — 10 pages)
$ pdftract extract --text - crates/pdftract-core/tests/fixtures/multipage-100.pdf
exit=0   (715,553 bytes of page text — 100 pages)
```

The real-world classes behind the shared signature (Docusign offer letter /
business report) keep extracting: the pdftract-7ec0f722 regression pins
`pin_incremental_updates_offer_letter_current_behavior` (/Prev chain, 3
%%EOF markers) and `pin_xref_stream_only_report_current_behavior`
(xref-stream-only) still pass.

## Verification record (clean extraction `/var/tmp/bead-4196ae99`, private target dir)

| Command | Result |
|---|---|
| `cargo test -p pdftract-core --lib parser::xref` (pristine HEAD baseline) | 96 passed / 9 failed — pre-existing: 8×`test_forward_scan_*`, `test_parse_multi_subsection_xref` |
| same, post-fix | **102 passed / 9 failed — identical failure set** (adds the 6 new passing tests) |
| `cargo test -p pdftract-core --lib parser::` post-fix | 856 passed / 12 failed; the 2 non-xref failures (`hint_stream::test_parse_hint_stream_full_minimal`, `object::cache` depth-tracking ×2) reproduced identically with PRISTINE xref.rs swapped in → pre-existing |
| `cargo test -p pdftract-core --test realworld_extraction_baseline` | 12 passed / 0 failed / 2 ignored (both ignored pins owned by pdftract-5b4c3d0e) — exit 0 |
| `cargo test -p pdftract-core --test test_cycle_detection --test test_basic_extraction` | 2 failures (`test_extract_base_hello`, `test_extract_tagged_pdf`) — identical with pristine xref.rs → pre-existing |
| `cargo build -p pdftract-cli` | exit 0 |
| CLI extracts (both fixtures) | exit 0, outputs above |
| `rustfmt --check` on both changed files | baseline file clean; xref.rs has only the 2 PRE-EXISTING drift regions (present at pristine HEAD, lines 586/6047) — none in the new code |

WARN (environmental, out of scope): no `scripts/definition-of-done.sh` in
this tree; the repo's bare `cargo test` is known-failing tree-wide (~300
pre-existing), so verification is by targeted `--test` targets as repo
CLAUDE.md mandates. Object 1000 (font) in linearized-10.pdf has no xref
entry at all and remains unresolvable (NotFound) — page text still
extracts; widening recovery to unlisted objects is a whole-file scan,
deliberately out of scope per the existing window-constant contracts.

Provenance: `tests/fixtures/profiles/PROVENANCE.md` untouched — no fixture
files were added or modified; both probe fixtures are the already-committed
ones. `scripts/check-provenance.sh` scope therefore unchanged by this bead.

## Post-push checklist run (step 2–4 of the dispatch checklist)

Re-verified from a FRESH `git archive HEAD` extraction of the pushed tip
`d1d47c6b` (`/var/tmp/bead-4196ae99-verify`, private target dir):

- `cargo test -p pdftract-core --test realworld_extraction_baseline` →
  12 passed / 0 failed / 2 ignored, exit 0
- `cargo test -p pdftract-core --lib parser::xref` → 102 passed / 9 failed,
  identical pre-existing set, exit 101 (not fenced)
- `cargo build -p pdftract-cli` → exit 0
- CLI extracts of both fixtures → exit 0 (150 B / 715,553 B)
- `git rev-list origin/main..HEAD` → empty (both commits pushed)
