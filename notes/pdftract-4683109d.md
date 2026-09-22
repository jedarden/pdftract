# pdftract-4683109d — fix silent empty text layer on wrong xref object offsets

- **Date:** 2026-09-22
- **Parent bead:** pdftract-257d92c3 (harness/fixtures: pdftract-7ec0f722)
- **Verified at:** HEAD 40b48b17 (parent of the commit landing this note) in a
  clean `git archive HEAD` extraction at /var/tmp/p468-head, private
  CARGO_TARGET_DIR=/build/scratch/p468-target. All results below were
  reproduced on the committed tree, never the shared working tree (which
  carries other workers' in-flight edits and does not compile as a union).

## Root cause (confirmed by code trace + probes)

For `tests/fixtures/valid-minimal.pdf` (repo-root W3C dummy, 534 bytes):

    obj 1 true header at 9    recorded 9
    obj 2 true header at 58   recorded 58
    obj 3 true header at 115  recorded 115
    obj 4 true header at 290  recorded 298   <-- 8 bytes PAST the header
    true xref keyword at 376  startxref says 403

`XrefResolver::resolve_with_source` (crates/pdftract-core/src/parser/xref.rs)
read 4 KiB at the recorded offset 298 — 8 bytes into the object, so the
buffer starts at the object body, not its header. `parse_indirect_object`
hits `<<` where it expects the object number, emits
STRUCT_INVALID_INDIRECT_HEADER into the parser-local diagnostics buffer
(which the resolve path drops), and returns None. The resolver then returned
`ResolveError::NotFound`. Downstream, `decode_page_content_streams`
translated that into `PageExtractionError::ContentStreamDecodeFailed`, the
page loop pushed a page with `spans: []` and `error: Some(..)`, and
`extract_pdf` still returned Ok — so `sdk::extract_text` produced an empty
string with no surface error. Silent empty output, exactly as pinned.

## Fix

In `resolve_with_source` (crates/pdftract-core/src/parser/xref.rs):

- The parse + id-verify + stream-offset fixup + cache sequence is factored
  into `parse_verified_object(obj_ref, bytes, base)`; behavior on the strict
  path is unchanged.
- When the strict read at the recorded offset does not yield exactly the
  requested object (parse failure OR id mismatch), a bounded rescan
  re-reads the window `[offset - OBJECT_OFFSET_RECOVERY_WINDOW, offset +
  4096)` (`OBJECT_OFFSET_RECOVERY_WINDOW = 1024`, mirroring the existing
  `XREF_KEYWORD_RECOVERY_WINDOW` shape) and scans it for the literal header
  needle `"N G obj"` of the requested object. Each hit is parsed with a
  fresh `ObjectParser` and accepted only on full parse + id match; the
  stream-offset fixup uses the *found* offset. A whole-file scan is
  deliberately out of scope. Well-formed documents never reach the rescan
  (the strict attempt succeeds first), so strict-xref behavior is
  unchanged.

## Side effect on a sibling class pin (documented flip, not drift)

`crates/pdftract-core/tests/fixtures/valid-minimal.pdf` — pinned by
`pin_core_valid_minimal_current_behavior` as the
"Document contains no pages" class (owned by pdftract-bcf935ec) — carries
the same drifted-entry class; it was failing one hop earlier in the page
tree:

    obj 3 true header at 117  recorded 115   <-- page-tree walk killer
    obj 4 true header at 243  recorded 268
    obj 5 true header at 313  recorded 345
    true xref keyword at 406  startxref says 439

The rescan recovers this document too: extraction now returns 1 page and
"Hello World" (PyMuPDF parity: 1 pp / 12 chars). Both bcf935ec pins
(`pin_core_valid_minimal_current_behavior`,
`desired_core_valid_minimal_page_tree_resolves`) were flipped to assert the
new reality with comments attributing the flip to pdftract-4683109d. Any
remaining page-tree-flattening scope of pdftract-bcf935ec beyond drifted
entries is untouched.

## Harness pins flipped (bead deliverable)

`crates/pdftract-core/tests/realworld_extraction_baseline.rs`:

- `desired_repo_root_valid_minimal_extracts_text`: `#[ignore]` removed;
  asserts 1 page + "Test".
- `pin_repo_root_valid_minimal_control_current_behavior`: asserts-empty ->
  asserts-contains("Test"), comments updated.
- The two core-suite pins above (side effect, documented in-file).

## Tests added (parser/xref.rs unit tests, MemorySource-backed)

- `recovers_object_recorded_past_its_header` (entry +8 past header — the
  fixture shape; asserts stream.offset lands on the true stream data)
- `recovers_object_recorded_short_of_its_header` (entry 7 short)
- `recovers_object_when_entry_points_at_another_object` (strict parse
  succeeds with a mismatched id; rescan still finds the right object)
- `correct_offset_still_resolves_strictly` (control: strict path unchanged)
- `destroyed_object_header_still_not_found` (genuinely missing object stays
  NotFound; no fabricated recovery)

## Commands and results (fix tree, clean extraction)

    cargo test -p pdftract-core --test realworld_extraction_baseline
      -> 10 passed; 0 failed; 4 ignored (remaining ignores belong to
         pdftract-4196ae99 x2 and pdftract-5b4c3d0e x2)  exit=0
    cargo test -p pdftract-core --lib parser::xref::tests::
      -> 88 passed; 9 failed — the 9 are the pre-existing
         test_forward_scan_* / test_parse_multi_subsection_xref set,
         byte-identical at pristine HEAD 40b48b17 (91 passed; 9 failed
         there — the +3 passing here are the new tests). All five new
         tests (parser::xref::tests::recovers_object_recorded_past_its_header,
         recovers_object_recorded_short_of_its_header,
         recovers_object_when_entry_points_at_another_object,
         correct_offset_still_resolves_strictly,
         destroyed_object_header_still_not_found) pass.
    cargo test -p pdftract-core --no-fail-fast  (full core sweep)
      -> fix tree 3551 passed / 107 failed vs pristine HEAD 40b48b17
         3541 passed / 112 failed. Normalized FAILED-name diff: zero new
         regressions; six pre-existing failures incidentally FIXED (the
         drifted-offset fixtures they build on now extract cleanly):
         extract::tests::test_result_to_json_format,
         extract::tests::test_result_to_json_includes_signatures,
         extract::tests::test_signatures_always_not_checked,
         extract::tests::test_untagged_pdf_no_deferred_diagnostic,
         output::ndjson::pipeline::tests::test_footer_errors_carry_structured_diagnostics,
         test_shape_match_fixture.
    cargo check --all-targets --quiet   (whole workspace, gate replica)
      -> exit=0

## Acceptance criteria

1. PASS — repo-root valid-minimal.pdf extracts "Test" at the pinned HEAD;
   strict-xref behavior on well-formed fixtures unchanged (full-core-sweep
   FAILED-set diff vs pristine HEAD is empty; strict-path control test
   added).
2. PASS — harness pins flipped;
   `cargo test -p pdftract-core --test realworld_extraction_baseline`
   green apart from the 4 remaining class pins owned by other beads.
3. PASS — this note, with commands and HEADs.
