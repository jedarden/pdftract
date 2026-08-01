# bf-2gsn0a — Truncated-flate test structure and error handling pattern

**Type:** research · **Parent:** [[bf-4bx00]] (error handling implementation)
**Purpose:** Document the truncated-flate test implementation, extraction result structure, and error assertion pattern from bf-2h1nt.

---

## 1. Test file structure

**Location:** `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`

The test file contains multiple integration tests for truncated FlateDecode stream recovery:
- `test_truncated_flate_fixture_exists` — basic fixture validation
- `test_truncated_flate_parses_as_pdf` — verifies PDF structure parsing
- `test_truncated_flate_emits_diagnostics` — scaffold test for diagnostic API
- `test_truncated_flate_partial_content_accessible` — page structure access
- `test_truncated_flate_extraction_result_structure` — examines extraction result
- `test_truncated_flate_materialize_pages` — materialize_pages() behavior
- `test_truncated_flate_extract_page_returns_result` — Result<PageExtraction> contract
- `test_truncated_flate_opens_with_extractor` — PdfExtractor::open() smoke test
- `test_truncated_flate_emits_stream_decode_error` — **KEY TEST for error assertion pattern**

### Key test for error assertion: `test_truncated_flate_emits_stream_decode_error`

Lines 348-397 demonstrate the complete pattern for asserting STREAM_DECODE_ERROR diagnostics:

```rust
#[test]
fn test_truncated_flate_emits_stream_decode_error() {
    let path = fixture_path();

    // Extract using extract_pdf to get full ExtractionResult with metadata.diagnostics
    let extraction_result = extract_pdf(&path, &ExtractionOptions::default())
        .expect("Should extract truncated-flate.pdf");

    // Check metadata.diagnostics field for STREAM_DECODE_ERROR
    let diagnostics = &extraction_result.metadata.diagnostics;

    // Assert that STREAM_DECODE_ERROR appears in the diagnostics
    // Following pattern from bf-2h1nt: use .contains() on Vec<String>
    let has_stream_decode_error = diagnostics
        .iter()
        .any(|d| d.contains("STREAM_DECODE_ERROR"));

    assert!(
        has_stream_decode_error,
        "Expected STREAM_DECODE_ERROR diagnostic not found. \
         Got {} diagnostics: {:?}",
        diagnostics.len(),
        diagnostics
    );
}
```

**Fixture path:** `tests/fixtures/malformed/truncated-flate.pdf`

---

## 2. Extraction result type and errors field

### Type hierarchy

```
ExtractionResult (src/extract.rs:237)
├── fingerprint: String
├── pages: Vec<PageResult>
├── metadata: ExtractionMetadata (src/extract.rs:396) ← ERRORS FIELD HERE
│   ├── page_count: usize
│   ├── span_count: usize
│   ├── block_count: usize
│   ├── error_count: usize
│   └── diagnostics: Vec<String> (line 416) ← THE ERRORS ARRAY
└── ... (signatures, form_fields, links, attachments, threads, javascript_actions)
```

### The errors/diagnostics field location

**File:** `crates/pdftract-core/src/extract.rs:416`

```rust
pub struct ExtractionMetadata {
    /// ... other fields ...
    
    /// Diagnostics emitted during extraction (coverage warnings, etc.)
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub diagnostics: Vec<String>,  // ← THIS IS THE ERRORS ARRAY
    
    /// ... more fields ...
}
```

### How to access in tests

1. **Via `extract_pdf()`** (recommended for full extraction):
   ```rust
   let extraction_result = extract_pdf(&path, &ExtractionOptions::default())?;
   let diagnostics = &extraction_result.metadata.diagnostics;
   ```

2. **Via `PdfExtractor`** (for stepwise extraction):
   ```rust
   let mut extractor = PdfExtractor::open(&path)?;
   let pages = extractor.materialize_pages()?;
   let page_result = extractor.extract_page(0)?;
   // Note: individual page extraction may not populate metadata.diagnostics
   ```

### Diagnostic format

Each diagnostic string follows the format: `"CODE: message"`

Example: `"STREAM_DECODE_ERROR: truncated zlib stream at offset 12345"`

The code part (before the colon) is what we test against.

---

## 3. Pattern from bf-2h1nt (error assertion examples)

**Source:** `notes/bf-2h1nt.md`

### Key principle: Match the representation

The codebase has **three error representations**, each with different assertion idioms:

| Representation | Type | Assertion idiom |
|---|---|---|
| Per-layer `Vec<Diagnostic>` (internal) | `Diagnostic { code: DiagCode, … }` | `diags.iter().any(\|d\| d.code == DiagCode::X)` |
| `ExtractionMetadata.diagnostics` | `Vec<String>` (each `"CODE: message"`) | **`.iter().any(\|d\| d.contains("CODE"))`** ← USE THIS |
| JSON `Output.errors` | `Vec<DiagnosticJson>` (code: String) | `.any(\|e\| e.code == "STRING")` |

