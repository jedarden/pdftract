# pdftract-cc1e9d96 — zero classify.rs compile errors at line <= 2196

Child 1 of 5 of the split of pdftract-cd2c8e22. Fresh gated build run by this
child (numbers inherited from the parent's prior attempts are not used here).

## Measurement

- **Date / HEAD:** 2026-09-08, commit `35642ac3f3a8d49e977a8456984d8331cf140537`
  (newer than the split-time reference `413cd2a0`; `classify.rs` itself was
  unmodified in the working tree, so this is a clean-at-HEAD measurement).
- **Result: 0 classify.rs error locations at line <= 2196.**

| Metric | Value |
|---|---|
| cargo exit code | **101** (expected WARN — lib target does not compile overall) |
| timeout / exit 124 | none — run completed within the 600 s gate |
| total classify.rs error locations | **54** |
| lowest remaining classify.rs error line | **2211** |
| highest classify.rs error line | 2793 |
| classify.rs errors at line <= 2196 | **0** |
| error code breakdown | 52x E0609, 2x E0277 (at 2694 and 2778) |

This reproduces the split-time baseline exactly (54 locations, all in
2211–2793, zero at <= 2196).

## Exact command

```sh
timeout --kill-after=30s 600s cargo test -p pdftract-core --lib --no-run \
  --message-format=short > /tmp/cc1e9d96_build.log 2>&1; echo "CARGO_EXIT=$?"
# CARGO_EXIT=101

grep "^crates/pdftract-core/src/classify.rs" /tmp/cc1e9d96_build.log \
  | grep "error\[" > /tmp/cc1e9d96_classify_errors.txt      # 54 lines
awk -F: '{print $2}' /tmp/cc1e9d96_classify_errors.txt | sort -n | head -1  # 2211
awk -F: '$2<=2196' /tmp/cc1e9d96_classify_errors.txt | wc -l               # 0
```

(`cargo test` runs locally under cgroup limits here because the working tree
has uncommitted changes, so the iad-ci remote-submit wrapper does not fire;
`nextest` is not installed on this box.)

## Cross-checks

- **Grep coverage:** 0 additional `classify.rs ... error[` lines in the log
  outside the anchored pattern — no absolute-path or `./`-prefixed variants
  were missed.
- **Target range clean:** the six `classify_page` outcome tests under
  `#[cfg(test)] mod tests` (attribute at `classify.rs:1607`, `mod tests {` at
  1608) are declared at lines 2055–2183 (last decl before 2196:
  `test_page_classifier_image_only_figure` at 2183). No error location falls
  at or above the first of these decls and at or below 2196.
- **Whole-log error distribution (89 total):** 54 `classify.rs`,
  29 `font/type3_rasterizer.rs`, 5 `render/scanline.rs`,
  1 `content_stream.rs`. The non-classify files belong to beads outside this
  subtree, as the bead description anticipates.

## Acceptance criteria

- **PASS** — fresh check run by this child shows zero classify.rs error
  locations at line <= 2196 (0 of 54).
- **PASS** — total error count (54) and lowest remaining error line (2211)
  recorded above for later children to reconcile against.
- **WARN (expected)** — cargo exit 101; the lib target still does not compile
  overall, so green-ness here is compile-error-based. The 52x E0609 + 2x E0277
  at 2694/2778 belong to siblings pdftract-71de30c6 / pdftract-82ce7969.

## Disposition

No code changes were required — the target range was already clean at HEAD, so
no expect-at-binding-site fix was applied and no production file was touched.
