# pdftract-d0b22a6e — xref trailer-loss characterization at HEAD

**Tested tree:** `git archive HEAD` extraction of
`db783e405f80733493b77f3ad2f6ba387c6ed599` into `/var/tmp/pdftract-d0b22a6e`
(the shared working tree was **not** used — it does not compile as a union;
`parser/pages.rs` carries a stranded in-flight refactor). Private
`CARGO_TARGET_DIR=/var/tmp/pdftract-d0b22a6e-target`. Every command below ran
in that extraction, timeout-wrapped, single run, no overlapping retries.

**Artifact:** `mod trailer_loss_probe` appended to
`crates/pdftract-core/src/parser/xref.rs` — test-only, additions-only diff vs
HEAD (440 added lines, 0 deleted; verified with `diff` against
`git show HEAD:...`). Six probe tests, **all PASS at HEAD** (they pin current
behavior; the two `CHARACTERIZATION FLIP POINT` sites tell the fixing child
which asserts to flip).

## Verdict

**No parser layer drops the trailer of a well-formed minimal PDF at HEAD.**
The premise this bead inherited from parent pdftract-3628e890's 2026-09-05
note (5/5 smoke failures; corpus 0/1377 with "No /Root reference in trailer")
is stale at `db783e40`:

1. `test_sdk_smoke` is **5/5 PASS** at db783e40. Since `66464ef7` (2026-09-17,
   "Fix real PDF page discovery smoke contract") all five tests use
   `tests/fixtures/test-minimal.pdf` — not the `valid-minimal.pdf` the parent
   note characterized.
2. Layer probe over test-minimal.pdf: every layer returns the full trailer —
   keys `["Size", "Root", "Info", "ID"]`, `/Root` present, 0 diagnostics —
   and `parse_pdf_file` returns `OK (pages=1)`. Nothing is lost anywhere.
3. `valid-minimal.pdf` (583 B, last touched 2026-06-01 in `a44814a0`) is
   itself malformed **two independent ways**; it is the only minimal fixture
   for which either error string still appears:
   - **(a) Self-inconsistent `startxref`.** Its trailer says `startxref 439`
     but the `xref` keyword is at byte **406** (od-verified; byte 439 is the
     6th digit of entry 1, `0000000009 00000 n`, which starts at 434).
     `parse_traditional_xref` therefore rejects at its header check
     (`xref.rs:532`, `XrefInvalidHeader` "xref keyword not found");
     `load_single_xref`'s `parse_xref_stream` fallback fails too; the /Prev
     chain propagates `trailer = None`; `document.rs:422` raises
     **"No trailer in xref section"**.
   - **(b) 19-byte xref entries.** Even at the *corrected* offset 406, the
     six 19-byte entries (bare-LF terminated; spec requires 20-byte entries)
     parse under the 20-byte window — the spill byte after the entry type
     separates as whitespace, so the 19-byte fallback (`xref.rs:754-757`) is
     dead code for this shape. `pos` drifts +1 B/entry (stride set at
     `xref.rs:684`, advanced at `xref.rs:746`), and after six entries it lands
     on the second `r` of `t-r-a-i-l-e-r` (415 + 6×20 = 535). The
     `trailer` keyword check (`xref.rs:586`) misses; the parser then walks
     `<< /Size 6 /Root 1 0 R >>`, `startxref`, `439`, `%%EOF` as junk
     subsection headers (observed as `Invalid subsection header: r`, … diags)
     and ends with `XrefTrailerNotFound` "Trailer dictionary not found (xref
     table may be truncated)" (`xref.rs:771`). `parse_trailer_dict`
     (`xref.rs:946`) is **never reached**.
4. **The two error strings are one root cause reported by two site
   families** — not two bugs. Real-run mapping on valid-minimal.pdf (probe
   output below):

   | entry point | internal path | error string |
   |---|---|---|
   | `sdk::extract` | `extract_pdf` → `extract.rs:659` | `No /Root reference in trailer` |
   | `sdk::extract_text` | `extract_text` (`extract.rs:1650`) | `No /Root reference in trailer` |
   | `sdk::extract_stream` | `extract_pdf_streaming` → `extract.rs:2131` | per-page `Extraction failed: No /Root reference in trailer` |
   | `sdk::get_metadata` | `parse_pdf_file` → `document.rs:422` | `No trailer in xref section` |
   | `sdk::hash` | `parse_pdf_file` → `document.rs:422` | `No trailer in xref section` |

   The extract.rs sites use `.trailer.as_ref().and_then(|t| t.get("Root"))`,
   which collapses *trailer absent* into **"No /Root reference in trailer"**;
   document.rs distinguishes the two cases. (Parent's 2026-09-05 note recorded
   the mirror-image 2/3 split because the smoke tests then hit
   valid-minimal.pdf through different entries.)

## AC1 — test_sdk_smoke per-test outcomes at db783e40