### For `Vec<String>` diagnostics (our case)

**Pattern:** Use `.contains("CODE")` filtering

```rust
// Presence check (does the code appear at all?)
let has_stream_decode_error = diagnostics
    .iter()
    .any(|d| d.contains("STREAM_DECODE_ERROR"));

assert!(has_stream_decode_error, "error message with {:?}", diagnostics);
```

```rust
// Count check (does it appear exactly N times?)
let count = diagnostics
    .iter()
    .filter(|d| d.contains("STREAM_DECODE_ERROR"))
    .count();

assert_eq!(count, 1, "expected 1, got {}", count);
```

**Why use `.contains()` not `.eq()`?**
- Diagnostic strings are `"CODE: message"` format
- We want to match on the code part regardless of message
- `.contains("CODE")` matches both `"CODE"` and `"CODE: details"`

### Failure message conventions

1. **Show what was expected**: name the specific code you're looking for
2. **Show what was found**: dump the full diagnostics slice or collected codes
3. **Include context**: length of slice, test name, fixture path

**Example:**
```rust
assert!(
    has_stream_decode_error,
    "Expected STREAM_DECODE_ERROR diagnostic not found. \
     Got {} diagnostics: {:?}",
    diagnostics.len(),
    diagnostics
);
```

### The dominant pattern in the codebase

From bf-2h1nt research:
- 51 sites use `assert_eq!(diag.code, DiagCode::X)` for single diagnostics
- 56 sites use `.iter().any(|d| d.code == DiagCode::X)` for presence checks
- 16 sites use `.iter().filter(...).count()` for exact counts
- 9 sites use `.contains("CODE")` on `Vec<String>` (our case)

**The enum (`DiagCode`) is preferred**, but when you only have `Vec<String>`, use `.contains("CODE")`.

### For reusable assertions (optional)

The `xref_helpers.rs` module provides reusable diagnostic assertion helpers:
- `assert_diagnostic(diagnostics: &[Diagnostic], code: DiagCode)`
- `assert_diagnostic_count(diagnostics: &[Diagnostic], code: DiagCode, count: usize)`

**Note:** These work on `&[Diagnostic]`, not `Vec<String>`, so they're not directly applicable to `metadata.diagnostics`.

---

## 4. Where assertion should be placed

Based on the test structure, assertions should be placed in:

### Option 1: In the existing test (RECOMMENDED)

**Test:** `test_truncated_flate_emits_stream_decode_error` (lines 348-397)

This test already demonstrates the correct pattern. Add your assertion following this example:
1. Extract PDF using `extract_pdf()` to get `ExtractionResult`
2. Access `extraction_result.metadata.diagnostics`
3. Use `.iter().any(|d| d.contains("CODE"))` to check for specific error
4. Provide clear failure message showing what was found

### Option 2: In a new dedicated test

Create a new test following the naming pattern:
```rust
#[test]
fn test_truncated_flate_<specific_error_condition>() {
    // Same structure as test_truncated_flate_emits_stream_decode_error
}
```

### Option 3: In an aggregation loop

If running multiple fixtures in a loop (like `stream_decoder_fixtures.rs` pattern):
```rust
for fixture in fixtures {
    let result = extract_pdf(&fixture.path, &ExtractionOptions::default())?;
    let diagnostics = &result.metadata.diagnostics;
    
    if !diagnostics.iter().any(|d| d.contains("EXPECTED_CODE")) {
        failures.push(format!(
            "Fixture {}: Expected EXPECTED_CODE, got {:?}",
            fixture.name, diagnostics
        ));
    }
}

assert!(failures.is_empty(), "Some fixtures failed:\n{}", failures.join("\n"));
```

---

## 5. Acceptance criteria status

- [x] **Test file structure understood** — §1 documents file location, test names, and purposes
- [x] **Errors array location identified** — §2 shows `ExtractionResult.metadata.diagnostics: Vec<String>` at `src/extract.rs:416`
- [x] **Pattern from bf-2h1nt documented** — §3 explains `.contains("CODE")` idiom for `Vec<String>` diagnostics
- [x] **Clear plan for where assertion should be placed** — §4 provides three options with recommended approach

## 6. Key references

- Test file: `crates/pdftract-core/tests/test_truncated_flate_recovery.rs`
- Extraction result type: `crates/pdftract-core/src/extract.rs:237-285`
- Diagnostics field: `crates/pdftract-core/src/extract.rs:416`
- Error assertion pattern catalog: `notes/bf-2h1nt.md`
- Parent bead: [[bf-4bx00]]
