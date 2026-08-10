# Unused Imports Inventory for bf-3emwgg

**Date:** 2026-08-10  
**Files analyzed:** `crates/pdftract-core/src/parser/xref.rs` and `crates/pdftract-core/src/parser/ocg.rs`  
**Tool:** `cargo clippy --all-targets --message-format=short`

## Summary

| File | Unused Imports Found | Expected (per task description) | Status |
|------|---------------------|----------------------------------|---------|
| `xref.rs` | 1 | ~5 | **Much lower than expected** |
| `ocg.rs` | 0 | ~4 | **Much lower than expected** |

## Details

### xref.rs (`crates/pdftract-core/src/parser/xref.rs`)

**Unused imports found: 1**

1. **Line 11, column 29:** `MemorySource`
   - Imported as: `use crate::parser::stream::{MemorySource, PdfSource};`
   - Status: **Unused** - The top-level import is not used
   - Note: `MemorySource` is imported locally within function `parse_xref_stream` at line 1677, so the top-level import is redundant

**Other unused items (non-imports):**
- Line 874: `diagnostics` parameter (unused variable)
- Line 907: `depth` variable (assigned but never read)
- Line 1227: `source_len` variable (unused)
- Line 4716: `rev_base` variable (unused)
- Line 4836: `rev1_offset` variable (unused)
- Line 4966: `has_prev` variable (unused)

### ocg.rs (`crates/pdftract-core/src/parser/ocg.rs`)

**Unused imports found: 0**

All imports in `ocg.rs` are actively used:
- `std::collections::HashMap` - Used extensively
- `crate::parser::object::{ObjRef, PdfDict, PdfObject}` - All used
- `crate::parser::xref::XrefResolver` - Used in `parse_oc_properties`
- `crate::parser::{DiagCode, Diagnostic}` - Used in diagnostics

**Other unused items (non-imports):**
- Various unused variables in test code (e.g., `diagnostics`, `obj_ref`, `resolver`)
- Unused associated functions: `OcmdPolicy::from_name`, `Ocmd::parse`

## Analysis

**Why the count is lower than expected:**

The task description expected approximately 5 unused imports in `xref.rs` and 4 in `ocg.rs`, but actual analysis found only 1 and 0 respectively. This discrepancy may be due to:

1. **Recent code cleanup** - Unused imports may have been removed in previous commits
2. **Task description outdated** - The expected counts may have been estimated from an older codebase state
3. **Different compiler behavior** - Different rustc versions or clippy lints may detect different unused items

**Key finding:**
- The only unused import across both files is `MemorySource` in `xref.rs` (line 11), which is shadowed by a local import within the `parse_xref_stream` function.

## Recommendations

1. **Remove the unused `MemorySource` import from `xref.rs` line 11** - It's redundant since there's a local import within the function that uses it.

2. **Update the task description expectations** - The actual counts (1 and 0) should be documented as the baseline for future cleanup work.

3. **Consider cleaning up unused variables** - While not imports, there are several unused variables that could be removed or prefixed with underscores to indicate intentional non-use.

## Verification Command

To reproduce these findings:

```bash
cargo clippy --all-targets --message-format=short 2>&1 | grep "unused.*import" | grep -E "(xref\.rs|ocg\.rs)"
```

Output:
```
crates/pdftract-core/src/parser/xref.rs:11:29: warning: unused import: `MemorySource`
```