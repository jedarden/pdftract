# Test Failure Root Cause Analysis

**Bead ID:** bf-n5w42  
**Date:** 2026-07-07  
**Parent:** bf-60vlq  
**Dependencies:** bf-1wczm (test execution)

## Executive Summary

Test execution from bead bf-1wczm completed **without any hangs or timeouts**. All 4 test modules terminated cleanly in under 1 second each. However, **11 tests failed across 2 modules** due to fixture issues, not test logic problems.

## Test Modules Summary

| Module | Status | Pass/Fail | Duration |
|--------|--------|-----------|----------|
| `unmapped_glyph_names_config` | ✅ PASS | 4/4 | <1s |
| `encoding_recovery` | ❌ FAIL | 1/6 | <1s |
| `cjk_encoding` | ❌ FAIL | 0/5 | <1s |
| `cmap_unmapped_glyphs` | ✅ PASS | 7/7 | <1s |

**Total:** 12/22 tests passing (54.5%), **0 hangs detected**

## Root Cause Analysis

### Issue 1: encoding_recovery Module - Corrupted PDF Fixtures

**Affected Tests (5 failures):**
- `test_agl_only_fixture` 
- `test_fingerprint_match_fixture`
- `test_shape_match_fixture`
- `test_no_mapping_fixture`
- `test_corpus_recovery_rate`

**Error Patterns:**

1. **"No /Root reference in trailer"** (3 tests)
   ```
   called `Result::unwrap()` on an `Err` value: 
   "Failed to open PDF: No /Root reference in trailer"
   ```
   - Affects: `agl-only.pdf`, `fingerprint-match.pdf`, `shape-match.pdf`
   - Root cause: PDF trailer is missing the required `/Root` catalog reference
   - PDF structure violation: The trailer must point to the document catalog via `/Root`

2. **"Page index 0 out of bounds (document has 0 pages)"** (2 tests)
   ```
   Failed to extract page: Page index 0 out of bounds (document has 0 pages)
   ```
   - Affects: `no-mapping.pdf`
   - Root cause: PDF file has no page tree or empty page count
   - Test attempts to extract page 0 but document reports 0 pages

**Fixture Files Location:** `/home/coding/pdftract/tests/fixtures/encoding/`

**Files Requiring Regeneration:**
- `agl-only.pdf`
- `fingerprint-match.pdf`  
- `shape-match.pdf`
- `no-mapping.pdf`

**Evidence:** Files exist (confirmed via `ls -la`) but are structurally invalid PDFs.

### Issue 2: cjk_encoding Module - Path Resolution Failure

**Affected Tests (5 failures):**
- `test_all_cjk_fixtures_exist`
- `test_cjk_big5_traditional_chinese`
- `test_cjk_gb18030_chinese`
- `test_cjk_shiftjis_japanese`
- `test_cjk_euckr_korean`

**Error Pattern:**
```
CJK fixture PDF should exist: ../../../tests/fixtures/cjk/cjk-chinese-gb18030.pdf
Failed to open PDF: Failed to open PDF file
```

**Root Cause:** Path resolution failure due to test working directory mismatch

**Test Code Path:** `../../../tests/fixtures/cjk/cjk-chinese-gb18030.pdf`  
**Actual File Location:** `/home/coding/pdftract/tests/fixtures/cjk/cjk-chinese-gb18030.pdf`  
**Test Working Directory:** `target/debug/deps/` (cargo test default)

The relative path `../../../tests/fixtures/` resolves from `target/debug/deps/` to `tests/fixtures/` in the parent directory, but cargo tests run with the workspace root as cwd in some configurations and target/deps in others. This creates intermittent path resolution failures.

**Files Confirmed to Exist:**
- `cjk-chinese-gb18030.pdf` ✅ (822 bytes)
- `cjk-japanese-shiftjis.pdf` ✅ (822 bytes)
- `cjk-korean-euckr.pdf` ✅ (826 bytes)
- `cjk-tc-big5.pdf` ✅ (814 bytes)

All files are present and non-zero in size, but cannot be found due to path resolution.

## No Test Hangs Detected

Per CLAUDE.md test hygiene requirements, I verified for hangs/timeouts:

