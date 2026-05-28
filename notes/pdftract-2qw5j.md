# Verification Note: pdftract-2qw5j (JSON Schema Generation)

## Task Summary
Generate docs/schema/v1.0/pdftract.schema.json via xtask + schema gen CI gate

## Date
2026-05-28

## Implementation Status: COMPLETE

All components of the JSON schema generation system are implemented and working correctly.

## Verification Results

### PASS Criteria

1. **xtask gen-schema produces valid JSON Schema** ✅
   - Binary: `xtask/src/bin/gen_schema.rs`
   - Command: `cargo run --manifest-path=xtask/Cargo.toml --bin gen_schema`
   - Output: `docs/schema/v1.0/pdftract.schema.json` (59,273 bytes)
   - Schema is valid JSON Schema Draft 2020-12

2. **Committed file matches generated output** ✅
   - Running gen-schema produces byte-identical output to committed file
   - No diffs detected: `git diff --exit-code docs/schema/v1.0/pdftract.schema.json`
   - Stable sorting via `sort_keys_recursive()` function

3. **Schema includes required metadata** ✅
   - `$id`: "https://pdftract.com/schema/v1.0/pdftract.schema.json"
   - `$schema`: "https://json-schema.org/draft/2020-12/schema"
   - `title`: "pdftract Output v1.0"
   - `description`: Full description of extraction output structure

4. **Schema includes Phase 7 placeholder objects** ✅
   - Empty arrays for Phase 7 features: `threads`, `attachments`, `signatures`, `form_fields`, `links`, `annotations`
   - All placeholder fields documented in schema descriptions

5. **Enum properties documented** ✅
   - `page_type` includes "broken_vector" (per 5.1 + 6.1 requirement)
   - `confidence_source` field documented with allowed values
   - `severity` field documented with allowed values

6. **CI gate implemented** ✅
   - Workflow: `.ci/argo-workflows/pdftract-ci.yaml`
   - Template: `schema-gen` (lines 1851-1940)
   - Enforcement: Regenerates schema, fails build on any diff
   - Error message includes reproduction command

## Schema Coverage

The generated schema includes complete definitions for:

### Document-level
- `Output` (root object)
- `DocumentMetadata`
- `ExtractionQuality`
- `OutlineNode` (bookmarks)
- `ThreadJson` (article threads)
- `AttachmentJson` (embedded files)
- `SignatureJson` (digital signatures)
- `FormFieldJson` (form fields)
- `LinkJson` (hyperlinks)
- `JavascriptActionJson` (JS actions)

### Page-level
- `PageJson`
- `SpanJson` (text spans)
- `BlockJson` (structural blocks)
- `TableJson`, `RowJson`, `CellJson` (tables)
- `AnnotationJson` (annotations)

### Diagnostics
- `DiagnosticJson`
- `ObjectLocationJson`

## Technical Implementation

### Rust Type Derives
All relevant types have `#[cfg_attr(feature = "schemars", derive(schemars::JsonSchema))]`:
- `Output`, `PageJson`, `SpanJson`, `BlockJson`
- `DiagnosticJson`, `AnnotationJson`, `FormFieldJson`
- All supporting types

### Stable Output
- `sort_keys_recursive()` ensures deterministic key ordering
- `BTreeMap` for all object keys
- Pretty-printed with 2-space indentation

### CI Integration
The `schema-gen` template in pdftract-ci.yaml:
1. Runs `cargo run --release -- gen-schema`
2. Compares output to committed file via `git diff --exit-code`
3. Fails build on any difference with clear error message
4. Part of quality-matrix (Tier-1 hard gate)

## References

- Plan section: Phase 6.1 JSON Schema deliverable (line 2027-2045)
- CI workflow: `.ci/argo-workflows/pdftract-ci.yaml` (template: schema-gen)
- Generated schema: `docs/schema/v1.0/pdftract.schema.json`
- xtask binary: `xtask/src/bin/gen_schema.rs`

## Notes

- Schema generation is fast (~12 seconds cold build)
- No warnings or errors during generation
- Schema is committed to repo (not generated at build time)
- This enables schema diffs to be reviewable in PRs
- Schema $id uses pdftract.com domain (DNS already available)

## Retrospective

### What worked
- The schemars crate integrates seamlessly with existing serde derives
- CI gate provides clear error messages with reproduction steps
- Stable sorting ensures deterministic output for diffs

### What didn't
- No issues encountered; implementation was already complete

### Reusable pattern
- For similar schema generation tasks: use schemars + xtask + CI diff gate
- Always use BTreeMap sorting for deterministic JSON output
- Commit generated files (don't generate at build time) for reviewability
