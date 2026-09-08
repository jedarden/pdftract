# pdftract-6fb49cdb

Unwrapped the `classify_page` Result at the binding site in the last 2 of
pdftract-daccf03d's 6 outcome tests in `crates/pdftract-core/src/classify.rs`
(`#[cfg(test)] mod tests`):

- `test_page_classifier_blank_page` (decl 2169)
- `test_page_classifier_image_only_figure` (decl 2182)

Each bare `let result = classify_page(&ctx);` became
`classify_page(&ctx).expect("classify_page should succeed for <reason>")`,
matching the pattern the sibling bead pdftract-72b4cbdf already established
in the 4 fns above these. `classify_page` now returns
`ClassificationResult<PageClassification>`, so the previous bindings held a
`Result` and every following field access failed with E0609. All assertions
in both fns are byte-identical — only the binding line(s) changed. No
production code, no derives, no other test fn touched.

Ownership honored: the sibling pdftract-71de30c6 range starts at
`test_page_classifier_short_circuit_no_text` (decl 2202 post-edit), so
everything from there on was left alone. Committed only
`crates/pdftract-core/src/classify.rs` + this note; `git diff --stat`
confirmed no other files were staged.

## Verification

`timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run
--message-format=short`, grepped for classify.rs `error[` locations:

- errors at line <= 2196 (this bead's range): **0** — PASS
- lowest remaining classify.rs error: line 2211, inside
  `test_page_classifier_short_circuit_no_text` → pdftract-71de30c6
- remaining classify.rs errors at 2211-2787 (short_circuit fns, smoke +
  microbenchmark fns at decls ~2614/~2735) plus type3_rasterizer.rs,
  render/scanline.rs, content_stream.rs belong to other beads — WARN as
  expected; the target still does not compile overall until those land.

`cargo fmt -p pdftract-core -- --check` flags ~69 files tree-wide
(including 21 pre-existing hunks in classify.rs production code at lines
100-800, all committed before this change). No hunk falls in this bead's
edited range (2169-2199) and the repo has no rustfmt.toml, so no fmt
regression was introduced and none was repaired here (out of scope).

Note: this checkout has no `forgejo` remote; `origin` points at the same
git.ardenone.com host, so the push went to `origin main`.