✅ All modules completed in <1 second  
✅ No process leaks detected  
✅ No port conflicts observed  
✅ No frozen `cargo test` processes  
✅ No orphaned `pdftract mcp` server processes  

The test execution was clean and rapid. Failures are immediate assertion panics, not hanging timeouts.

## Proposed Fix Strategy

### Priority 1: Fix encoding_recovery Fixtures

1. **Regenerate corrupted PDFs:**
   ```bash
   cd /home/coding/pdftract/tests/fixtures/encoding/
   # Re-run fixture generation scripts
   python3 create_unmapped_comprehensive.py
   python3 generate_unmapped_glyphs.py
   ```

2. **Validate regenerated PDFs:**
   ```bash
   # Verify /Root reference exists
   pdfgrep -a "Root" agl-only.pdf
   # Verify page count > 0
   pdfinfo agl-only.pdf | grep "Pages:"
   ```

3. **Test after regeneration:**
   ```bash
   cargo test -p pdftract-core --test encoding_recovery -- --test-threads=1
   ```

### Priority 2: Fix cjk_encoding Path Resolution

**Option A: Use CARGO_MANIFEST_DIR (Recommended)**
```rust
// In cjk_encoding.rs
fn get_fixtures() -> Vec<CjkFixture> {
    let manifest_dir = env!("CARGO_MANIFEST_DIR");
    let fixture_base = format!("{}/tests/fixtures/cjk", manifest_dir);
    
    vec![
        CjkFixture {
            name: "chinese-gb18030",
            pdf_path: &format!("{}/cjk-chinese-gb18030.pdf", fixture_base),
            // ...
        },
    ]
}
```

**Option B: Use find-crate crate for dynamic path resolution**
```rust
use find_crate::{Find, Source};

fn get_fixture_path(fixture_name: &str) -> String {
    let pkg = Find::current().expect("find package");
    let source = pkg.source;
    format!("{}/tests/fixtures/cjk/{}", source, fixture_name)
}
```

**Option C: Use std::path absolute resolution**
```rust
fn get_fixtures() -> Vec<CjkFixture> {
    let base = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/cjk");
    
    vec![
        CjkFixture {
            name: "chinese-gb18030",
            pdf_path: base.join("cjk-chinese-gb18030.pdf").to_str().unwrap(),
            // ...
        },
    ]
}
```

### Priority 3: Verify Unmapped Glyph Tests

The `cmap_unmapped_glyphs` and `unmapped_glyph_names_config` modules pass cleanly. No action needed.

## Test Hygiene Verification

Per CLAUDE.md test hygiene section, I checked for known failure modes:

- ✅ **Process leaks:** None detected (all processes terminated cleanly)
- ✅ **Port conflicts:** N/A (tests don't bind ports)
- ✅ **Timeout issues:** None (all tests finished instantly)
- ✅ **Orphaned processes:** None (no subprocess spawning in test code)
- ✅ **Undrained pipes:** N/A (no server subprocess spawning)

The test failures are purely fixture/path issues, not the hanging/timeout patterns warned about in CLAUDE.md.

## Recommendations for Parent Bead (bf-60vlq)

1. **Add fixture validation to CI:**
   - Run `pdfinfo` on all PDF fixtures before test execution
   - Fail fast if fixtures are corrupted

2. **Add path resolution tests:**
   - Unit test to verify fixture files are discoverable at test initialization
   - Panic early with clear error message if paths fail

3. **Document fixture generation:**
   - Add README.md in `tests/fixtures/` explaining how to regenerate fixtures
   - Include validation commands

4. **Consider cargo-nextest for hung test detection:**
   - Migrate test execution from `cargo test` to `cargo nextest run`
   - Leverage `slow-timeout` configuration for automatic hung test termination
   - Add `.config/nextest.toml` with per-test timeout settings

## Conclusion

**No test hangs occurred.** All test modules executed rapidly and terminated cleanly. The 11 test failures are due to:
- 5 corrupted PDF fixtures (encoding_recovery)
- 5 path resolution failures (cjk_encoding)

Both issues are fixable with fixture regeneration and path resolution code changes. The underlying test logic for unmapped glyph assertions is working correctly (11/11 tests passing in the 2 passing modules).
