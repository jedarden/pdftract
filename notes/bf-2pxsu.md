# bf-2pxsu: Add logging for JavaScript execution attempts

## Task Completion Summary

Successfully added warning-level logging for JavaScript execution attempts in pdftract-core's detection module.

## Changes Made

### File: `crates/pdftract-core/src/detection.rs`

1. **Added `warn` import to logging macros:**
   - Changed `use tracing::{debug, info};` to `use tracing::{debug, info, warn};`

2. **Added execution warning at all JavaScript detection points:**
   - Catalog `/OpenAction` detection
   - Catalog `/AA` (Additional Actions) detection
   - Page-level `/AA` detection
   - Page annotation `/A` (primary action) detection
   - Page annotation `/AA` (additional actions) detection
   - AcroForm fields `/AA` detection

Each warning uses the message:
```
"JavaScript execution attempted but not supported - detection only (per TH-04 threat model)"
```

## Acceptance Criteria

- ✅ `log::debug!` or `log::warn!` macro added at any execution entry point
- ✅ Log message clearly states 'JavaScript execution attempted but not supported'
- ✅ Log includes a warning that execution is not supported (references TH-04 threat model)
- ✅ Code compiles successfully
- ✅ All tests pass (46 detection tests, 0 failed)

## Implementation Notes

Since pdftract is a detection-only tool (per TH-04 threat model), there is no actual JavaScript execution path. The warning logs fire whenever JavaScript is detected to explicitly communicate that:

1. JavaScript was found in the PDF
2. Execution is NOT supported and will NEVER occur
3. This is detection-only for downstream security review

The warnings are emitted at the same points where JavaScript detection occurs, providing clear visibility when JavaScript is present while reinforcing the security posture.

## Testing

- Compilation: ✅ `cargo check --package pdftract-core`
- Unit tests: ✅ All 46 detection-related tests pass
- No behavior changes - only added logging

## Dependencies

This bead built upon bf-2pyg1, which added the initial debug/info logging for JavaScript detection points.
