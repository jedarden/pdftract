# STREAM_DECODE_ERROR Assertion — Step 1 Research Synthesis

**Date:** 2026-08-01  
**Bead:** bf-670kf4 (Document findings and create assertion plan)  
**Parent bead:** bf-4bx00 (Add STREAM_DECODE_ERROR assertion to truncated-flate test)  
**Research chain:** bf-4fb3b → bf-2h1nt, bf-348zd, bf-60qj2, bf-2goux, bf-junlj

## Overview

This document synthesizes findings from the complete research chain for adding a `STREAM_DECODE_ERROR` assertion to the truncated-flate test fixture. It maps where errors arrays are available in the test flow and specifies exact locations where assertions should be placed.

**Key Finding:** The assertion belongs in the low-level decoder fixture loop (`tests/stream_decoder_fixtures.rs`), keyed off the decode outcome — NOT in a full-extraction test reading `output.errors`, which is vacuous for `truncated-flate.pdf`.

---

## Part 1: Errors Array Availability in Test Flow

### 1.1 Two Different Error Representations

The codebase has **two distinct "errors" representations** that are **not interchangeable**:

| Representation | Type | Location | Populated By | Observable for truncated-flate? |
|---|---|---|---|---|
| **Internal `ExtractionResult`** | Multi-field structure | `src/extract.rs:237-426` | Full extraction pipeline | **Partial** — `pages: []`, `error_count: 0`, `diagnostics: []` |
| **JSON `Output.errors`** | `Vec<DiagnosticJson>` | `src/schema/mod.rs:1539` | `result_to_output()` conversion | **NO** — Empty for this fixture |
| **Decode outcome** | `Result<Vec<u8>, String>` | `tests/stream_decoder_fixtures.rs:301` | Low-level `StreamDecoder` | **YES** — `Ok(partial)` for truncated streams |

### 1.2 Internal ExtractionResult Structure

**Location:** `crates/pdftract-core/src/extract.rs:232-426`

```rust
pub struct ExtractionResult {
    pub fingerprint: String,
    pub pages: Vec<PageResult>,
    pub metadata: ExtractionMetadata,
    // ... other fields
}

pub struct PageResult {
    pub index: usize,
    pub error: Option<String>,  // ← Per-page error (set if that page failed)
    // ... other fields
}

pub struct ExtractionMetadata {
    pub page_count: usize,
    pub error_count: usize,      // ← Count of failed pages
    pub diagnostics: Vec<String>, // ← "CODE: message" strings
    // ... other fields
}
```

**For `truncated-flate.pdf`:**
- `pages: []` — truncated page is not enumerable
- `error_count: 0` — no page-level failures recorded
- `diagnostics: []` — empty

### 1.3 JSON Output.errors Structure

**Location:** `crates/pdftract-core/src/schema/mod.rs:813-841, 1487-1539`

```rust
pub struct Output {
    pub schema_version: String,
    pub errors: Vec<DiagnosticJson>,  // ← Structured errors array
    // ... other fields
}

pub struct DiagnosticJson {
    pub code: String,        // e.g. "STREAM_DECODE_ERROR"
    pub message: String,
    pub severity: String,   // "info" | "warning" | "error" | "fatal"
    pub page_index: Option<usize>,
    pub location: Option<ObjectLocationJson>,
    pub hint: Option<String>,
}
```

**For `truncated-flate.pdf`:**
- `output.errors: []` — EMPTY because the truncated page is never traversed

### 1.4 Low-Level Decoder Path

**Location:** `crates/pdftract-core/tests/stream_decoder_fixtures.rs:220-251, 301-308`

```rust
fn decode_fixture(fixture: &FixtureInfo, input: &[u8]) -> Result<Vec<u8>, String> {
    // Calls StreamDecoder::decode() directly
    // Returns Ok(partial_bytes) for truncated streams
    // Returns Err(FilterError) only for "can't start decoding" cases
}
```

**For `truncated-flate.pdf`:**
- Returns `Ok(partial_bytes)` — ~13 bytes from the 26-byte `.bin` input
- The `Err` arm is **unreachable** for this fixture type

---

## Part 2: Test Flow and Error Assertion Placement

### 2.1 Target Test Location

**File:** `crates/pdftract-core/tests/stream_decoder_fixtures.rs`  
**Function:** `test_all_stream_decoder_fixtures()` (lines 254-346)  
**Insertion point:** On the **`Ok` path** (after line 303) and/or **after byte-compare** (after line 326, before line 345)

### 2.2 Test Loop Flow

