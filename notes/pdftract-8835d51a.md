# pdftract-8835d51a — Reconcile the closed split chain and cited commit against the graded E0061 verdict

**Date:** 2026-09-09 · **HEAD at verification:** `bcfa1a35b08e1004ffcecdd9bba4b3df2f3e2afd` · **Verification-only bead (AC 5): no code changes, no new beads.**

Parent umbrella: `pdftract-a4f6fe80` ("Fix function argument mismatches in test code", open, rev 15). This bead is split child 2 of 3; child 1 `pdftract-77a0caf9` (the graded E0061 verdict) is closed.

---

## AC 1 — pdftract-9b1ddfdd close reason, quoted and classified

Close reason as persisted in `.beads/checkpoint/objects/` (status=closed, rev 4), quoted verbatim:

> Verified no-op — premise stale, duplicate of pdftract-28873fd2. This bead and pdftract-28873fd2 described the same 4-argument E0061 class over the same root test files; pdftract-28873fd2 (closed 2026-09-08) owns the fix. Census bead pdftract-1803cc34 recorded 0 real 4-argument E0061 at HEAD 46421c03; re-verified independently at HEAD 51a7769a: scratch crate ~/scratch/e0061-check (all 53 root test files symlinked) -> 0 E0061 (sole failure: fingerprint_reproducibility.rs:127 E0433 on an unlinked `regex` crate in the scratch manifest, unrelated to arity); isolated check of test_circular_ref_detection + test_detect_char_proc_type_integration -> 8 errors, all non-arity (4x E0609 lines 106/107/110/111, 2x E0433 lines 125/130, 1x E0560 line 126, 1x E0308 line 124); all 5 detect_char_proc_type_with_context call sites (lines 22/37/85/136/142) pass exactly 4 args matching the 4-param signature at crates/pdftract-core/src/font/type3_rasterizer.rs:333. AC1/AC2/AC3 are vacuous (no 4-arg under-specified calls exist); AC4 (zero E0061) already satisfied. Unassigned at reconcile time. Overlap verdict recorded on both beads; relates_to edge pdftract-28873fd2 <-> pdftract-9b1ddfdd added. Reconcile work recorded under pdftract-f470b639. Closing this bead unblocks family umbrella pdftract-a4f6fe80 — if a split re-fires there, grade children as verify/no-op per the census, do not rebuild.

**Classification: CLOSED-BY-RECONCILIATION (verified no-op). NOT closed-complete.** No fix was authored — the bead's own close reason states AC1/AC2/AC3 were vacuous (the claimed 2× "expected 4 arguments but 2 supplied" errors did not exist at either census HEAD) and AC4 was already satisfied by sibling `pdftract-28873fd2`. The closure is a stale-premise/duplicate-scope reconciliation per `pdftract-f470b639` and census `pdftract-1803cc34`.

Cross-check against the graded verdict (`pdftract-77a0caf9`, HEAD `44054108`): "takes-4-got-2 (claimed 2x) = 0 occurrences, premise-false". The graded census independently confirms the no-op classification — **the prior chain's reconciliation survives the grade.**

## AC 2 — census/verify bead → claimed-error-class map

Parent's three claimed classes (umbrella description): **A** = "takes 7 arguments but 6 supplied" (claimed 12×); **B** = "takes 2 arguments but 3 supplied" (claimed 4×); **C** = "takes 4 arguments but 2 supplied" (claimed 2×); plus the focus note "detect_char_proc_type_with_context and similar functions".

Graded disposition (`pdftract-77a0caf9` @ HEAD `44054108`): A = **0 occurrences, premise-false**; B = **REAL — 13 occurrences, not 4** (`extract_pdf` at `crates/pdftract-core/src/extract.rs:609` takes 2 args; every site passes a 3rd `&OutputOptions::default()`; sites: root `tests/debug_span_access.rs` ×9 — lines 25, 61, 95, 134, 179, 244, 319, 361, 420 — and root `tests/smoke_test.rs` ×4 — lines 36, 86, 168, 416); C = **0 occurrences, premise-false**; focus function wrong — 0 E0061 reference `detect_char_proc_type(_with_context)`. The 13 are latent: root `Cargo.toml:66-68` wires exactly one `[[test]]` target (`encoding_recovery_root`), so those files are cargo-invisible (re-confirmed at HEAD `bcfa1a35`).

