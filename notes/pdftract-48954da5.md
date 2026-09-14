# pdftract-48954da5 — markdown-anchor public API contract enforcement

**Status: complete.** The four guarantees declared by
`docs/integrations/markdown-anchors.md` are now enforced by
`crates/pdftract-core/tests/markdown_anchor_contract.rs` (8 tests, all
passing) plus a supporting change to the anchor emission path in
`crates/pdftract-core/src/markdown.rs`.

## What was implemented

### 1. Contract test suite (new integration test target)

`crates/pdftract-core/tests/markdown_anchor_contract.rs` — 8 tests covering
every acceptance criterion of the bead:

| Guarantee | Test(s) |
|---|---|
| Regex schema freeze | `documented_regex_schema_is_frozen` — asserts `ANCHOR_REGEX_PATTERN` is byte-identical to all **three** occurrences of the pattern in the doc (Regex Schema fence, Python snippet, JS snippet), that the pinned pattern compiles, and that the stability-declaration wording ("stable public API", "will not change in a breaking way", the Rust API path) stays in the doc |
| Documented examples parse | `documented_examples_parse_with_documented_fields` — pins all four documented anchor examples byte-for-byte (heading p3/b12, heading p0/b0, paragraph p0/b1, table p1/b0; 1-decimal bbox precision verified via `Anchor::to_comment()` round-trip), and asserts the schema snippets in the doc do **not** parse as anchors |
| Round-trip property | `roundtrip_recovers_block_list_across_pages` (multi-page, per-page block indexing restarts at 0), `roundtrip_property_recovers_block_list` (proptest over generated block lists), both driven through **both** emitters (`page_to_markdown_with_options` and `page_to_markdown_with_links_and_footnotes`) — every block yields exactly one anchor with matching (page, block, bbox, kind), in document order; inline styling documented as lossy and not compared |
| Code-fence edge case | `code_block_text_is_passed_through_verbatim_inside_fence` (emission side) + `parse_anchors_is_a_flat_scan_fences_are_not_skipped` (parse side: `parse_anchors` is documented flat-scan behavior — fenced anchor-like comments still parse) |
| Empty-block emission | `empty_block_emits_anchor_with_empty_content` — empty paragraph still emits its anchor with correct page/block/bbox/kind |
| End-to-end over fixtures | `fixture_pdf_roundtrip_when_extraction_available` — runs the anchor↔block assertion on every `tests/fixtures/*.pdf` whose extraction succeeds; pre-existing extraction failures are reported and skipped (tracked outside this contract, cf. the 2026-09-14 extract breakage) |

### 2. Public regex constant + per-list-item anchors (`markdown.rs`)

- `ANCHOR_REGEX_PATTERN` is now a documented `pub const` — the schema-freeze
  test pins it against the doc, and `anchor_regex()` builds from it so the
  implementation cannot drift from the pinned constant.
- New `block_anchor_comment()` helper shared by **all** emission paths so
  every anchored output carries identical syntax.
- **Behavioral fix required by the round-trip contract**: consecutive list
  runs previously emitted **one anchor for the whole run** (plain path) or an
  anchor only for the run leader (links/footnotes path). Both paths now emit
  **one anchor per list item** with the correct per-item block index
  (`i + offset`), matching the documented "one anchor per block, block index
  per-page" contract. List indentation logic extracted unchanged into
  `emit_indented_list_line()` so both paths share it.

## Verification (run on a clean `git archive` candidate = HEAD 172ad527 + the
two files above, private target dir, 2026-09-14)

- Gate `default_rust` (`cargo check --all-targets --quiet`): **exit 0**
  (127-warning pre-existing debt unchanged; no errors).
- `cargo test -p pdftract-core --test markdown_anchor_contract`:
  **8 passed, 0 failed**.
- `cargo test -p pdftract-core --lib markdown`: 117 passed, 2 failed —
  `test_spans_to_markdown_with_links_and_footnotes_footnote_takes_precedence`
  and `test_block_to_markdown_formula_display`. **Both fail identically on a
  pure-HEAD extraction without these changes** → pre-existing at HEAD
  (part of the documented ~300 pre-existing failures), not caused by this
  work.

## Notes for future workers

- The working tree also carries an uncommitted `document.rs` diff removing
  unused test imports (`intern`, `ObjRef`, `parse_catalog`, etc.) — that is
  gate-warning hygiene flagged by an earlier attempt's gate output, not part
  of this contract; it was deliberately **left uncommitted** here to avoid
  sweeping a possibly in-flight edit. Landing it is safe whenever its owner
  is unclear (pure dead-import removal).
- The 4 E0277 `catch_unwind`/`UnwindSafe` errors seen when checking the
  shared working tree come from another worker's in-flight uncommitted
  `parser/xref.rs` strand (`source: Option<Arc<dyn PdfSource>>` field on
  `XrefResolver`); they do not exist at HEAD and are unrelated to this bead.
