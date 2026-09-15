# pdftract-22f3772c — Markdown anchor stability contract and round-trip enforcement tests

Bead: `pdftract-22f3772c` (Phase 6.5 coordinator `pdftract-1xrn0`)
Date: 2026-09-15
Status: **CLOSED — substantive criteria PASS; two WARN items documented below**

## What was done

`docs/integrations/markdown-anchors.md` declares the anchor HTML-comment format a
stable public API with a published regex schema and a round-trip property, but no
test enforced either claim. This bead lands the enforcement suite
`crates/pdftract-core/tests/markdown_anchor_contract.rs` — the schema-freeze test
already named by the `ANCHOR_REGEX_PATTERN` doc comment in
`crates/pdftract-core/src/markdown.rs` — and one behavioral fix the suite requires.

### 1. New enforcement suite (14 tests, all PASS)

| Test | Enforces |
|---|---|
| `published_doc_regex_is_byte_identical_to_anchor_regex_constant` | doc's ` ```regex ` fence ≡ `ANCHOR_REGEX_PATTERN` (the byte-identity markdown.rs requires) |
| `doc_python_and_javascript_examples_match_published_regex` | doc's Python and JS example patterns ≡ the constant (extracts the doc's own text, so drift fails) |
| `every_emitted_anchor_matches_published_regex_and_count_equals_blocks` | every anchor from every emission path matches the published schema (strict 1-decimal bbox shape, negative components admitted); anchor count == block count; no malformed `<!-- pdftract:` comment escapes |
| `block_index_is_per_page_not_global` | page 1 restarts at block 0 (a global counter would emit 8..10) |
| `block_to_markdown_emits_requested_page_and_block_index` | single-block path honors the caller's (page, block) |
| `round_trip_recovers_block_list_from_emitted_markdown` | emission → `parse_anchors` recovers count, order, exact (page, block), exact kind, bbox within 1-decimal tolerance (±0.051); non-empty blocks have content after their anchor, empty blocks none |
| `anchors_preserve_document_order` | `parse_anchors` returns anchors in emission order |
| `round_trip_from_synthesized_pdf` | end-to-end: synthesized 2-page PDF → `extract_pdf` → CLI-shaped anchored markdown → `parse_anchors` → recovered blocks (see WARN 1) |
| `empty_blocks_still_emit_anchors` | doc edge case: empty blocks emit one anchor each, empty content following |
| `anchor_like_comment_inside_code_fence_is_never_an_anchor` | doc edge case: anchor-like comment inside a fence stays verbatim content but never false-positives in `parse_anchors`; round-trip count holds |
| `parse_anchors_ignores_anchor_like_text_inside_fences` | unit-level: fences hide anchor-like text; real anchors around it still parse |
| `parse_anchors_handles_tilde_fences_and_fence_like_content` | `~~~` fences, nested fence runs, shorter same-char runs |
| `real_anchors_are_never_emitted_inside_fences` | emission-side guarantee: anchors only at fence depth 0 |
| `parse_anchors_and_anchor_remain_public_exports` | `pdftract_core::{parse_anchors, Anchor}` and `pdftract_core::markdown::{...}` both compile and round-trip |

### 2. Behavioral fix the suite requires: fence-aware `parse_anchors`

