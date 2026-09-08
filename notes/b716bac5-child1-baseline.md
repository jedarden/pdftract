# b716bac5 child 1 — pre-gate git baseline

**Bead:** pdftract-3e8309f4 (grandchild 1 of 4, parent pdftract-fa233a5e, umbrella pdftract-b716bac5)
**Purpose:** Point-in-time record of the git state and in-flight edit inventory immediately
before the pdftract-core compile gate, so the gate verdict can be read against the exact
tree it was produced on. Documentation only — no cargo invocation, no source changes.

## Snapshot

| Field | Value |
|---|---|
| Captured (UTC) | 2026-09-08T11:48:28Z |
| HEAD | `3dcfb6bee55b1ff11cb3e73dbb06e97b76494e13` |
| origin/main | `3dcfb6bee55b1ff11cb3e73dbb06e97b76494e13` (remote = Forgejo `https://git.ardenone.com/jedarden/pdftract.git`; re-fetched at capture time) |
| Ahead / behind | 0 / 0 — HEAD and origin/main are identical |
| Branch | `main` |

## Working-tree state under `crates/`

Point-in-time snapshot at the timestamp above; sibling workers may still be editing.
Names only, no diffs, no contents. 44 modified + 5 untracked = 49 entries.

### Modified (44)

```
 M crates/pdftract-cli/src/grep/worker.rs
 M crates/pdftract-cli/src/main.rs
 M crates/pdftract-cli/src/mcp/tools/registry.rs
 M crates/pdftract-cli/src/panic_hook.rs
 M crates/pdftract-cli/tests/TH-05-ssrf-block.rs
 M crates/pdftract-core/Cargo.toml
 M crates/pdftract-core/src/attachment/filespec.rs
 M crates/pdftract-core/src/attachment/name_tree.rs
 M crates/pdftract-core/src/audit.rs
 M crates/pdftract-core/src/cache/key.rs
 M crates/pdftract-core/src/cache/lru.rs
 M crates/pdftract-core/src/content_stream.rs
 M crates/pdftract-core/src/decoder/jbig2.rs
 M crates/pdftract-core/src/detection.rs
 M crates/pdftract-core/src/document.rs
 M crates/pdftract-core/src/encryption/detection.rs
 M crates/pdftract-core/src/extract.rs
 M crates/pdftract-core/src/font/type3.rs
 M crates/pdftract-core/src/font/type3_rasterizer.rs
 M crates/pdftract-core/src/font/type3_rasterizer_test.rs
 M crates/pdftract-core/src/forms/mod.rs
 M crates/pdftract-core/src/javascript.rs
 M crates/pdftract-core/src/layout/correction.rs
 M crates/pdftract-core/src/layout/figure.rs
 M crates/pdftract-core/src/output/markdown/links.rs
 M crates/pdftract-core/src/output/ndjson/frames.rs
 M crates/pdftract-core/src/output/ndjson/pipeline.rs
 M crates/pdftract-core/src/page_class.rs
 M crates/pdftract-core/src/parser/catalog.rs
 M crates/pdftract-core/src/parser/marked_content_operators.rs
 M crates/pdftract-core/src/parser/pages.rs
 M crates/pdftract-core/src/parser/resources.rs
 M crates/pdftract-core/src/parser/stream.rs
 M crates/pdftract-core/src/signature/mod.rs
 M crates/pdftract-core/src/source/mmap.rs
 M crates/pdftract-core/src/span/mod.rs
 M crates/pdftract-core/src/table/output.rs
 M crates/pdftract-core/tests/type3_charproc_structure.rs
 M crates/pdftract-py/pyproject.toml
 M crates/pdftract-py/src/extract.rs
 M crates/pdftract-py/src/extract_markdown.rs
 M crates/pdftract-py/src/extract_stream.rs
 M crates/pdftract-py/src/extract_text.rs
 M crates/pdftract-py/src/lib.rs
```

By area: 31 under `pdftract-core/src`, 5 under `pdftract-py/src`, 4 under
`pdftract-cli/src`, plus `pdftract-core/Cargo.toml`, `pdftract-core/tests/type3_charproc_structure.rs`,
`pdftract-cli/tests/TH-05-ssrf-block.rs`, and `pdftract-py/pyproject.toml`.

### Untracked (5)

```
?? crates/pdftract-core/tests/type3_glyph_shape.rs
?? crates/pdftract-core/tests/zz_probe_core_extract.rs
?? crates/pdftract-core/tests/zz_tmp_diag.rs
?? crates/pdftract-py/conformance-report.json
?? crates/pdftract-py/tests/zz_probe_search.rs
```

The three `zz_*`/probe files under `pdftract-core/tests` and `pdftract-py/tests` are
diagnostic scratch tests from in-flight sibling work — relevant to the gate because
`cargo test`/compile checks that glob `tests/` will pick them up.

### Outside `crates/` (for context only, not part of this bead's criteria)

The tree also carries in-flight edits to `.beads/checkpoint/*`, `.ci/scripts/lint-ruby-visibility.sh`,
`.needle-predispatch-sha`, and other non-crate paths, plus two untracked `notes/pdftract-*.md`
files from sibling workers. None of these were touched by this bead.

## Constraint honored

No file under `crates/` was created, edited, staged, or reverted by this bead — the
`git status --porcelain -- crates/` output above is the pre-existing sibling in-flight
state, recorded verbatim. Only this note file was added.