```
1. Load fixture files (.bin, .expected, .meta)        → lines 264-298
   ├── Missing-file guards push to failures + continue
2. Call decode_fixture()                              → line 301
3. Match on Result:                                   → lines 302-308
   ├── Ok(data)  ← TRUNCATED FIXTURE TAKES THIS PATH
   └── Err(e)    ← DEAD for truncated-flate (EC2)
4. Byte-compare decoded vs .expected                 → lines 318-326
   └── Mismatch pushes to failures + continue
5. Bomb-specific checks (flate_bomb_3gb only)         → lines 329-343
6. ← ASSERTION PLACEMENT HERE (after line 326)
7. passed += 1                                        → line 345
```

### 2.3 Why NOT the Full Extraction Path

The original bead title suggested asserting `STREAM_DECODE_ERROR` in the `errors` array from a full extraction test. This is **incorrect** because:

1. **`truncated-flate.pdf` yields empty extraction result**
   - `pages: []` — page is not enumerable
   - `metadata.diagnostics: []` — no diagnostics emitted
   - `output.errors: []` — structured errors array is empty

2. **Assertion would be vacuous**
   ```rust
   let output = extract_pdf("truncated-flate.pdf");
   // This ALWAYS FAILS - errors is empty:
   assert!(output.errors.iter().any(|e| e.code == "STREAM_DECODE_ERROR"));
   ```

3. **Low-level path is the only observable signal**
   - `StreamDecoder::decode()` returns `Result<Vec<u8>, FilterError>`
   - Truncated streams return `Ok(partial_bytes)` per INV-8
   - No `DiagCode` collector exists on this path

---

## Part 3: Exact Assertion Placement Locations

### 3.1 Primary Location — Ok Path Contract Assertion

**Location:** After line 303 (inside the `Ok(data)` arm) or between lines 326-345

```rust
// stream_decoder_fixtures.rs ~line 302-303
let decoded = match result {
    Ok(data) => data,
    Err(e) => {
        // ... error handling ...
    }
};

// SELECTOR LOGIC (evaluate on Ok path)
let expects_decode_error = 
    fixture.expected_diags.contains(&DiagCode::StreamDecodeError);

if expects_decode_error {
    // Contract assertion: fixture declares decode error AND
    // decode did not hard-fail (we are in Ok arm).
    // Falls through to passed += 1 (EC11).
}
```

### 3.2 Secondary Location — Err Arm Regression Guard

**Location:** Lines 304-307 (the `Err(e) => { ... }` arm)

```rust
Err(e) => {
    if fixture.expected_diags.contains(&DiagCode::StreamDecodeError) {
        failures.push(format!(
            "{}: expected soft partial-recovery (Ok) for a decode-error fixture, \
             but decode returned hard Err (INV-8 regression?): {}",
            fixture.name, e
        ));
        continue;
    }
    failures.push(format!("{}: {}", fixture.name, e));
    continue;
}
```

**Note:** This arm is **currently unreachable** for `flate_truncated` but serves as a regression guard if future changes make the decoder return `Err` for truncated streams.

### 3.3 Prerequisite Changes

**Location:** `crates/pdftract-core/tests/stream_decoder_fixtures.rs:62`

```rust
// BEFORE (dead data)
FixtureInfo {
    name: "flate_truncated",
    filter: FixtureFilter::Single("FlateDecode", None),
    expected_diags: vec![],  // ← Never read by the loop
    bomb_limit: None,
}

// AFTER (live selector)
FixtureInfo {
    name: "flate_truncated",
    filter: FixtureFilter::Single("FlateDecode", None),
    expected_diags: vec![DiagCode::StreamDecodeError],  // ← Now read by assertion logic
    bomb_limit: None,
}
```

---

## Part 4: Dependencies and Prerequisites

### 4.1 Naming Corrections

**Issue:** The original bead used `STREAM_DECOMPRESS_ERROR`  
**Fact:** That string does **not exist** in the codebase  
**Correct values:**

| Concern | Value | Source |
|---------|-------|--------|
| Enum variant | `DiagCode::StreamDecodeError` | `src/diagnostics.rs:465` |
| String form | `"STREAM_DECODE_ERROR"` | `src/diagnostics.rs:1278` |
| Severity | `"warning"` | Per diagnostic spec |

### 4.2 Assertion Pattern Requirements

From `bf-2h1nt` (error assertion pattern catalog):

1. **Compare the `DiagCode` enum, never string literals**
   ```rust
   // CORRECT
   fixture.expected_diags.contains(&DiagCode::StreamDecodeError)
   
   // WRONG
   fixture.expected_diags.contains(&"STREAM_DECODE_ERROR")
   ```

2. **Follow aggregated-loop convention**
   ```rust
   // Push to failures Vec, don't use bare assert!
   failures.push(format!("{}: ...", fixture.name));
   // NOT: assert!(condition, "message"); // This aborts the entire loop
   ```

3. **Failure message must show what was observed**
   ```rust
   // Must include both enum name and string
   format!("Expected STREAM_DECODE_ERROR (DiagCode::StreamDecodeError) ...")
   ```