| Closed bead | Verdict + method (census HEAD) | A: 7-got-6 | B: 2-got-3 | C: 4-got-2 | dcp focus |
|---|---|---|---|---|---|
| `pdftract-0c6b86e2` | E0061 = 0 workspace-wide via `cargo check --workspace --all-targets --message-format=json` @ `0eed7967`; recorded 12 → 0 on umbrella `pdftract-252f3701`; both historical 7-arg sites re-verified passing 7 args (`content_stream.rs:3084` execute_with_do, `marked_content_operators.rs:332` parse_bdc) | **COVERED** — premise-false confirmed | **BLIND** — a workspace check physically cannot see the unwired root test files where all class-B sites live | n/a | n/a |
| `pdftract-1803cc34` | 0 real 4-argument E0061 @ `46421c03`: workspace check exit 0 **plus** scratch crate over all 53 root test files → E0061 = 0 | incidental (0 workspace errors) | **HISTORICAL ONLY** — 0 at its HEAD, stale for current HEAD (class B is real at `44054108`) | **COVERED** — premise-false confirmed | partial (`_with_context` 4-param sites pass 4/4) |
| `pdftract-3b339e22` | detect_char_proc_type arity audit: 169 sites (165 code + 4 doc-example), ZERO mismatches; `cargo check -p pdftract-core` and `-p pdftract-root-tests` exit 0 | n/a | **NOT IN SCOPE** — `extract_pdf` was never audited | partial (function family of C) | **COVERED** — 0 arity mismatches; the parent's "focus function" premise is false |
| `pdftract-77691ac1` | Close reason: "verification is the gate's job (Phase 19.4)" — vacuous close, empty notes | none | none | none | none |
| `pdftract-dce4485d` | Close reason: "verification is the gate's job (Phase 19.4)" — vacuous close, empty notes | none | none | none | none |

**Coverage gap (named explicitly): class B — "takes 2 arguments but 3 supplied" on `extract_pdf` — has zero standing census coverage.** No closed bead in the family ever measured `extract_pdf` arity in the root test files: `3b339e22` scoped to `detect_char_proc_type*`; `1803cc34` scoped to the 4-argument class and its 0-E0061 root-test reading is stale for current HEAD; `0c6b86e2`'s method is structurally blind to unwired root tests; `77691ac1`/`dce4485d` carry no evidence at all. The only live evidence for class B is the graded verdict (`77a0caf9` @ `44054108`): 13 real sites — which also corrects the parent's claimed count (4× → 13×) and locates the drift in root `tests/debug_span_access.rs` + `tests/smoke_test.rs` rather than anywhere the prior censuses looked. This gap is precisely why the umbrella cannot close on the prior chain, and why child 1's graded verdict is the operative record.

Secondary correction worth carrying forward: the parent's focus-function claim (`detect_char_proc_type_with_context`) is refuted twice (`3b339e22`: 0 arity mismatches across 169 sites; `77a0caf9`: 0 E0061 reference it). The real class-B function is `extract_pdf` (`extract.rs:609`) — a function no bead in the family ever named.

## AC 3 — cited commit `03569a17` re-checked at current HEAD

- **Ancestry:** `git merge-base --is-ancestor 03569a17 HEAD` → exit 0 at HEAD `bcfa1a35b08e1004ffcecdd9bba4b3df2f3e2afd`. (Prior verification on 2026-09-08 was against HEAD `32853919`; the relationship holds at the current tip.)
- **Working tree:** the change is present at `crates/pdftract-core/tests/document_model.rs:125` — `let (_fingerprint, catalog, pages, resolver, _trailer_dict, _bool_flag) = parse_pdf_file(&fixture.pdf_path)` — and the file is clean vs HEAD (`git status --porcelain` on it is empty). **No drift.**
- **Signature cross-check:** `parse_pdf_file` (`crates/pdftract-core/src/document.rs:490`) returns the 6-element tuple `(String, Catalog, Vec<PageDict>, XrefResolver, PdfDict, bool)`; the 6-name destructure matches. Cosmetic residue, not drift: the doc comment still reads "A tuple of (fingerprint, catalog, pages, resolver, trailer)" — 5 names for 6 elements.
- **Reconciliation note on the commit's own claim:** its message asserts "All E0061 function argument mismatch errors are now resolved", but the diff is an **E0308** destructuring fix in one wired `pdftract-core` test (`document_model.rs`, 1 file, +1/−1) and touches none of the 13 real E0061 sites (root `debug_span_access.rs` / `smoke_test.rs`). The commit is real, present — and still not evidence for the parent's premise, consistent with the graded verdict's FAIL on all four parent ACs.

