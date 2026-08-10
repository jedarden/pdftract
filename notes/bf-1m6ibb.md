# Test Inventory Generation (bf-1m6ibb)

## Summary
Generated cargo test inventory by extracting test function names from source code.

## Method
`cargo test --list` failed due to 235 compilation errors (missing imports: `intern`, `MemorySource`, `PdfDict`, `TableSpan`, `json!` macro). Worked around by parsing source code directly:

```bash
find /home/coding/pdftract -name "*.rs" -type f -not -path "*/.claude/worktrees/*" -not -path "*/target/*" \
  -exec awk '/#\[test\]/ {p=1; next} p==1 && /fn [a-z_][a-z0-9_]*/ {print; p=0}' {} \; | \
  sed 's/fn \([a-z_][a-z0-9_]*\).*/\1/' | sort -u > tests/cargo-test-inventory.txt
```

## Results
- **File:** `tests/cargo-test-inventory.txt`
- **Total tests:** 5221
- **Format:** One test name per line, sorted alphabetically

## Test Breakdown
- `test_*`: 724 unit tests
- `debug_*`: 15 integration/debug tests  
- `fuzz*`: 3 fuzz tests
- `verify*`: 7 verification tests
- Other: 4472 (benchmarks, examples, uncategorized)

## Acceptance Criteria Status
1. ❌ `cargo test --list` completes successfully - **FAILED**: 235 compilation errors prevent test listing
2. ✅ Output saved to `tests/cargo-test-inventory.txt`
3. ✅ File contains one test name per line
4. ✅ Total test count captured (5221)

## Notes
The cargo build has compilation errors that block `cargo test --list`. The source-code parsing approach successfully extracted the test inventory despite build failures. This provides a baseline for verifying test completeness once the compilation errors are resolved.

## Compilation Errors (summary)
Main missing imports in test code:
- `intern()` function in `parser/catalog.rs`, `parser/pages.rs`
- `MemorySource` type in `parser/xref.rs`  
- `PdfDict` type in `parser/resources.rs`
- `TableSpan` type in `table/output.rs`
- `json!` macro in `schema/mod.rs`
- Various types in test modules (`ObjRef`, `Arc`, `FitType`)

These should be fixed before attempting to run `cargo test --list` again.
