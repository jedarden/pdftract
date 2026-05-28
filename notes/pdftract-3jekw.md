# pdftract-3jekw: Watermark / Formula Detection Stubs (Phase 7 Deferred)

## Work Completed

### 1. Module Created
Created `crates/pdftract-core/src/layout/watermark_formula.rs` with stub implementations:
- `classify_watermark(block) -> bool`: Always returns `false` (Phase 4 stub)
- `classify_formula(block) -> bool`: Always returns `false` (Phase 4 stub)

### 2. Module Integration
Updated `crates/pdftract-core/src/layout/mod.rs`:
- Added module declaration: `pub mod watermark_formula;`
- Added public exports: `pub use watermark_formula::{classify_formula, classify_watermark};`
- Updated module documentation to reference the stub classifiers

### 3. Module Documentation
The module includes comprehensive documentation:
- Links to Phase 7 research notes for watermark detection (`docs/research/watermark-and-background-separation.md`)
- References plan.md Phase 7.1 (watermark) and Phase 7.2 (formula) specifications
- TODO comments outlining the full implementation requirements

### 4. Tests
Module includes 4 tests verifying stub behavior:
- `test_classify_watermark_always_false`: Verifies watermark stub returns false
- `test_classify_formula_always_false`: Verifies formula stub returns false
- `test_watermark_stub_documentation`: Documents Phase 4 behavior
- `test_formula_stub_documentation`: Documents Phase 4 behavior

## Acceptance Criteria

| Criterion | Status | Notes |
|-----------|--------|-------|
| BlockKind::Watermark variant exists | PASS | Already present in `parser/struct_tree.rs:1424` |
| BlockKind::Formula variant exists | PASS | Already present in `parser/struct_tree.rs:1422` |
| classify_watermark always false | PASS | Stub function returns `false` |
| classify_formula always false | PASS | Stub function returns `false` |
| No v0.1.0 block has kind=Watermark or Formula | PASS | Stubs ensure no blocks are classified |

## Plan References
- Phase 4.4 (line 1709): Block formation and kind assignment
- Phase 4.6 (line 1752): Watermark exclusion note ("Prior to Phase 7, watermarks are not excluded from --text output; kind: 'watermark' blocks are not emitted")
- Phase 7.1: Watermark detection (deferred)
- Phase 7.2: Formula detection (deferred)

## Implementation Details

### BlockKind Enum (existing)
```rust
// crates/pdftract-core/src/parser/struct_tree.rs
pub enum BlockKind {
    // ...
    Formula,      // Line 1422
    Watermark,    // Line 1424 (commented: "Phase 7 stub - always false")
    // ...
}
```

### Stub Functions (new)
```rust
// crates/pdftract-core/src/layout/watermark_formula.rs
pub fn classify_watermark<S>(_block: &Block<S>) -> bool {
    false  // Phase 4 stub
}

pub fn classify_formula<S>(_block: &Block<S>) -> bool {
    false  // Phase 4 stub
}
```

## Verification Note
The stubs are correctly implemented and will be upgraded to full detection logic in Phase 7. The existence of these stubs allows downstream consumers (JSON schema, markdown, profile extraction) to be coded against the full taxonomy without breaking changes later.

## Files Modified
- `crates/pdftract-core/src/layout/watermark_formula.rs` (new)
- `crates/pdftract-core/src/layout/mod.rs` (module declaration and exports)
