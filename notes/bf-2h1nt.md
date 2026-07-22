# bf-2h1nt — Existing error-assertion patterns in the test suite

**Type:** explore · **Parent:** [[bf-mzf4i]] (umbrella: add STREAM_DECODE_ERROR assertion to truncated-flate)
**Purpose:** Before writing a new error assertion, survey how the existing test suite already asserts
on errors/diagnostics arrays so the new assertion matches an established pattern. This note is the
*pattern catalog*; the errors-array *format* lives in `docs/errors-array-format.md` and the
truncated-flate-specific implementation guide in [[bf-4fb3b]].
**Re-verified against current source:** 2026-07-22

---

## 0. TL;DR — the one pattern to follow

The codebase has **one overwhelmingly dominant idiom family**: compare the **`DiagCode` enum
variant** (never a string literal) over a `Diagnostic`/`&[Diagnostic]`. Verified frequency across
`crates/`:

| Idiom | Sites | When to use |
|---|---|---|
| `assert_eq!(diag.code, DiagCode::X)` (single value / `[0]`) | **51** | exactly-one-diagnostic case (e.g. after `unwrap_err()`) |
| `diags.iter().any(\|d\| d.code == DiagCode::X)` | **56** | "did this code fire at all?" (presence, slice) |
| `diags.iter().any(\|d\| matches!(d.code, DiagCode::X))` | **11** | same as above; `matches!` is an accepted equivalent |
| `diags.iter().filter(\|d\| d.code == DiagCode::X).count()` | **16** | "did it fire exactly N times?" (count) |
| `diags.iter().filter(\|d\| d.contains("CODE"))` on `Vec<String>` | 9 | only when you hold `metadata.diagnostics: Vec<String>` |
| `output.errors[0].code == "STRING"` on JSON | **1** | synthetic schema self-test only (`schema/mod.rs:2740`, code `"TEST_WARNING"`) — not used by any real extraction test |

**Use the enum.** The `==` and `matches!` forms are interchangeable house style for presence;
pick whichever reads better (test files lean slightly toward `matches!`, src tests toward `==`).
For the single-diagnostic case, `assert_eq!(diag.code, DiagCode::X)` (51 sites) is preferred over
a hand-rolled `assert!(diag.code == …)`. Prefer the reusable helpers in
`crates/pdftract-core/tests/xref_helpers.rs` (§2, which *is* used in production tests — §3) over
hand-rolled asserts. The string-literal `.code == "..."` form seen in some design notes is
*derived from the JSON schema*; the only `output.errors[…]code == "STRING"` in the tree is a
synthetic schema self-test (`schema/mod.rs:2740`, `"TEST_WARNING"`) — real extraction tests never
assert on the JSON string surface.

---

## 1. The three error representations (pick the right one before asserting)

Asserting the wrong representation is the #1 way to land a vacuous test (see [[bf-4fb3b]] §0).
Each lives on a different surface and takes a different assertion idiom:

| Representation | Type | Where populated | Field compared | Idiom |
|---|---|---|---|---|
| **Per-layer `Vec<Diagnostic>`** (internal) | `Diagnostic { code: DiagCode, … }` (`src/diagnostics.rs:2385,2387`) | emitted by `emit!()` wherever a layer parses | **`DiagCode` enum** | **enum `.any`/`.filter` — the dominant pattern** |
| `ExtractionMetadata.diagnostics` | `Vec<String>` (`src/extract.rs:416`), each `"CODE: message"` | aggregated onto the full-extraction result | `&str` substring | `.filter(\|d\| d.contains("CODE"))` |
| JSON `Output.errors` | `Vec<DiagnosticJson>` (`src/schema/mod.rs:1539`), `code: String` | built by `result_to_output()` for JSON consumers | `String` | `.any(\|e\| e.code == "STRING")` — but **no existing test uses this** |

> Key fact for [[bf-mzf4i]]: `extract_pdf`/`materialize_pages` returns the **2nd** representation
> for `truncated-flate.pdf` and it is **empty** on that path (the truncated page is not
> enumerable). The code in play is `DiagCode::StreamDecodeError` (enum,
> `src/diagnostics.rs:465`) → string `"STREAM_DECODE_ERROR"` (`:1278`). Full format/line refs in
> `docs/errors-array-format.md` and [[bf-4fb3b]] §1/§6.

