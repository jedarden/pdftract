# pdftract-72b4cbdf

Landed the in-flight, uncommitted fixes for 4 of 6 classify_page outcome
tests in `crates/pdftract-core/src/classify.rs` (already present as a
working-tree diff from a prior worker attempt):

- test_page_classifier_vector_pure_text
- test_page_classifier_scanned_image_only
- test_page_classifier_broken_vector
- test_page_classifier_hybrid_with_grid

Each was already rewritten to `.expect("classify_page should succeed for
<reason>")` at the `classify_page(&ctx)` binding site, since
`classify_page` now returns `ClassificationResult<PageClassification>`
instead of a bare `PageClassification`. Assertions after the binding were
left untouched. Verified `git diff` contained exactly these 4 hunks before
committing.

Committed only `crates/pdftract-core/src/classify.rs` (verified no other
files were staged) and pushed to `origin main` (no `forgejo` remote exists
in this checkout; `origin` points to the same git.ardenone.com host).

Verification: `cargo test -p pdftract-core --lib --no-run` reports zero
compile errors at line <= 2175 in classify.rs (the 4 fns' range).
Remaining errors at lines 2176+ (blank_page/image_only_figure tests) and
in font/type3_rasterizer.rs, render/scanline.rs, content_stream.rs are
out of scope for this bead (child 2 and other beads).