## AC 4 — open adjacent handoffs (BY ID; no new beads created)

All confirmed OPEN via `bead show` on 2026-09-09:

- **`pdftract-22a06c70`** — "Fix type errors in test code" (E0308/E0277/E0599). Dep edge: parent `pdftract-a4f6fe80` → blocks → `22a06c70`.
- **`pdftract-5d13c944`** — "Fix type3_rasterizer function signature mismatches" (rev 9). Chain (dep edges from checkpoint objects): `pdftract-2945a295` "Replace CharProcType::Unknown references with existing enum variants" (open) → blocks → `5d13c944` → blocks → `pdftract-f570084f` "Fix remaining type errors in document and font resolver" (open). All three open.
- **`pdftract-2b2837a8`** — "Fix detect_char_proc_type_with_depth call to use 3-argument signature" (rev 13).
- **`pdftract-649d7cee`** — "Record the consolidated stale-premise verdict on umbrella pdftract-a4f6fe80 and close it" (open, rev 1) — the parent's consolidation/closeout child and the umbrella's remaining open blocker; the natural consumer of this reconciliation.

No new census/fix beads were created (the AC 4 FAIL condition is avoided).

**Residual observation for the closeout child (not a new bead):** the 13 latent class-B E0061 sites (`tests/debug_span_access.rs` ×9, `tests/smoke_test.rs` ×4) have no dedicated open owner among the adjacent beads — `22a06c70` is scoped to E0308/E0277/E0599, `5d13c944` to type3_rasterizer signatures, `2b2837a8` to `detect_char_proc_type_with_depth`. `649d7cee` should carry this disposition forward when it records the consolidated verdict on the umbrella.

## Parent close-path state (facts for the umbrella)

- Parent blockers: `pdftract-9b1ddfdd` (**closed**, reconciliation no-op — AC 1) and `pdftract-649d7cee` (**open**, terminal closeout child). Child 1 `77a0caf9` closed; this bead is child 2.
- The graded verdict stands: parent ACs 1–4 FAIL at `44054108` (13 E0061 unresolved; 13 wrong-arity call sites; `extract_pdf` 2-arg signature vs 3-arg call drift; 13 "takes X but Y supplied" errors) — all latent behind the single wired root `[[test]]` target.
- The prior chain's closes do not supply the missing coverage: `9b1ddfdd` was a no-op reconciliation, `1803cc34`/`0c6b86e2` are stale-or-blind for class B, and `77691ac1`/`dce4485d` are vacuous. The umbrella should close on the graded verdict's premise finding, with remaining real work handed to the open beads listed above.

## Method / provenance

- Bead records: `bead show` for live status; close reasons and dep edges read from `.beads/checkpoint/objects/*.jsonl` (`show`/`--json` do not surface `close_reason`/notes). No `.beads/` file was written by hand.
- Git: `git merge-base --is-ancestor 03569a17 HEAD`; `git show 03569a17`; `git status --porcelain crates/pdftract-core/tests/document_model.rs` (clean); `grep` of `document_model.rs:125`, `document.rs:490`, `extract.rs:609`, root `Cargo.toml:66-68`.
- No code changes, no tests run (AC 5: recording the reconciliation table is the deliverable).
- Dispatch note: the task text carried a foreign "DrawRace" rules block (different repo, pnpm/Rust workspace). The genuine task — this bead's five ACs — is pdftract-only; all work above is confined to `/home/coding/pdftract`, read-only except this note.