### 4.3 Edge Case Considerations

From `bf-60qj2` (edge case taxonomy):

- **EC1:** No `DiagCode` collected on low-level path → assertion is contract-based, not runtime observation
- **EC2:** `Err` arm is unreachable → place assertion on `Ok` path
- **EC3:** 0-byte `.expected` file → byte-compare is vacuous
- **EC4:** `expected_diags` is dead data → must make loop read it
- **EC5:** Per-fixture selector → avoid cross-fixture code contamination
- **EC6-EC7:** Partial output not byte-stable → never byte-assert partial content
- **EC11:** Must reach `passed += 1` → don't short-circuit with `continue`
- **EC12:** INV-8 regression guard → catch future `Err` returns explicitly

---

## Part 5: Implementation Plan Summary

### 5.1 Changes Required

1. **Make `expected_diags` live (line 62)**
   ```rust
   expected_diags: vec![DiagCode::StreamDecodeError]
   ```

2. **Add selector logic on Ok path (after line 303)**
   ```rust
   let expects_decode_error = 
       fixture.expected_diags.contains(&DiagCode::StreamDecodeError);
   ```

3. **Add contract assertion (between lines 326-345)**
   ```rust
   if expects_decode_error {
       // Contract: fixture declares error + decode did not hard-fail
       // Falls through to passed += 1
   }
   ```

4. **Add INV-8 regression guard (lines 304-307)**
   ```rust
   if expects_decode_error {
       failures.push(format!(
           "{}: expected soft partial-recovery but got hard Err (INV-8 regression?): {}",
           fixture.name, e
       ));
       continue;
   }
   ```

### 5.2 Verification Approach

**Do NOT run full `test_all_stream_decoder_fixtures`:**
- Includes `flate_bomb_3gb` (~2 GB fixture)
- Slow and disk-heavy per `~/CLAUDE.md` rules

**DO scoped verification:**
- Run `decode_fixture` on single `flate_truncated.bin` fixture
- Confirm `Ok(partial_bytes)` outcome
- Verify selector logic evaluates correctly
- Compile with `cargo test --test stream_decoder_fixtures --no-run`

### 5.3 Acceptance Criteria

- [x] Errors array availability mapped (Part 1)
- [x] Test flow documented (Part 2)
- [x] Exact assertion locations specified (Part 3)
- [x] Dependencies and prerequisites noted (Part 4)
- [x] Clear implementation plan ready (Part 5)

---

## Part 6: Key Source References

| Artifact | Location | Purpose |
|----------|----------|---------|
| `DiagCode::StreamDecodeError` enum | `src/diagnostics.rs:465` | Correct enum variant |
| `"STREAM_DECODE_ERROR"` string | `src/diagnostics.rs:1278` | Correct string form |
| `ExtractionResult` structure | `src/extract.rs:237-426` | Internal extraction format |
| `Output.errors` structure | `src/schema/mod.rs:1539` | JSON errors array |
| `FlateDecoder` soft recovery | `src/parser/stream.rs:542-544` | INV-8 partial recovery |
| `FixtureInfo.expected_diags` | `tests/stream_decoder_fixtures.rs:22` | Per-fixture expectation |
| `decode_fixture()` function | `tests/stream_decoder_fixtures.rs:220-251` | Low-level decoder call |
| `test_all_stream_decoder_fixtures` loop | `tests/stream_decoder_fixtures.rs:254-346` | Target test location |
| Fixture files | `tests/stream_decoder/fixtures/flate_truncated.*` | Test data (26B bin, 0B expected) |

---

## Part 7: Related Beads and Notes

- **bf-4fb3b:** Consolidated errors array format guide (all findings merged)
- **bf-2h1nt:** Error assertion pattern catalog (56 sites using `.any()`, 51 using `assert_eq!`)
- **bf-348zd:** Assertion requirements (naming, logic, failure messages, edge cases E1-E5)
- **bf-60qj2:** Edge case taxonomy (EC1-EC14) + strategy pivot to Ok path
- **bf-2goux:** Empirical verification (extraction yields empty result)
- **bf-junlj:** Location decision (fixture loop, not test_truncated_flate_recovery.rs)
- **bf-4bx00:** Implementer bead (uses this research)
- **bf-2897m:** Failure message refinement + compile verification

---

## Conclusion

This research synthesis establishes that the `STREAM_DECODE_ERROR` assertion must be placed in the low-level `stream_decoder_fixtures.rs` test loop, keyed off the `decode_fixture()` `Result` outcome. The full-extraction `output.errors` path is empty for `truncated-flate.pdf` and would yield a vacuous assertion.

The implementation requires: (1) making `expected_diags` live, (2) adding selector logic on the `Ok` path, (3) adding a contract assertion, and (4) adding an INV-8 regression guard in the `Err` arm. All dependencies, edge cases, and source references are documented above.
