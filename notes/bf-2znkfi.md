# Bead bf-2znkfi: Fix unused imports in ocg.rs

## Task
Remove unused imports from `crates/pdftract-core/src/parser/ocg.rs` based on the inventory from the first step.

## Investigation
The task description claimed "ocg.rs has 4 unused imports" but investigation showed this was incorrect.

## Verification Results

### Unused Imports Found
**None.** There are zero unused imports in ocg.rs.

### All Imports in ocg.rs Are Used
```rust
use std::collections::HashMap;                                              // Used: lines 196, 198, 201, 202, 212, 213, 214, 215, 291, 292, 293, 294
use crate::parser::object::{ObjRef, PdfDict, PdfObject};                  // All used extensively
use crate::parser::xref::XrefResolver;                                      // Used: line 282
use crate::parser::{DiagCode, Diagnostic};                                  // Both used throughout
```

### Verification Commands
```bash
cargo check --all-targets      # No warnings for ocg.rs
cargo clippy --all-targets      # No unused import warnings for ocg.rs
```

### Other Warnings (Not Imports)
The file has other code quality warnings (unused variables), but these are outside the scope of this task:
- Unused variable `diagnostics` in `OcGroup::parse` (line 144)
- Unused variable `obj_ref` in `make_test_ocg` (line 433)

## Conclusion
No changes needed. The task description was based on outdated/incorrect information. ocg.rs has zero unused imports.

## References
- Parent inventory: commit 6ca3c2af (notes/bf-3emwgg-inventory.md)
- That inventory correctly identified: xref.rs had 1 unused import (MemorySource), ocg.rs had 0
