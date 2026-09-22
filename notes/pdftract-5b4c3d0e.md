# pdftract-5b4c3d0e — Fix silent empty text layer for escaped parentheses in literals

Base HEAD: `2a931921` (test(pdftract-bcf935ec): pin MCP extract_text content for
both valid-minimal fixtures). Supersedes attempt 1's note (2026-09-22T13:46Z,
base `3d7a94b0`, recovered from `stash@{0}` / the WIP trace patch) — same root
cause and fix, re-verified at today's HEAD where the sibling-bead pins
(pdftract-4196ae99 / bcf935ec / 4683109d / 0df07688) are already flipped, so the
baseline harness now runs 14 pins with zero ignores.

## Root cause

`crates/pdftract-core/src/parser/lexer/mod.rs`, `lex_literal_string()`: the
`\(` escape arm incremented the paren-balancing `depth`. Per PDF
32000-1:2008 §7.3.4.2 an escaped `\(` is literal content and does NOT open a
nesting level (the `\)` arm already handled this correctly — the two arms were
asymmetric). With `\(` counted as depth, a literal like `(a \(b\) c)` never
terminated at its own `)` — the closing paren was consumed as content and the
string swallowed every following byte of the content stream (all subsequent
`Tj`/`TJ` operators through `ET`) until EOF, where the lexer emitted an
"Unterminated literal string" diagnostic and returned one giant string token
with no operator after it. The text interpreter executed zero text-showing
operators → extraction returned Ok with 1 page and an empty text layer, no
error — the silent failure class pinned in pdftract-7ec0f722.

Repro isolation (from the bead / notes/pdftract-7ec0f722.md, confirmed here by
the flipped pins): dense-one-page-agreement.pdf extracts 0 chars; the
byte-identical document with the parens replaced by a dash extracts 2,014.

## Fix

- `crates/pdftract-core/src/parser/lexer/mod.rs`: removed `depth += 1;` from
  the `\(` arm (one line + comment). The `\(` arm now mirrors the `\)` arm:
  push the byte, leave depth alone.
- `lexer/mod.rs` tests: `string_literal_escape_left_paren` had encoded the
  buggy behavior (asserted `(\(nested))` lexes as the single balanced string
  `(nested)`); corrected to spec behavior — string `(nested`, then the stray
  `)` yields the existing diagnostic + `Token::Null` placeholder. Added
  `string_literal_escaped_parens_both_stay_literal` for the minimal
  `(a \(b\) c)` shape from the bead.
- `crates/pdftract-core/src/content_stream.rs` tests: added
  `test_process_with_mode_escaped_parens_keep_text_and_trailing_ops` —
  `BT (a \(b\) c) Tj (end) Tj ET` produces text `a (b) cend`, proving both the
  string content and the trailing operators survive.
- `crates/pdftract-core/tests/realworld_extraction_baseline.rs`: flipped the
  pins owned by this bead:
  - `pin_dense_one_page_agreement_current_behavior`: asserts-empty → asserts
    `contains("MUTUAL NON-DISCLOSURE")` (now a regression pin on the text
    layer itself).
  - `desired_dense_one_page_agreement_extracts_text`: `#[ignore]` removed.
  - `desired_startxref_offset_edge_extracts_like_pymupdf`: `#[ignore]` removed
    — its ignore reason named this bead as the only remaining blocker, and it
    passes with the fix (the trailer-loss half was already fixed by
    pdftract-0df07688).
  - Header/class doc comments updated to record both classes as FIXED.

The working tree this was developed in carries ~50 stranded uncommitted edits
in OTHER files (shared checkout); none of the files above were dirty before
this dispatch (the baseline file's stale pre-flip worktree/index copies were
set aside to `/var/tmp/pdft-5b4c3d0e/stale-worktree-baseline.rs.bak` and
restored to HEAD before editing — the stale staged blob would have reverted
pdftract-4196ae99/4683109d pins had it been committed).

## Verification (clean `git archive HEAD` extraction at 2a931921 with the fix
applied; private CARGO_TARGET_DIR; never the shared dirty tree)

```
cargo test -p pdftract-core --lib -- lexer paren
  → 127 passed; 1 failed — the 1 is cache::multi_process::
    test_write_creates_parent_dirs (environment-dependent, PRE-EXISTING:
    fails identically at pristine HEAD, see A/B below).
    All string-literal lexer tests pass, including the rewritten
    string_literal_escape_left_paren, the new
    string_literal_escaped_parens_both_stay_literal, and the untouched
    string_literal_balanced_parens / _deeply_nested_parens /
    _escape_right_paren; content_stream
    test_process_with_mode_escaped_parens_keep_text_and_trailing_ops passes.

A/B attribution run at pristine HEAD (same extraction, fix files reverted):
  filter "test_write_creates_parent_dirs string_literal" → same cache test
  FAILED; old-behavior string_literal tests pass. So the cache failure is
  not caused by this change.

cargo test -p pdftract-core --test realworld_extraction_baseline --no-fail-fast
  → EXIT=0, 14 passed; 0 failed; 0 ignored — both flipped dense pins green,
    desired_startxref_offset_edge_extracts_like_pymupdf green (2 pp +
    MUTUAL NON-DISCLOSURE), all sibling-class pins (offer letter, business
    report, linearized-10, multipage-100, valid-minimal both copies) green.

cargo test -p pdftract-core --lib --no-fail-fast   (full lib suite A/B)
  → with fix: 3559 passed; 107 failed (x2 result sections).
    pristine HEAD: 3557 passed; 107 failed.
    Failure name-sets byte-identical pre/post (sort/diff of grep FAILED) —
    all 107 pre-existing (xref forward-scan family, render::scanline,
    threads::walk_beads, parser::object::cache, ...). Only pass-count delta
    is this change's rewritten + newly added tests. Zero regressions.

cargo test -p pdftract-core --test ocr   (corpus extraction gate)
  → EXIT=0, 10 passed; 0 failed.

cargo build -p pdftract-cli --bin pdftract  → EXIT=0
./target/debug/pdftract extract tests/fixtures/realworld/dense-one-page-agreement.pdf
  → 1 page, non-empty text; "MUTUAL NON-DISCLOSURE" ×2 (was 0 chars total
    before the fix — the silent-empty repro).
./target/debug/pdftract extract tests/fixtures/realworld/startxref-offset-edge.pdf
  → 2 pages (PyMuPDF parity), "MUTUAL NON-DISCLOSURE" ×4.
```

## Acceptance criteria

1. `dense-one-page-agreement.pdf` extracts non-empty text containing MUTUAL
   NON-DISCLOSURE at HEAD — **PASS** (`pin_dense_one_page_agreement_current_behavior`
   + `desired_dense_one_page_agreement_extracts_text` + CLI probe). No parser
   regression on `tests/fixtures/realworld/` siblings — **PASS** (full baseline
   suite green; lib-suite failure set byte-identical to pristine HEAD).
2. Harness pins flipped; `cargo test -p pdftract-core --test
   realworld_extraction_baseline` green apart from remaining class pins —
   **PASS** (14 passed / 0 failed / 0 ignored at this HEAD; the previously
   remaining ignores belonged to beads whose fixes have since landed, plus
   this bead's two, both now flipped).
3. This note + commands + HEAD — this file.