The doc's code-fence edge case and the round-trip property were contradictory
before this bead: `emit_code_block` emits code text verbatim (documented
behavior), so a PDF whose code content quotes the anchor format would inject a
phantom anchor into every downstream `parse_anchors` call and break the
recovered block list. `parse_anchors` is now fence-aware (CommonMark ``` / ~~~
delimiters, 0-3 space indent, same-char/≥-length closing rule): anchor-like
comments inside fences are never anchors. This is backward compatible — pdftract
has always emitted real anchors at fence depth 0, so every previously-parseable
anchor still parses. Content is untouched; sanitizing emitted code was rejected
because the doc explicitly promises verbatim fence emission and mutation would
damage fidelity. Doc updated accordingly (Code Fences section), plus a note that
raw-regex integrators should apply the same skip rule.

### 3. File lineage: replaces the pdftract-48954da5 suite (disclosure)

`crates/pdftract-core/tests/markdown_anchor_contract.rs` already existed at
commit `a2b62662` (bead `pdftract-48954da5`) but was **deleted — uncommitted —
in the shared working tree** before this dispatch; this commit's suite replaces
it. The replacement is deliberate and directional: the old suite's
`parse_anchors_is_a_flat_scan_fences_are_not_skipped` asserted the exact
opposite of this bead's requirement ("anchor-like HTML comments inside
code-fence content ... would false-positive in parse_anchors and break
round-trip"). The flat-scan assertion contradicted the doc's own round-trip
claim, which is the contradiction this bead resolves. All other old-suite
coverage is preserved by the new suite (schema freeze incl. the three doc
pattern occurrences, documented-example grammar, cross-page round-trip,
verbatim fence content, empty-block anchors, extraction-gated fixture leg —
now synthesized-PDF based). Two intentional deltas: the flat-scan test is
reversed (fences ARE skipped), and the proptest-generated property is replaced
by fixed fixtures covering the contract's corners (list runs, empty text,
fractional and negative bboxes, tilde fences).

### 4. Pre-existing stranded amendment included (disclosure)

`markdown.rs` and `docs/integrations/markdown-anchors.md` were dirty before this
dispatch with an uncommitted negative-bbox amendment: bbox charset `[\d.,]` →
`[-\d.,]` in the frozen regex constant and the doc's regex/Python/JS patterns,
plus the doc bullet documenting negative MediaBox origins. The amendment is a
prerequisite for this bead's tests (the byte-identity tests pin the doc and code
together; the round-trip fixture includes negative components the old charset
would have silently dropped). It rides in this commit; the files also carry this
bead's own changes (fence-aware `parse_anchors` + Code Fences doc section), per
the dirty-file rule.

## Verification

```
CARGO_TARGET_DIR=<isolated> cargo test -p pdftract-core --test markdown_anchor_contract
→ test result: ok. 14 passed; 0 failed; 0 ignored; 0 measured
```

- In-module markdown lib tests: identical with and without the change
  (117 passed; 2 pre-existing failures `test_block_to_markdown_formula_display`,
  `test_spans_to_markdown_with_links_and_footnotes_footnote_takes_precedence` —
  formula/footnote tests, unrelated to anchors, failing at pristine HEAD too).
- Doctests: the 9 failures in `span_to_markdown`/`spans_to_markdown_with_links`/
  `block_to_markdown_with_links`/`page_to_markdown_with_links` pre-exist at
  pristine HEAD; both `parse_anchors` doctests (original + new fenced-content
  one) pass. `rustfmt --check` clean on both files. No clippy warnings from the
  new code (remaining markdown.rs clippy warnings pre-exist outside the edited
  regions; the dirty tree's clippy errors in `layout/correction.rs` and
  `parser/xref.rs` are other workers' in-flight files / pre-existing).
- `cargo check -p pdftract-cli`: compiles.

## WARN items

1. **WARN — end-to-end PDF round-trip is waiver-gated.** `extract_pdf` fails for
   every PDF at HEAD with the pre-existing "Document contains no pages" error
   (reproduced on a freshly synthesized, pypdf-validated two-page PDF, at clean
   HEAD via git-archive — not caused by in-flight tree edits, and older than this
   bead; fixing it is out of scope here). While that stands,
   `round_trip_from_synthesized_pdf` prints a loud WAIVER and enforces only the
   error surface; the full round-trip assertions engage automatically the moment
   extraction succeeds (or fails any other way, which then panics). Emission-level
   round-trip — the documented property's actual surface — is fully enforced now.
2. **WARN — extraction breakage is tracked separately.** The synthesized-PDF
   builder in the test was byte-validated against pypdf while authoring; if the
   waiver path keeps firing after extraction is fixed, the diagnostic points at
   block segmentation.

## Artifacts

- `crates/pdftract-core/tests/markdown_anchor_contract.rs` (new, 14 tests)
- `crates/pdftract-core/src/markdown.rs` (fence-aware `parse_anchors`, `fence_marker`,
  `anchor_from_captures`, doc comments)
- `docs/integrations/markdown-anchors.md` (Code Fences section: emission and
  parse-side guarantees)