| test | outcome | error string |
|---|---|---|
| `test_sdk_extract_basic` | **PASS** | — |
| `test_sdk_extract_text` | **PASS** | — |
| `test_sdk_get_metadata` | **PASS** | — |
| `test_sdk_hash` | **PASS** | — |
| `test_sdk_extract_stream` | **PASS** | — |

Raw output (see "Commands"): `test result: ok. 5 passed; 0 failed; 0
ignored; 0 measured; 0 filtered out` — exit 0. There is no failing entry
point on the checked-in smoke surface at HEAD; for the failure *strings* on
the broken fixture see the table in §Verdict.4 and the probe output below.

## AC2 — first-layer-lost verdict (file:line)

For valid-minimal.pdf the trailer is never *dropped* by a layer that had it —
it is *never found*:

- **As shipped (stated `startxref: 439`):** first failure is the xref-keyword
  check inside `parse_traditional_xref` —
  `crates/pdftract-core/src/parser/xref.rs:532` (fn at `:489`). Downstream
  raiser: `crates/pdftract-core/src/document.rs:422`.
- **At the corrected offset 406:** first loss is the entry-stride handling in
  `parse_traditional_xref`'s entry loop — stride init `xref.rs:684`, window
  parse `xref.rs:714-727`, per-entry advance `xref.rs:746`, dead 19-byte
  fallback `xref.rs:754-757` — which makes the `trailer` keyword check at
  `xref.rs:586` miss. `parse_trailer_dict` (`xref.rs:946`) is never reached;
  `parse_xref_stream` (`xref.rs:1584`) appears only as the failed fallback.

## Corpus check (parent context: "0/1377, No /Root" on 2026-09-15)

20-file sample of the checked-in corpus at HEAD via `parse_pdf_file`
(`probe_corpus_sample_entry_errors`): **15 OK** (1–19 pages), **5 fail** — all
five as `Failed to parse catalog: Failed to resolve /Root: object 1 0 R not
found` (`tests/fixtures/profiles/book_chapter/*.pdf`). That is a *third,
different* failure: trailer and `/Root` are found; the resolver cannot resolve
object `1 0 R`. The 0/1377 claim does not hold at HEAD, and the residual
corpus failure is not the trailer-parse bug this chain targets.

## Hypothesis handed to children 2 and 3

- **Child 2 (trailer-parse fix):** the actionable parser defect is 19/20-byte
  stride detection in `parse_traditional_xref` (`xref.rs:684-757`): a 20-byte
  window over a 19-byte-stride table usually still parses, so the fallback
  never triggers and the drift eats the `trailer` keyword. Candidate fix:
  accept stride 20 only when the window's EOL field matches, or derive stride
  from the first entry's EOL. Note that fixing stride alone still leaves the
  shipped fixture unparseable — `valid-minimal.pdf` must **also** get its
  `startxref` value corrected (439 → 406) or the parser gains a backward-scan
  recovery. The flip points are marked `CHARACTERIZATION FLIP POINT (child 2)`
  in the probe; flip them and re-check `sdk::extract` on valid-minimal.pdf.
- **Child 3 (/Root fix):** there is **no /Root-resolution bug** on this path
  at HEAD. The defensible change is the error-string conflation at
  `extract.rs:659/1762/2131`: report "No trailer in xref section" when
  `xref_section.trailer.is_none()` and reserve "No /Root reference in
  trailer" for a present trailer lacking the key. Also note `sdk.rs:310`
  queries `trailer.get("/Encrypt")` **with** the leading slash while every
  other site queries without it; dict keys are stored slash-less (probe prints
  `["Size", "Root", "Info", "ID"]`), so that check is currently dead.
- **Do not touch** `document.rs` trailer resolution, `load_xref_with_prev_chain`,
  `merge_hybrid` / `merge_linearized_xrefs` — all verified intact at HEAD.
- The residual corpus failure (`Failed to resolve /Root: object 1 0 R not
  found`, book_chapter fixtures) is resolver/catalog-layer, out of this
  chain's scope; see `probe_corpus_sample_entry_errors` output.

## Commands + raw outputs

All in the extraction `/var/tmp/pdftract-d0b22a6e` (HEAD `db783e40`), private
target dir:

```
$ CARGO_TARGET_DIR=/var/tmp/pdftract-d0b22a6e-target timeout --kill-after=30s 600s \
    cargo test -p pdftract-core --test test_sdk_smoke
# exit=0
running 5 tests
test test_sdk_extract_stream ... ok
test test_sdk_extract_basic ... ok
test test_sdk_extract_text ... ok
test test_sdk_get_metadata ... ok
test test_sdk_hash ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.02s
```

