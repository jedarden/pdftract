# pdftract-76a0885e — Audit: every parse_bdc / parse_bmc caller and the resolver each passes

- **Audited tree:** HEAD `50037a66d50bf39753fb35575a281bcc22a87a79` (committed state, read via
  `git show HEAD:<path>` / `git grep HEAD` — the shared working tree carries other workers'
  in-flight edits, but `git grep` confirmed zero drift between HEAD and the working tree on all
  5 files listed below).
- **Method:** exhaustive `git grep -e parse_bdc -e parse_bmc` over the whole HEAD tree
  (not just `crates/`); every hit outside the 5 code files is bead-checkpoint text, test-run
  logs, or test inventories. Each call site's argument list was extracted mechanically
  (balanced-paren scan) so the "resolver passed" column is the literal argument at HEAD, not a
  recollection. Split-time reference state was f50f3d21; line numbers below are at
  50037a66 and were re-verified fresh.
- **Child 1/4 of the auto-split of pdftract-4f5ade3a.** Read-only audit; no code change made or
  expected. Read with child 2 (helper-level unit tests), child 3 (gap-threading), child 4
  (end-to-end fixture verification).

## The functions

    parse_bmc(stack: &mut MarkedContentStack, tag: Arc<str>) -> bool
        crates/pdftract-core/src/parser/marked_content_operators.rs:31

    parse_bdc(stack, tag, props, resources, default_off_ocgs, diagnostics,
              resolver: Option<&XrefResolver>) -> bool
        crates/pdftract-core/src/parser/marked_content_operators.rs:60

`parse_bmc` has **no resolver parameter by design** — a BMC tag carries no properties dict, so
"which resolver does it pass" is not a question for any parse_bmc site. The defect surface is
`parse_bdc`'s 7th parameter, consumed by:

