# pdftract-3ad3f1dd — audit: classify.rs chain commits are cfg(test)-only with untouched assertions

Child 2 of 5 of the split of pdftract-cd2c8e22. **Read-only audit — no code
was changed by this bead.** Range audited: `942a795e..HEAD`, file
`crates/pdftract-core/src/classify.rs` (clean vs HEAD at audit time; HEAD =
`3970f0ff`).

## 1. Chain commit / hunk enumeration

`git log 942a795e..HEAD -- crates/pdftract-core/src/classify.rs` returns
exactly two commits; the range diff (12 insertions / 6 deletions) is the exact
sum of their per-commit stats, so nothing else touched the file in the range.

| Commit | Bead | Hunks (new-file start lines) | Stat | Verdict |
|---|---|---|---|---|
| `f40117c3` | pdftract-72b4cbdf | 2063, 2083, 2110, 2156 | 8+/4− | PASS — all 4 inside `mod tests`, binding-site only |
| `ff006426` | pdftract-6fb49cdb | 2170, 2189 | 4+/2− | PASS — both inside `mod tests`, binding-site only |
| **Total** | | **6 hunks** | **12+/6−** | **PASS** |

Hunk → test mapping (test decl lines at HEAD):

| New-file hunk line | Enclosing test (decl line) | Changed content |
|---|---|---|
| 2063 | `test_page_classifier_vector_pure_text` (2055) | `let result = classify_page(&ctx);` → `.expect("…pure vector text page")` |
| 2083 | `test_page_classifier_scanned_image_only` (2076) | same shape, `.expect("…image-only scanned page")` |
| 2110 | `test_page_classifier_broken_vector` (2096) | same shape, `.expect("…invisible-text-over-image page")` |
| 2156 | `test_page_classifier_hybrid_with_grid` (2123) | same shape, `.expect("…valid grid cells")` |
| 2170 | `test_page_classifier_blank_page` (2169) | same shape, `.expect("…a blank page")` |
| 2189 | `test_page_classifier_image_only_figure` (2183) | same shape, `.expect("…an image-only page")` |

## 2. cfg(test)-only scope — PASS

- `#[cfg(test)]` at `classify.rs:1607`, `mod tests {` at 1608; file is 3660
  lines, so the test module spans 1608–3660 (EOF).
- Every hunk's new-file start line (2063–2189) is > 1608 → **all 6 hunks are
  inside `#[cfg(test)] mod tests`.**
- The complete set of changed content lines in the range diff is the 18 lines
  shown in §1 (6 removed bare bindings + 12 added expect-wrapped bindings).
  **Zero hunks touch any production region.**
- The neighbouring `test_page_classifier_short_circuit_no_text` (decl 2202,
  outside the 6-fn range) is untouched, as expected.

## 3. No new derives — PASS

- `diff <(git show 942a795e:classify.rs | grep derive) <(git show
  HEAD:classify.rs | grep derive)` → **empty** (byte-identical derive sets).
  All 10 `#[derive(…)]` attributes sit at lines < 1607 (production region) and
  none changed.
- `ClassificationError` (decl `classify.rs:37`) carries
  `#[derive(Debug, Clone, PartialEq)]` at **both** revisions — it did **not**
  gain `Serialize`/`Deserialize`. No derive was added to any production type.
- The 2× E0277 sites are still present and untouched:
  - `classify.rs:2694` — `let json_output = serde_json::to_string(&result);`
  - `classify.rs:2778` — `let json_result = serde_json::to_string(&result);`
  - `git diff 942a795e..HEAD` contains **0** changed lines referencing either
    line.
- Compile confirmation: sibling pdftract-cc1e9d96's fresh gated census
  (`notes/pdftract-cc1e9d96.md`, run at `35642ac3`) reports 54 classify.rs
  error locations, **52× E0609 + 2× E0277 at 2694 and 2778**, 0 at ≤ 2196.
  `git diff 35642ac3..HEAD -- classify.rs` is empty, so that census is valid
  for the audited HEAD. The E0277s remain live for sibling
  pdftract-82ce7969 (they clear on unwrap; adding a derive here would have
  been a scope violation, and none was added).

## 4. classify_page signature — PASS (byte-identical)

Line 747 at both `942a795e` and HEAD, byte-for-byte identical:

```rust
pub fn classify_page(ctx: &PageContext) -> ClassificationResult<PageClassification> {
```

The full function body at both revisions is likewise identical
(`validate_page_context(ctx)?` → `PageClassifier::new()` →
`classify(ctx)` → `validate_classification_result(&result)?` → `Ok(result)`),
consistent with zero production hunks.

## 5. Test assertions untouched — PASS

Two independent checks:

1. **Changed-line enumeration:** every one of the 18 changed content lines in
   the range diff is a `let result = …` binding line (see §1); the count of
   added/removed lines containing `assert` is **0**. All assertion lines
   appear only as diff context.
2. **Per-test body comparison:** for each of the 6 tests, the full function
   body was extracted at `942a795e` and at HEAD and diffed. In all 6 cases the
   only differing hunk is the binding site (the bare `let result =
   classify_page(&ctx);` vs the expect-wrapped binding); every other line of
   every body — setup, context mutation, and all `assert_eq!`/`assert!`
   statements — is identical. Sample diff for
   `test_page_classifier_vector_pure_text` (representative; the other 5 are
   the same shape):

```diff
12c12,13
<         let result = classify_page(&ctx);
---
>         let result =
>             classify_page(&ctx).expect("classify_page should succeed for pure vector text page");
```

## Acceptance criteria

- **PASS** — audit table above lists each chain commit, its hunk line numbers,
  and a per-commit verdict, with totals (6 hunks, 12+/6−).
- **PASS** — zero production hunks (§2); no new derives, ClassificationError
  unchanged, both E0277 sites still present (§3); classify_page signature
  byte-identical (§4); all 6 tests' assertions unchanged (§5).
- **FAIL conditions** — none triggered. No hunk outside the cfg(test) module,
  no derive added to a production type, no altered assertion.

## Disposition

No code changes. Verdict: **the chain f40117c3 + ff006426 is cleanly scoped to
`#[cfg(test)] mod tests` in classify.rs, touching only the Result handling at
the 6 binding sites.** No violation to escalate to the umbrella owner
(pdftract-cd2c8e22).