```
$ CARGO_TARGET_DIR=/var/tmp/pdftract-d0b22a6e-target timeout --kill-after=30s 600s \
    cargo test -p pdftract-core --lib trailer_loss_probe -- --nocapture --test-threads=1
# exit=0 — test result: ok. 6 passed; 0 failed; 0 ignored; 0 measured; 3632 filtered out
# (excerpts; full capture retained in the bead's close evidence)

[test-minimal.pdf] LAYER 2 parse_traditional_xref (xref.rs:489): entries=16 trailer=true keys=["Size", "Root", "Info", "ID"] hybrid=false diags=0
[test-minimal.pdf] LAYER 1 load_single_xref (xref.rs:2212): entries=16 trailer=true keys=["Size", "Root", "Info", "ID"] hybrid=false diags=0
[test-minimal.pdf] LAYER 3 parse_trailer_dict (xref.rs:946): keys=["Size", "Root", "Info", "ID"] diags=0
[test-minimal.pdf] LAYER 4 load_xref_with_prev_chain (xref.rs:2293): entries=16 trailer=true keys=["Size", "Root", "Info", "ID"] hybrid=false diags=0
[test-minimal.pdf] LAYER 5 parse_xref_stream (xref.rs:1584) [informational]: entries=0 trailer=false keys=[] hybrid=false diags=1
[test-minimal.pdf]   diag: Failed to parse xref stream as indirect object
[test-minimal.pdf] parse_pdf_file: OK (pages=1 pages_ref=ObjRef { object: 4, generation: 0 })

[valid-minimal.pdf] LAYER 2 parse_traditional_xref (xref.rs:489) [at stated offset 439]: entries=0 trailer=false keys=[] hybrid=false diags=1
[valid-minimal.pdf]   diag: xref keyword not found
[valid-minimal.pdf] parse_pdf_file: ERR No trailer in xref section

[valid-minimal.pdf] LAYER 2 parse_traditional_xref (xref.rs:489) [at corrected offset 406]: entries=6 trailer=false keys=[] hybrid=false diags=6
[valid-minimal.pdf]   diag: Invalid subsection header: r
[valid-minimal.pdf]   diag: Invalid subsection header: << /Size 6 /Root 1 0 R >>
[valid-minimal.pdf]   diag: Invalid subsection header: startxref
[valid-minimal.pdf]   diag: Invalid subsection header: 439
[valid-minimal.pdf]   diag: Invalid subsection header: %%EOF
[valid-minimal.pdf]   diag: Trailer dictionary not found (xref table may be truncated)

[valid-minimal.pdf] sdk::extract: ERR No /Root reference in trailer
[valid-minimal.pdf] sdk::extract_text: ERR No /Root reference in trailer
[valid-minimal.pdf] sdk::get_metadata: ERR No trailer in xref section
[valid-minimal.pdf] sdk::hash: ERR No trailer in xref section
[valid-minimal.pdf] sdk::extract_stream: OK ("first page ERR Extraction failed: No /Root reference in trailer")

[corpus sample] checked=20 files
[corpus sample]   5 x Failed to parse catalog: Failed to resolve /Root: object 1 0 R not found  e.g. ["/var/tmp/pdftract-d0b22a6e/crates/pdftract-core/../../tests/fixtures/profiles/book_chapter/academic_chapter.pdf", ".../novel_chapter.pdf"]
[corpus sample]   8 x OK(pages=1)   e.g. [".../grep-corpus/corpus/synthetic_104.pdf", ".../classifier/contract/01.pdf"]
[corpus sample]   1 x OK(pages=11)  e.g. [".../grep-corpus/corpus/synthetic_10.pdf"]
[corpus sample]   1 x OK(pages=12)  e.g. [".../grep-corpus/corpus/synthetic_103.pdf"]
[corpus sample]   1 x OK(pages=19)  e.g. [".../grep-corpus/corpus/synthetic_101.pdf"]
[corpus sample]   1 x OK(pages=5)   e.g. [".../grep-corpus/corpus/synthetic_1000.pdf"]
[corpus sample]   2 x OK(pages=6)   e.g. [".../grep-corpus/corpus/synthetic_100.pdf", ".../synthetic_102.pdf"]
[corpus sample]   1 x OK(pages=7)   e.g. [".../grep-corpus/corpus/synthetic_1.pdf"]
```

Fixture byte-level verification (od, offset 400-470 and tail):

```
0000400   n   d   o   b  j  \n   x   r   e   f  \n   0       6  \n   0
0000416   0   0   0   0   0   0   0   0   0       6   5   5   3   5
0000432   f  \n   0   0   0   0   0   0   0   0   0   9       0   0   0
0000448   0   0       n  \n   ...
0000555       0       R       >   >  \n   s   t   a   r   t   x   r   e
0000571   f  \n   4   3   9  \n   %   %   E   O   F  \n
```

→ `xref` keyword at 406; `startxref` value 439; entry 1 starts at 434, so 439
is inside its digits.

## Acceptance criteria

- **AC1 PASS** — per-test table above, from real runs (exit codes 0), named
  HEAD sha, raw output in this note.
- **AC2 PASS** — first-layer verdict stated as file:line above; probe checked
  in (test-only) with output recorded here.
- **AC3 PASS** — commit touches only `crates/pdftract-core/src/parser/xref.rs`
  (test-only append; additions-only diff vs HEAD) and this note; zero
  production code changes.
- **AC4 PASS** — this note is checked in at `notes/pdftract-d0b22a6e.md` and
  cited in the close reason.