---

## 2. The canonical helper module — `crates/pdftract-core/tests/xref_helpers.rs` ⭐

**This is the recommended pattern to follow.** A self-contained, unit-tested module of reusable
diagnostic-assertion helpers, all taking `&[Diagnostic]` + a `DiagCode`. Import and call rather
than re-inventing:

```rust
use pdftract_core::diagnostics::{DiagCode, Diagnostic};

// Presence (§2.1)
pub fn assert_diagnostic(diagnostics: &[Diagnostic], code: DiagCode) {
    let found = diagnostics.iter().any(|d| d.code == code);          // xref_helpers.rs:18
    if !found {
        panic!(
            "Expected diagnostic {:?} not found. Got: {:?}",
            code,
            diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()   // show ALL observed codes
        );
    }
}

// Exact count (§2.2)
pub fn assert_diagnostic_count(diagnostics: &[Diagnostic], code: DiagCode, count: usize) {
    let actual = diagnostics.iter().filter(|d| d.code == code).count(); // :88
    if actual != count {
        panic!("Expected diagnostic {:?} to appear {} times, but found {} times",
               code, count, actual);
    }
}

// Byte-offset range match (§2.3)
pub fn assert_diagnostic_in_range(diagnostics: &[Diagnostic], code: DiagCode,
                                  byte_offset_range: RangeInclusive<u64>);  // :40

// Absence by severity (§2.4)
pub fn assert_no_diagnostic_with_severity(diagnostics: &[Diagnostic], severity: Severity); // :105

// Non-panicking count (for the assert!-side of a check)
pub fn count_diagnostics(diagnostics: &[Diagnostic], code: DiagCode) -> usize;  // :131
```

**Why this is the pattern to copy:**
- Enum comparison, not strings (matches the 51+56+16 dominant sites).
- **Failure messages dump every observed code** via `diagnostics.iter().map(|d| d.code).collect::<Vec<_>>()` — the single most useful failure-message idiom in the suite; copy it.
- Self-tested: the module's own `#[cfg(test)] mod tests` (`:135–203`) pins each helper's pass + `#[should_panic]` behavior, so the contract is documented in code.
- **Actually used in production tests**, not just defined — `xref_integration_test.rs:309–352`
  calls it three times: `assert_diagnostic(&result.diagnostics, DiagCode::XrefRepaired)`
  (`:310`), `…StructDepthExceeded` (`:331`), `…StructCircularRef` (`:352`).

---

## 3. The dominant inline idiom — enum comparison on `&[Diagnostic]`

When a test asserts inline (most do), the house style is enum `.any`/`.filter` directly on the
`diagnostics` slice a parsing function returns. Two real excerpts:

**Presence** (`crates/pdftract-core/src/render/pdfium_path.rs:327`):
```rust
assert!(diags.iter().any(|d| d.code == DiagCode::StructMissingKey));
```

**Count** (`crates/pdftract-core/src/content_stream.rs:2906–2922`) — filter → count → `assert_eq!`:
```rust
// Count overflow diagnostics
let overflow_count = result
    .diagnostics
    .iter()
    .filter(|d| d.code == DiagCode::GstateStackOverflow)
    .count();
assert_eq!(overflow_count, 1, "Overflow diagnostic should be emitted exactly once per page");
```

**Direct equality on a single diagnostic (51 sites — the most common single form).** Use this
when you expect exactly one diagnostic, typically after `unwrap_err()` or indexing `[0]`. Two
flavors, both `assert_eq!` (no hand-rolled `assert!(… == …)`):

`tests/test_cycle_detection.rs:36` (from a `Result::Err`):
```rust
let diag = result.unwrap_err();
assert_eq!(diag.code, DiagCode::StructCircularRef);
```
`src/font/resolver.rs:931–932` (from a `Vec<Diagnostic>`, asserting length first):
```rust
assert_eq!(diagnostics.len(), 1);
assert_eq!(diagnostics[0].code, DiagCode::FontGlyphUnmapped);
```
Other call-sites: `test_cycle_detection.rs:69,211,322`, `TH-05-ssrf-block.rs:321`
(`RemoteUrlPrivateNetwork`), `encryption_integration_tests.rs:207` (`EncryptionUnsupported`).