- `extract_mcid_from_props` (marked_content_operators.rs:122) — `PdfObject::Name` whose
  `/Properties` entry is an indirect `PdfObject::Ref` is resolved only when `resolver` is
  `Some`; with `None` it emits `STRUCT_INVALID_BDC_OPERAND` ("is indirect reference but no
  resolver available") and returns `None` (:178-190). Inline dicts and direct-dict
  `/Properties` entries never need the resolver.
- `extract_ocg_ref_from_props` (marked_content_operators.rs:288) — same Ref branch returns
  **silently** `None` when `resolver` is `None` (:321-324); no diagnostic on this path
  (asymmetry vs. the MCID path — noted for child 3, not a defect to fix here).

## Production caller table (src/, at HEAD)

| # | Site | Enclosing fn | Calls | Resolver passed | Origin of the Option |
|---|------|--------------|-------|-----------------|----------------------|
| P1 | content_stream.rs:589 | `process_with_mode` (:487) | parse_bdc | `resolver` — its own param (:493) | threaded from caller (see U1/U2) |
| P2 | content_stream.rs:1179 | `execute_with_do` (:1041) | parse_bdc | `resolver` — its own param (:1048) | **no in-tree production caller** |
| P3 | content_stream.rs:544 | `process_with_mode` | parse_bmc | n/a (no resolver param) | — |
| P4 | content_stream.rs:1115 | `execute_with_do` | parse_bmc | n/a (no resolver param) | — |

(`parser/mod.rs:37` re-exports the three fns; not a call site. `content_stream.rs:478/481/485`
are `no_run` doc-comment examples passing `None` — listed under "legitimately-None".)

## Upstream trace of P1 (`process_with_mode`) — every in-tree caller supplies Some

    U1  extract.rs:202   process_content_stream_to_glyphs()  -> Some(resolver)   (:208)
        resolver is a REQUIRED &XrefResolver param (:189), not an Option.
        Only caller: extract.rs:2498, guarded by
            if let (Some(content_bytes), Some(res)) = (decoded_streams.as_ref(), resolver)  (:2497)
        -> when the document resolver is None, glyph extraction is skipped
           entirely (else branch :2499-2500 returns Vec::new()); parse_bdc is
           never reached with None from this path.

    U2  cli/grep/worker.rs:429   extract_spans_from_page()   -> Some(resolver)   (:435)
        resolver is a REQUIRED &XrefResolver param (:414).
        Calls resources.warm_indirect_properties(resolver, source) first (:423)
        because XrefResolver::resolve on this path is cache-only (bead
        pdftract-4f5ade3a). U1's path is warmed too (extract.rs:2481).

    Upstream of U1: extract_page_from_dict (:2435, resolver param :2441), fed
    Some(&resolver_arc) by all three public entry points:
        extract_pdf            extract.rs:881
        extract_pdf_ndjson     extract.rs:1928
        extract_pdf_streaming  extract.rs:2252

## Upstream trace of P2 (`execute_with_do`)

No in-tree production caller exists at HEAD — every caller is a test
(39 in-file sites in content_stream.rs:3459-4455, plus tests/content_stream_font_diagnostic.rs:20,
all passing `None`). `execute_with_do` is public API (`pub mod content_stream`, lib.rs:170;
`pub fn execute_with_do`), so out-of-tree SDK consumers (pdftract-py/ruby/dotnet — none checked
out on this box, not audited) could introduce a None-passing call site. Flag for child 4.

## Test caller table (every site at HEAD)

### crates/pdftract-core/src/parser/marked_content_operators.rs (#[cfg(test)], 25 sites)

| Line | Test | Calls | Resolver arg |
|------|------|-------|--------------|
| 386 | test_parse_bmc | parse_bmc | n/a |
| 398 | test_parse_bdc_with_inline_dict_mcid | parse_bdc | None |
| 416 | test_parse_bdc_with_inline_dict_no_mcid | parse_bdc | None |
| 435 | test_parse_bdc_with_property_name_found | parse_bdc | None |
| 454 | test_parse_bdc_with_property_name_not_found | parse_bdc | None |
| 473 | test_parse_emc_success | parse_bmc | n/a |
| 560 | test_nested_marked_content | parse_bdc | None |
| 571 | test_nested_marked_content | parse_bmc | n/a |
| 589 | test_bmc_tag_leading_slash | parse_bmc | n/a |
| 602 | test_bdc_tag_leading_slash | parse_bdc | None |
| 623 | test_stack_depth_limit | parse_bmc | n/a |
| 627 | test_stack_depth_limit | parse_bmc | n/a |
| 637 | test_parse_bdc_with_real_mcid_large | parse_bdc | None |
| 656 | test_parse_bdc_oc_tag_not_ocg | parse_bdc | None |
| 680 | test_parse_bdc_oc_tag_with_ocg_not_in_off_set | parse_bdc | None |
| 704 | test_parse_bdc_oc_tag_with_ocg_in_off_set | parse_bdc | None |
| 729 | test_parse_bdc_slash_oc_tag | parse_bdc | None |
| 828 | test_bdc_integration_ocg_visibility_from_parsed_catalog | parse_bdc | None |
| 841 | test_bdc_integration_ocg_visibility_from_parsed_catalog | parse_bmc | n/a |
| 850 | test_bdc_integration_ocg_visibility_from_parsed_catalog | parse_bdc | None |
| 871 | test_bdc_oc_tag_without_ocg_property_never_hidden_even_with_off_set | parse_bdc | None |
| 898 | test_parse_bdc_non_oc_tag_ignores_ocg_property | parse_bdc | None |
| 1213 | test_parse_bdc_property_name_resolves_mcid_via_resolver | parse_bdc | **Some(&resolver)** |
| 1232 | test_parse_bdc_property_name_no_resolver_emits_diagnostic | parse_bdc | None (deliberate: pins the STRUCT_INVALID_BDC_OPERAND no-resolver diagnostic) |
| 1293 | test_parse_bdc_property_name_direct_dict_recovers_mcid | parse_bdc | None (deliberate: /Properties entry is a direct dict — resolver not needed) |

The 15 None-passing sites at :398-:898 all feed inline dicts or direct-dict `/Properties`
entries, where `extract_mcid_from_props` never consults the resolver.

### crates/pdftract-core/src/content_stream.rs (#[cfg(test)], 69 sites)

- **30 `process_with_mode` sites** (:2579, :2594, :2619, :2638, :2647, :2670, :2681, :2704,
  :2725, :2746, :2769, :2804, :2812, :2824, :2838, :2895, :2915, :2936, :2958, :2980, :3003,
  :4480, :4506, :4532, :4557, :4582, :4602, :4622 — 28 sites) pass **None**. None of them
  exercise indirect /Properties; they cover text extraction, positioning, marked-content
  inline-MCID, and diagnostics.
- :3037 `test_process_with_mode_bdc_indirect_properties_via_resolver` passes
  **Some(&resolver)** — the positive indirect-resolution path.
- :3064 `test_process_with_mode_bdc_indirect_properties_no_resolver_drops_mcid` passes
  **None** (deliberate — pins the metadata-drops-without-resolver behavior the parent bead
  pdftract-4f5ade3a exists to fix). If child 3 changes any default, this test's expectation is
  the one that moves.
- **39 `execute_with_do` sites** (:3459, :3491, :3522, :3542, :3563, :3590, :3615, :3634,
  :3653, :3672, :3692, :3713, :3742, :3762, :3783, :3874, :3923, :3946, :3969, :3991, :4013,
  :4037, :4066, :4090, :4118, :4141, :4162, :4190, :4210, :4231, :4252, :4277, :4302, :4327,
  :4347, :4374, :4401, :4428, :4455) all pass **None** — text-state / Tf / TJ / BT-ET
  diagnostics; none exercise BDC indirect properties, and none build a document.

### Integration tests

| File:line | Test/helper | Calls | Resolver arg |
|-----------|-------------|-------|--------------|
| tests/marked_content_properties_fixtures.rs:176 | `run_bdc_mc0` helper (used by the indirect + inline fixture tests) | parse_bdc | **Some(resolver)** |
| tests/content_stream_font_diagnostic.rs:20 | `run` helper (2 Tf diagnostic tests) | execute_with_do | None (no document, by design — font-diagnostics only) |

## Verdicts

- **must-switch: NONE.** At HEAD `50037a66` no production call path reaches `parse_bdc` with
  `resolver = None` while a resolver is available. Both production `parse_bdc` sites forward a
  resolver parameter, and every in-tree production upstream supplies `Some` — the three public
  extract entry points hard-code `Some(&resolver_arc)`, `extract_page_from_dict`'s None case
  short-circuits *before* content processing (:2497), and the grep worker takes a required
  `&XrefResolver`. The split-time worry ("BOTH already passed Some") still holds at HEAD.
- **already-Some: P1** (content_stream.rs:589 via process_with_mode) and, vacuously, **P2**
  (:1179, whose only callers are tests).
- **legitimately-None (no action):** all parse_bmc sites (no resolver param exists); all 69
  in-file content_stream.rs test sites except :3037 (Some) — including the deliberate negative
  pins at :3064 and marked_content_operators.rs:1232; the 15 inline/direct-dict
  marked_content_operators.rs unit tests; tests/content_stream_font_diagnostic.rs:20; the
  `no_run` doc examples at content_stream.rs:478/481/485.
- **Acceptance question answered explicitly: no PRODUCTION path at HEAD can still reach
  parse_bdc with None while a resolver is available.** There is nothing for child 3 to
  re-thread inside this repo's production code. Child 3's residual scope, should it want one,
  is limited to the caveats below; child 4 should verify the end-to-end fixtures still
  demonstrate the Some path (marked_content_properties_fixtures.rs already does).

## Caveats for child 3 / child 4

1. **`execute_with_do` is pub with no in-tree production caller.** If an SDK repo binds it
   directly, that call site would pass whatever the SDK has (likely None — none of the SDK
   repos are checked out on this box, so this is unverified). Out of this repo's scope; child
   4's end-to-end pass should confirm through the actual public entry points (extract_*),
   which are all Some.
2. **Diagnostic asymmetry:** the no-resolver branch of `extract_mcid_from_props` emits
   `STRUCT_INVALID_BDC_OPERAND`, but `extract_ocg_ref_from_props`' no-resolver branch
   (:321-324) is silent. Only reachable from tests today; noted in case child 2 wants to pin it.
3. **Warm precondition:** `XrefResolver::resolve` on the extraction path is cache-only, so
   callers must run `ResourceDict::warm_indirect_properties(resolver, source)` before
   processing (grep/worker.rs:423, extract.rs:2481). Any future new caller must preserve that
   ordering — this is the trap that would reintroduce the silent-loss defect in the presence
   of `Some(resolver)`.
4. **Working-tree drift:** none on the audited files today, but the shared checkout carries
   other workers' in-flight edits; child 3 must re-run this audit's grep at its own HEAD
   before editing.

## Site-count summary

103 call sites audited at HEAD: 2 production parse_bdc, 2 production parse_bmc, 18 in-file
test parse_bdc, 7 in-file test parse_bmc, 30 in-file test process_with_mode, 39 in-file test
execute_with_do, 1 integration parse_bdc (Some), 1 integration execute_with_do (None),
3 doc-comment examples. Plus 1 re-export line (parser/mod.rs:37).
