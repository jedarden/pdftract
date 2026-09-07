# pdftract-d0c18a80 — Catalog::default_off_ocgs accessor

Child 1 of 4 of umbrella pdftract-bdb98b50.

## What was done

Added one pure accessor to the `impl Catalog` block in
`crates/pdftract-core/src/parser/catalog.rs`:

```rust
pub fn default_off_ocgs(&self) -> std::collections::HashSet<ObjRef>
```

It delegates to `OcProperties::off_ocg_set()` (`crates/pdftract-core/src/parser/ocg.rs:237`)
via `self.oc_properties.as_ref().filter(|p| p.present).map(|p| p.off_ocg_set()).unwrap_or_default()`,
so absence of `/OCProperties` (`oc_properties == None` **or** `present == false`,
matching the always-Some convention asserted at catalog.rs:1184-1186) yields an
empty set. The doc comment records the BaseState interaction: `off_ocg_set()`
only contains refs explicitly recorded in `default_visibility`; OCGs absent from
that map inherit BaseState and are treated visible per ISO 32000-2 8.11.4.4.
No behavior of `ocg.rs` was changed and no call site was touched.

## Acceptance criteria

- **PASS** — accessor compiles and is reachable from the parser module's public
  surface: `pub mod parser` (lib.rs:194) → `pub mod catalog` (parser/mod.rs:5) →
  `pub struct Catalog` → `pub fn`. Verified empirically with a throwaway probe
  crate that calls `pdftract_core::parser::catalog::Catalog::default_off_ocgs()`
  and asserts the absent branch returns an empty set — compiled and ran clean
  (`probe ok: accessor reachable, absent branch returns empty set`).
- **WARN** — `cargo build` / `cargo clippy` clean for this change, verified in an
  isolated copy of `crates/pdftract-core`: the shared working tree has sibling
  in-flight edits that break the whole-tree build independent of this bead
  (`extract.rs:2442` E0061 — a sibling added a 5th `default_off_ocgs` parameter
  to `process_content_stream_to_glyphs` without yet updating its one call site;
  the `lib test` target additionally has ~116 pre-existing errors in
  `classify.rs`, `render/scanline.rs`, `parser/xref.rs`,
  `layout/correction.rs`). Zero errors and zero new warnings reference
  `parser/catalog.rs` in either tree; the two catalog.rs warnings that do appear
  (`emit_diagnostic` never used, `from_str` naming) are pre-existing —
  `emit_diagnostic` has zero callers in the file. This bead is forbidden from
  touching call sites, so the sibling's gap is out of scope. Consequence: the
  existing catalog tests could not be *run* (the `lib test` target does not
  compile tree-wide until the sibling's classify.rs work lands); no test was
  modified and the accessor is additive, so no existing test behavior changed.
- **PASS** — no existing call site modified: the bead's diff is
  `crates/pdftract-core/src/parser/catalog.rs` (+22: this accessor, plus 7 lines
  of pre-existing test-only imports — `intern` / `IndexMap` — that were already
  dirty in the file before this bead started and are required by its own test
  module).

## Verification method note

Because the shared checkout is mid-migration by sibling beads, the
clean-compile evidence came from a temporary copy of the crate under
`~/scratch/` (trimmed workspace manifest, the sibling's one missing `None`
argument patched in the copy only, shared `Cargo.lock`), verified with
`cargo check -p pdftract-core --lib` (Finished, clean) and the probe crate, then
deleted. Nothing under `~/scratch/` was left behind.