**`matches!` variant (11 sites).** Semantically identical to `d.code == DiagCode::X` for a single
variant; favored in the `remote_*` integration tests. `tests/remote_forward_scan_disable.rs:97–104`:
```rust
let has_remote_diagnostic = result
    .diagnostics
    .iter()
    .any(|d| matches!(d.code, DiagCode::XrefRemoteNoForwardScan));
assert!(has_remote_diagnostic, "Expected XREF_REMOTE_NO_FORWARD_SCAN diagnostic for remote source");
```
Also in `remote_mock_server_tests.rs:424` / `remote_fetch_sequence.rs:627` / `remote_integration.rs:227`.

**Compound predicate on a single value** (`crates/pdftract-core/tests/test_cycle_detection.rs:329–334`)
— an `Err(diag) =>` match arm comparing `.code` directly (no iterator), with a minimal message:
```rust
assert!(
    diag.code == DiagCode::StructCircularRef
        || diag.code == DiagCode::StructDepthExceeded,
    "Unexpected error code: {:?}",
    diag.code
);
```

> `Diagnostic.code` is the `DiagCode` enum (`src/diagnostics.rs:2387`), and `DiagCode` derives
> `PartialEq`, so `d.code == DiagCode::StreamDecodeError` compiles and is the correct form. The
> enum also gives `.severity()` (`:2443`), `.byte_offset`, `.message` if you need them.

---

## 4. Secondary idioms (only when you hold that exact type)

**`Vec<String>` substring** — when asserting on `metadata.diagnostics` (the `"CODE: message"`
strings), e.g. `error_recovery_integration.rs:37`:
```rust
fn assert_diagnostic_count_at_least(diagnostics: &[String], code: &str, min_count: usize) {
    let actual_count = diagnostics.iter().filter(|d| d.contains(code)).count();
    assert!(actual_count >= min_count,
        "Expected at least {} '{}' diagnostics, found {}. Diagnostics: {:?}",
        min_count, code, actual_count, diagnostics);   // dumps the full slice
}
```
The fixture-driven sibling `test_truncated_mid_stream` (`error_recovery_integration.rs:175–184`)
filters expected-diagnostic metadata and `assert!(!…is_empty(), "Should expect STREAM_DECODE_ERROR diagnostic")` — a **metadata-contract** check, not a runtime emission check.

**Message-substring** — when the *code* is stable but you want the human text:
`struct_tree.rs:1875` → `diags.iter().any(|d| d.message.contains("cycle"))`;
`table/cell.rs:1896` → `diags.iter().any(|d| d.contains("merged_cells"))` (here `d` is a `String`).

**MCP/JSON result code** (different surface — `ToolCallResult`, not extraction diagnostics):
`TH-05-ssrf-block.rs:973` defines `has_error_code(&self, expected_code: &str) -> bool` that reads
`data.code` from a JSON error payload; `mcp-tools-integration.rs:121–129,240–247` asserts on
`tools::ERROR_*` / `tools::CODE_*` constants. Listed for completeness; **not** the
extraction-diagnostic pattern.

**CLI stderr string-contains** (`crates/pdftract-cli/tests/`): when a CLI test only has captured
stdout/stderr, assert the human text *or* the code string appear —
`test_encryption_errors.rs:120` / `test_encryption_unsupported.rs:43`:
```rust
assert!(
    stderr.contains("Unsupported encryption") || stderr.contains("ENCRYPTION_UNSUPPORTED"),
    "Expected stderr to contain 'Unsupported encryption' or 'ENCRYPTION_UNSUPPORTED', got: {}",
    stderr
);
```
JSON-RPC layer (`mcp-http.rs:224`) uses numeric codes: `assert_eq!(json["error"]["code"], -32002)`.

---

## 5. Failure-message conventions (the three styles in use)

1. **xref_helpers style (preferred):** lead with `Expected diagnostic {:?} not found. Got: {:?}`,
   then `collect()` of every observed `d.code`. (`xref_helpers.rs:21,52,91`)
2. **error_recovery style:** `"Expected at least {} '{}' diagnostics, found {}. Diagnostics: {:?}"`,
   then the full `&[String]`. (`error_recovery_integration.rs:42–45`)
