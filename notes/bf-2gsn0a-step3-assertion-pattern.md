# Error Assertion Pattern Template

**Source:** Bead bf-2h1nt research (verified 2026-07-22)

## Core Rule

**Always compare the `DiagCode` enum variant, never string literals.**

The `DiagCode` enum derives `PartialEq`, so `d.code == DiagCode::StreamDecodeError` compiles and is the correct form.

## Three Canonical Patterns

### Pattern 1: Single Diagnostic (51 sites)

Use when expecting exactly one diagnostic, typically after `unwrap_err()` or indexing `[0]`:

```rust
// From unwrap_err()
let diag = result.unwrap_err();
assert_eq!(diag.code, DiagCode::StructCircularRef);

// From indexed Vec<Diagnostic>
assert_eq!(diagnostics.len(), 1);
assert_eq!(diagnostics[0].code, DiagCode::FontGlyphUnmapped);
```

### Pattern 2: Presence in Slice (56 sites)

Use to check "did this code fire at all?":

```rust
// Using == (preferred)
assert!(diags.iter().any(|d| d.code == DiagCode::StructMissingKey));

// Using matches! (accepted variant)
let has_remote_diagnostic = result
    .diagnostics
    .iter()
    .any(|d| matches!(d.code, DiagCode::XrefRemoteNoForwardScan));
assert!(has_remote_diagnostic, "Expected XREF_REMOTE_NO_FORWARD_SCAN diagnostic");
```

### Pattern 3: Count in Slice (16 sites)

Use to check "did it fire exactly N times?":

```rust
let overflow_count = result
    .diagnostics
    .iter()
    .filter(|d| d.code == DiagCode::GstateStackOverflow)
    .count();
assert_eq!(overflow_count, 1, "Overflow diagnostic should appear exactly once per page");
```

## Reusable Helper Module (Recommended)

**Path:** `crates/pdftract-core/tests/xref_helpers.rs`

Import these helpers instead of hand-rolling assertions:

```rust
use pdftract_core::diagnostics::{DiagCode, Diagnostic};

// Presence check
assert_diagnostic(&result.diagnostics, DiagCode::XrefRepaired);

// Exact count
assert_diagnostic_count(&result.diagnostics, DiagCode::StructDepthExceeded, 2);

// Byte-offset range match
assert_diagnostic_in_range(&diagnostics, DiagCode::XrefTableCorrupt, 1000..=5000);

// Absence by severity
assert_no_diagnostic_with_severity(&diagnostics, Severity::Error);

// Non-panicking count
let count = count_diagnostics(&diagnostics, DiagCode::StreamDecodeError);
```

**Why use helpers:**
- Failure messages dump all observed codes for debugging
- Self-tested with `#[cfg(test)] mod tests`
- Used in production tests (`xref_integration_test.rs:309–352`)

## Failure Message Convention

**Never use bare `assert!(cond)` with no message.** Always show what was observed:

```rust
// Preferred (xref_helpers style)
panic!(
    "Expected diagnostic {:?} not found. Got: {:?}",
    code,
    diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()
);

// Alternative (error_recovery style)
assert!(
    actual_count >= min_count,
    "Expected at least {} '{}' diagnostics, found {}. Diagnostics: {:?}",
    min_count, code, actual_count, diagnostics
);
```

## Secondary Pattern: Vec<String> Substring

Only when asserting on `metadata.diagnostics` (the `"CODE: message"` strings):

```rust
fn assert_diagnostic_count_at_least(diagnostics: &[String], code: &str, min_count: usize) {
    let actual_count = diagnostics.iter().filter(|d| d.contains(code)).count();
    assert!(actual_count >= min_count,
        "Expected at least {} '{}' diagnostics, found {}. Diagnostics: {:?}",
        min_count, code, actual_count, diagnostics);
}
```

## What NOT to Do

- **Never** compare string literals: `output.errors[0].code == "STREAM_DECODE_ERROR"`
- **Never** use bare `assert!(condition)` without a message
- **Never** forget to show what was actually observed on failure

## Quick Template Copy-Paste

```rust
// Case 1: Single diagnostic
assert_eq!(diag.code, DiagCode::YOUR_CODE_HERE);

// Case 2: Presence check
assert!(diags.iter().any(|d| d.code == DiagCode::YOUR_CODE_HERE));

// Case 3: Count check
let count = diags.iter().filter(|d| d.code == DiagCode::YOUR_CODE_HERE).count();
assert_eq!(count, EXPECTED_COUNT, "Diagnostic should appear exactly {} times", EXPECTED_COUNT);

// Case 4: Using helpers (recommended)
use pdftract_core::tests::xref_helpers::assert_diagnostic;
assert_diagnostic(&result.diagnostics, DiagCode::YOUR_CODE_HERE);
```