3. **bf-300b5 narrative template** (used in the unmapped-glyph migration, `font/*.rs`): four-line
   block — `<Description>` / `Expected: …` / `Found: …` / `Why this matters: …` (cataloged in
   `notes/bf-1gd0b-catalog.md`). Heavier; reserved for high-value explanatory assertions.

Common thread across all three: **never a bare `assert!(cond)` with no message**, and **always show
what was actually observed** (collected codes or the full slice) so a failure is diagnosable.

---

## 6. Recommendation for [[bf-mzf4i]]

Given the truncated-flate fixture only surfaces a signal on the low-level decode path (not on
`Output.errors` — see [[bf-4fb3b]] §0/§2), and the per-fixture `stream_decoder_fixtures.rs` loop
collects a `failures: Vec<String>` and panics once at the end:

- **Match the existing idiom:** compare the **`DiagCode` enum** (`fixture.expected_diags.contains(&DiagCode::StreamDecodeError)`), not a string literal — this is what 56+16 existing sites do.
- **For a one-off assertion in an aggregated loop**, follow [[bf-4fb3b]] §5: `push` to `failures`
  with a message that names **both** the enum (`DiagCode::StreamDecodeError`) and the string
  (`STREAM_DECODE_ERROR`), states expected-vs-observed, and uses a byte **count** (not raw bytes).
- **For a standalone diagnostic test**, reach for `xref_helpers::assert_diagnostic` (§2) first.

---

## 7. Acceptance criteria status

- [x] **Existing error assertion patterns identified** — §0 tally (56 any / 16 filter-count / 9
  Vec<String> contains / 0 JSON string-eq) + §2 helper module + §3–§4 idioms.
- [x] **Clear pattern to follow documented** — §0 "use the enum", §2 `xref_helpers.rs` as the
  recommended reusable helpers, §6 concrete guidance for the parent bead.
- [x] **Examples of similar assertions found** — §2 (5 helpers), §3 (3 inline excerpts with
  file:line), §4 (4 secondary), §5 (3 message styles).

## 8. References

- Parent: [[bf-mzf4i]] → genesis `pdftract-qkc77`.
- Sibling notes: [[bf-4fb3b]] (truncated-flate impl guide + errors-array format), [[bf-4g6dj]]
  (scaffold/extraction-result exam), `docs/errors-array-format.md`, [[bf-1gd0b-catalog]]
  (bf-300b5 message template).
- Source-of-truth line refs (all verified 2026-07-22):
  - `DiagCode` enum / `StreamDecodeError` / string: `src/diagnostics.rs:139, 465, 1278`
  - `Diagnostic { code: DiagCode }` / `.severity()`: `src/diagnostics.rs:2385, 2387, 2443`
  - Helpers: `tests/xref_helpers.rs:17, 40, 87, 105, 131` (+ self-tests `:135–203`); real
    call-sites `tests/xref_integration_test.rs:309–352`
  - Direct equality `assert_eq!(diag.code, DiagCode::X)` (51): `tests/test_cycle_detection.rs:36,
    69, 211, 322`, `tests/TH-05-ssrf-block.rs:321`, `tests/encryption_integration_tests.rs:207`,
    `src/font/resolver.rs:932`
  - `matches!` presence (11): `tests/remote_forward_scan_disable.rs:97, 161`,
    `tests/remote_mock_server_tests.rs:424, 508`, `tests/remote_fetch_sequence.rs:627`,
    `tests/remote_integration.rs:227`
  - Inline enum presence/count: `src/preprocess.rs` (×13), `src/content_stream.rs:2906–2922`,
    `tests/test_cycle_detection.rs:330`, `src/render/pdfium_path.rs:327`
  - Vec<String> substring: `tests/error_recovery_integration.rs:37, 175–184`
  - Message-substring: `src/parser/struct_tree.rs:1875`, `src/table/cell.rs:1896`
  - JSON surfaces: schema self-test `src/schema/mod.rs:2740` (`output.errors[0].code,
    "TEST_WARNING"`); MCP `tests/TH-05-ssrf-block.rs:973`, `crates/pdftract-cli/tests/
    mcp-tools-integration.rs:121, 240`, `mcp-http.rs:224`; CLI stderr
    `crates/pdftract-cli/tests/test_encryption_errors.rs:120`
