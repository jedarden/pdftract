# Compiler Warnings in Test Files

## Summary

- **Total test files with warnings:** 50
- **Total warnings in test files:** 100

### Warning Types Distribution

- **unused_imports:** 52
- **unused_variables:** 25
- **unknown:** 17
- **unnecessary_mut:** 4
- **useless_comparison:** 1
- **unused_code:** 1

## Detailed Warnings by File

### crates/pdftract-core/tests/conformance.rs

**Total warnings:** 6

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-core/tests/conformance.rs`

**Code Context:**
```rust
19 | use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused import: `regex::Regex``

**Location:** `crates/pdftract-core/tests/conformance.rs`

**Code Context:**
```rust
22 | use regex::Regex;
```

#### Line unknown: unnecessary_mut

**Message:** `variable does not need to be mutable`

**Location:** `crates/pdftract-core/tests/conformance.rs`

**Code Context:**
```rust
416 |     let mut result = serde_json::json!({
```

#### Line unknown: unnecessary_mut

**Message:** `variable does not need to be mutable`

**Location:** `crates/pdftract-core/tests/conformance.rs`

**Code Context:**
```rust
454 |     let mut result = serde_json::json!({
```

#### Line unknown: unknown

**Message:** `field `min_schema_version` is never read`

**Location:** `crates/pdftract-core/tests/conformance.rs`

**Code Context:**
```rust
33 | struct TestCase {
...
43 |     min_schema_version: Option<String>,
```

#### Line unknown: unknown

**Message:** `fields `version` and `schema_version` are never read`

**Location:** `crates/pdftract-core/tests/conformance.rs`

**Code Context:**
```rust
50 | struct ConformanceSuite {
51 |     version: String,
```

### crates/pdftract-cli/tests/test_encryption_errors.rs

**Total warnings:** 5

#### Line unknown: unused_imports

**Message:** `unused import: `std::error::Error``

**Location:** `crates/pdftract-cli/tests/test_encryption_errors.rs`

**Code Context:**
```rust
22 | use std::error::Error;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused imports: `Write` and `self``

**Location:** `crates/pdftract-cli/tests/test_encryption_errors.rs`

**Code Context:**
```rust
25 | use std::io::{self, Write};
```

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-cli/tests/test_encryption_errors.rs`

**Code Context:**
```rust
26 | use std::path::{Path, PathBuf};
```

#### Line unknown: unused_imports

**Message:** `unused import: `std::time::Duration``

**Location:** `crates/pdftract-cli/tests/test_encryption_errors.rs`

**Code Context:**
```rust
28 | use std::time::Duration;
```

#### Line unknown: unused_imports

**Message:** `unused import: `super::*``

**Location:** `crates/pdftract-cli/tests/test_encryption_errors.rs`

**Code Context:**
```rust
325 |     use super::*;
```

### crates/pdftract-core/tests/TH-03-mcp-no-auth.rs

**Total warnings:** 5

#### Line unknown: unused_imports

**Message:** `unused import: `TcpListener``

**Location:** `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs`

**Code Context:**
```rust
15 | use std::net::{SocketAddr, TcpListener, ToSocketAddrs};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused imports: `ChildStderr` and `ChildStdout``

**Location:** `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs`

**Code Context:**
```rust
16 | use std::process::{Child, ChildStderr, ChildStdout, Command, Stdio};
```

#### Line unknown: unused_variables

**Message:** `unused variable: `attempt``

**Location:** `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs`

**Code Context:**
```rust
512 |     for attempt in 0..10 {
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unknown

**Message:** `struct `TestResult` is never constructed`

**Location:** `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs`

**Code Context:**
```rust
69 | struct TestResult {
= note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unknown

**Message:** `variant `Null` is never constructed`

**Location:** `crates/pdftract-core/tests/TH-03-mcp-no-auth.rs`

**Code Context:**
```rust
79 | enum StdioConfig {
...
83 |     Null,
```

### crates/pdftract-core/tests/hint_stream_integration.rs

**Total warnings:** 5

#### Line unknown: unused_variables

**Message:** `unused variable: `offset``

**Location:** `crates/pdftract-core/tests/hint_stream_integration.rs`

**Code Context:**
```rust
428 |     fn read_range(&self, offset: u64, length: usize) -> std::io::Result<bytes::Bytes> {
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_variables

**Message:** `unused variable: `length``

**Location:** `crates/pdftract-core/tests/hint_stream_integration.rs`

**Code Context:**
```rust
428 |     fn read_range(&self, offset: u64, length: usize) -> std::io::Result<bytes::Bytes> {
```

#### Line unknown: unused_variables

**Message:** `unused variable: `expected_ranges``

**Location:** `crates/pdftract-core/tests/hint_stream_integration.rs`

**Code Context:**
```rust
445 |     let (hint_data, expected_ranges) = create_test_hint_stream(5);
```

#### Line unknown: unknown

**Message:** `struct `MockPrefetchSource` is never constructed`

**Location:** `crates/pdftract-core/tests/hint_stream_integration.rs`

**Code Context:**
```rust
394 | struct MockPrefetchSource {
= note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unknown

**Message:** `associated function `new` is never used`

**Location:** `crates/pdftract-core/tests/hint_stream_integration.rs`

**Code Context:**
```rust
401 | impl MockPrefetchSource {
402 |     /// Create a new mock source with the given hint stream data.
403 |     fn new(hint_stream_data: Vec<u8>) -> Self {
```

### crates/pdftract-cli/tests/conformance.rs

**Total warnings:** 4

#### Line unknown: unused_imports

**Message:** `unused import: `std::collections::HashMap``

**Location:** `crates/pdftract-cli/tests/conformance.rs`

**Code Context:**
```rust
11 | use std::collections::HashMap;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused imports: `PathBuf` and `Path``

**Location:** `crates/pdftract-cli/tests/conformance.rs`

**Code Context:**
```rust
13 | use std::path::{Path, PathBuf};
```

#### Line unknown: unused_variables

**Message:** `unused variable: `feature``

**Location:** `crates/pdftract-cli/tests/conformance.rs`

**Code Context:**
```rust
187 |     let feature = case.get("feature").and_then(|v| v.as_str());
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_variables

**Message:** `unused variable: `fixture``

**Location:** `crates/pdftract-cli/tests/conformance.rs`

**Code Context:**
```rust
263 | fn execute_method(method: &str, fixture: &str, options: &Value) -> Result<Value> {
```

### crates/pdftract-core/tests/encryption_integration_tests.rs

**Total warnings:** 4

#### Line unknown: unused_imports

**Message:** `unused import: `Diagnostic``

**Location:** `crates/pdftract-core/tests/encryption_integration_tests.rs`

**Code Context:**
```rust
12 | use pdftract_core::diagnostics::{DiagCode, Diagnostic};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused imports: `CryptFilterMethod`, `DecryptionError`, `EncryptionInfo`, `FileKeyResult as Aes256FileKeyResult`, `FileKeyResult as Rc4FileKeyResult`, `PasswordValidation`, `aes_256_decrypt`, `decrypt_object`, `decrypt_with_password`, and `validate_user_password``

**Location:** `crates/pdftract-core/tests/encryption_integration_tests.rs`

**Code Context:**
```rust
16 |     aes_256::{aes_256_decrypt, Aes256Decryptor, FileKeyResult as Aes256FileKeyResult},
17 |     decryptor::{decrypt_with_password, DecryptionError, PasswordValidation},
```

#### Line unknown: unused_imports

**Message:** `unused imports: `XrefEntry` and `XrefResolver``

**Location:** `crates/pdftract-core/tests/encryption_integration_tests.rs`

**Code Context:**
```rust
30 | use pdftract_core::parser::xref::{XrefEntry, XrefResolver};
```

#### Line unknown: unknown

**Message:** `method `with_encrypt_dict` is never used`

**Location:** `crates/pdftract-core/tests/encryption_integration_tests.rs`

**Code Context:**
```rust
39 | impl MockResolver {
...
44 |     fn with_encrypt_dict(mut self, dict: PdfDict) -> Self {
```

### crates/pdftract-core/tests/error_recovery_integration.rs

**Total warnings:** 4

#### Line unknown: unused_variables

**Message:** `unused variable: `test_name``

**Location:** `crates/pdftract-core/tests/error_recovery_integration.rs`

**Code Context:**
```rust
50 | fn assert_no_panic<F>(test_name: &str, f: F) -> Result<(), Box<dyn std::any::Any + Send>>
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unknown

**Message:** `fields `expected_pages`, `expected_objects`, and `expected_behavior` are never read`

**Location:** `crates/pdftract-core/tests/error_recovery_integration.rs`

**Code Context:**
```rust
17 | struct ExpectedDiagnostics {
...
21 |     expected_pages: Option<String>,
```

#### Line unknown: unknown

**Message:** `field `description` is never read`

**Location:** `crates/pdftract-core/tests/error_recovery_integration.rs`

**Code Context:**
```rust
29 | struct ExpectedDiagnostic {
...
32 |     description: String,
```

#### Line unknown: unknown

**Message:** `function `assert_diagnostic_count_at_least` is never used`

**Location:** `crates/pdftract-core/tests/error_recovery_integration.rs`

**Code Context:**
```rust
36 | fn assert_diagnostic_count_at_least(diagnostics: &[String], code: &str, min_count: usize) {
```

### crates/pdftract-cli/tests/test_legal_filing.rs

**Total warnings:** 3

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-cli/tests/test_legal_filing.rs`

**Code Context:**
```rust
19 | use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused import: `super::*``

**Location:** `crates/pdftract-cli/tests/test_legal_filing.rs`

**Code Context:**
```rust
584 |     use super::*;
```

#### Line unknown: unused_variables

**Message:** `unused variable: `fixture_dir``

**Location:** `crates/pdftract-cli/tests/test_legal_filing.rs`

**Code Context:**
```rust
352 |     let fixture_dir = fixture_dir();
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-cli/tests/test_contract.rs

**Total warnings:** 3

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-cli/tests/test_contract.rs`

**Code Context:**
```rust
19 | use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused import: `super::*``

**Location:** `crates/pdftract-cli/tests/test_contract.rs`

**Code Context:**
```rust
404 |     use super::*;
```

#### Line unknown: unused_variables

**Message:** `unused variable: `fixture_dir``

**Location:** `crates/pdftract-cli/tests/test_contract.rs`

**Code Context:**
```rust
336 |     let fixture_dir = fixture_dir();
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-cli/tests/test_scientific_paper.rs

**Total warnings:** 3

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-cli/tests/test_scientific_paper.rs`

**Code Context:**
```rust
21 | use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused import: `super::*``

**Location:** `crates/pdftract-cli/tests/test_scientific_paper.rs`

**Code Context:**
```rust
507 |     use super::*;
```

#### Line unknown: unused_variables

**Message:** `unused variable: `fixture_dir``

**Location:** `crates/pdftract-cli/tests/test_scientific_paper.rs`

**Code Context:**
```rust
359 |     let fixture_dir = fixture_dir();
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-cli/tests/test_slide_deck.rs

**Total warnings:** 3

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-cli/tests/test_slide_deck.rs`

**Code Context:**
```rust
19 | use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused import: `super::*``

**Location:** `crates/pdftract-cli/tests/test_slide_deck.rs`

**Code Context:**
```rust
641 |     use super::*;
```

#### Line unknown: unused_variables

**Message:** `unused variable: `fixture_dir``

**Location:** `crates/pdftract-cli/tests/test_slide_deck.rs`

**Code Context:**
```rust
352 |     let fixture_dir = fixture_dir();
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-cli/../../tests/list_pdf_fixtures.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `std::path::Path``

**Location:** `crates/pdftract-cli/../../tests/list_pdf_fixtures.rs`

**Code Context:**
```rust
4 | use std::path::Path;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unnecessary_mut

**Message:** `variable does not need to be mutable`

**Location:** `crates/pdftract-cli/../../tests/list_pdf_fixtures.rs`

**Code Context:**
```rust
14 |         let mut entries = walkdir::WalkDir::new(fixtures_path)
```

### crates/pdftract-cli/tests/single_page_access.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-cli/tests/single_page_access.rs`

**Code Context:**
```rust
13 | use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: useless_comparison

**Message:** `comparison is useless due to type limits`

**Location:** `crates/pdftract-cli/tests/single_page_access.rs`

**Code Context:**
```rust
132 |         span_count >= 0,
= note: `#[warn(unused_comparisons)]` on by default
```

### crates/pdftract-cli/tests/cli_invocation_fixtures.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-cli/tests/cli_invocation_fixtures.rs`

**Code Context:**
```rust
21 | use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused import: `discover_fixtures_in_dir``

**Location:** `crates/pdftract-cli/tests/cli_invocation_fixtures.rs`

**Code Context:**
```rust
23 | use fixture_discovery::{fixtures_root, discover_all_fixtures, discover_fixtures_by_category, discover_fixtures_in_dir, fixture_categ...
```

### crates/pdftract-cli/tests/test_form.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-cli/tests/test_form.rs`

**Code Context:**
```rust
17 | use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_variables

**Message:** `unused variable: `content``

**Location:** `crates/pdftract-cli/tests/test_form.rs`

**Code Context:**
```rust
271 |     let content = fs::read_to_string(profile_path).expect("Failed to read form profile");
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-cli/tests/test_encryption_unsupported.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `pdftract_cli::password``

**Location:** `crates/pdftract-cli/tests/test_encryption_unsupported.rs`

**Code Context:**
```rust
10 | use pdftract_cli::password;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused imports: `DIAGNOSTIC_CATALOG`, `DiagCode`, `DiagInfo`, `Diagnostic`, `DiagnosticsCollector`, `ObjRef`, and `Severity``

**Location:** `crates/pdftract-cli/tests/test_encryption_unsupported.rs`

**Code Context:**
```rust
12 |     DiagCode, DiagInfo, Diagnostic, DiagnosticsCollector, ObjRef, Severity, DIAGNOSTIC_CATALOG,
```

### crates/pdftract-core/tests/test_decoder_debug.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `PdfDict``

**Location:** `crates/pdftract-core/tests/test_decoder_debug.rs`

**Code Context:**
```rust
4 | use pdftract_core::parser::object::{PdfDict, PdfObject};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused import: `normalize_filter_name``

**Location:** `crates/pdftract-core/tests/test_decoder_debug.rs`

**Code Context:**
```rust
6 |     normalize_filter_name, ASCII85Decoder, FlateDecoder, LZWDecoder, StreamDecoder,
```

### crates/pdftract-core/tests/debug_fingerprint_fixtures.rs

**Total warnings:** 2

#### Line unknown: unused_variables

**Message:** `unused variable: `cat1``

**Location:** `crates/pdftract-core/tests/debug_fingerprint_fixtures.rs`

**Code Context:**
```rust
11 |     let (fp1, cat1, pages1, _resolver1) = parse_pdf_file(v1_path).unwrap();
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_variables

**Message:** `unused variable: `cat2``

**Location:** `crates/pdftract-core/tests/debug_fingerprint_fixtures.rs`

**Code Context:**
```rust
20 |     let (fp2, cat2, pages2, _resolver2) = parse_pdf_file(v2_path).unwrap();
```

### crates/pdftract-core/tests/test_cycle_detection.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `std::collections::hash_map::DefaultHasher``

**Location:** `crates/pdftract-core/tests/test_cycle_detection.rs`

**Code Context:**
```rust
131 |     use std::collections::hash_map::DefaultHasher;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused imports: `Hash` and `Hasher``

**Location:** `crates/pdftract-core/tests/test_cycle_detection.rs`

**Code Context:**
```rust
132 |     use std::hash::{Hash, Hasher};
```

### crates/pdftract-core/tests/object_parser_proptest.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused imports: `ObjectParser`, `PdfDict`, `PdfObject`, and `intern``

**Location:** `crates/pdftract-core/tests/object_parser_proptest.rs`

**Code Context:**
```rust
6 | use pdftract_core::parser::object::{intern, ObjectParser, PdfDict, PdfObject};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused import: `proptest::prelude::*``

**Location:** `crates/pdftract-core/tests/object_parser_proptest.rs`

**Code Context:**
```rust
7 | use proptest::prelude::*;
```

### crates/pdftract-core/tests/encoding_recovery.rs

**Total warnings:** 2

#### Line unknown: unused_code

**Message:** `unused doc comment`

**Location:** `crates/pdftract-core/tests/encoding_recovery.rs`

**Code Context:**
```rust
229 | /     /// Overall recovery rate for the entire corpus.
230 | |     ///
231 | |     /// The Phase 2 exit gate requires ≥90% recovery rate on this corpus.
232 | |     /// This is calculated as the weighted average recovery across all fixtures.
```

#### Line unknown: unknown

**Message:** `field `description` is never read`

**Location:** `crates/pdftract-core/tests/encoding_recovery.rs`

**Code Context:**
```rust
17 | struct EncodingFixture {
...
21 |     description: &'static str,
```

### crates/pdftract-core/tests/xref_integration_test.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `PathBuf``

**Location:** `crates/pdftract-core/tests/xref_integration_test.rs`

**Code Context:**
```rust
10 | use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_imports

**Message:** `unused imports: `merge_hybrid`, `parse_traditional_xref`, and `parse_xref_stream``

**Location:** `crates/pdftract-core/tests/xref_integration_test.rs`

**Code Context:**
```rust
16 |     merge_hybrid, parse_traditional_xref, parse_xref_stream, XrefEntry, XrefSection,
```

### crates/pdftract-core/tests/xref_helpers.rs

**Total warnings:** 2

#### Line unknown: unknown

**Message:** `function `assert_no_diagnostic_with_severity` is never used`

**Location:** `crates/pdftract-core/tests/xref_helpers.rs`

**Code Context:**
```rust
105 | pub fn assert_no_diagnostic_with_severity(
= note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unknown

**Message:** `function `count_diagnostics` is never used`

**Location:** `crates/pdftract-core/tests/xref_helpers.rs`

**Code Context:**
```rust
131 | pub fn count_diagnostics(diagnostics: &[Diagnostic], code: DiagCode) -> usize {
```

### crates/pdftract-core/examples/test_fingerprint_debug.rs

**Total warnings:** 2

#### Line unknown: unused_variables

**Message:** `unused variable: `v1_cat``

**Location:** `crates/pdftract-core/examples/test_fingerprint_debug.rs`

**Code Context:**
```rust
8 |     let (v1_fp, v1_cat, v1_pages, _) = parse_pdf_file(v1_path).unwrap();
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_variables

**Message:** `unused variable: `v2_cat``

**Location:** `crates/pdftract-core/examples/test_fingerprint_debug.rs`

**Code Context:**
```rust
9 |     let (v2_fp, v2_cat, v2_pages, _) = parse_pdf_file(v2_path).unwrap();
```

### crates/pdftract-core/tests/ocr_integration.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `std::path::Path``

**Location:** `crates/pdftract-core/tests/ocr_integration.rs`

**Code Context:**
```rust
12 | use std::path::Path;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unknown

**Message:** `function `tesseract_available` is never used`

**Location:** `crates/pdftract-core/tests/ocr_integration.rs`

**Code Context:**
```rust
15 | fn tesseract_available() -> bool {
= note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/http_range_integration.rs

**Total warnings:** 2

#### Line unknown: unused_imports

**Message:** `unused import: `pdftract_core::source::PdfSource``

**Location:** `crates/pdftract-core/tests/http_range_integration.rs`

**Code Context:**
```rust
6 | use pdftract_core::source::PdfSource;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_variables

**Message:** `unused variable: `length``

**Location:** `crates/pdftract-core/tests/http_range_integration.rs`

**Code Context:**
```rust
350 |         let length = 0usize;
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-libpdftract/tests/test_parse.rs

**Total warnings:** 2

#### Line unknown: unused_variables

**Message:** `unused variable: `catalog``

**Location:** `crates/pdftract-libpdftract/tests/test_parse.rs`

**Code Context:**
```rust
7 |         Ok((fingerprint, catalog, pages, resolver)) => {
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

#### Line unknown: unused_variables

**Message:** `unused variable: `resolver``

**Location:** `crates/pdftract-libpdftract/tests/test_parse.rs`

**Code Context:**
```rust
7 |         Ok((fingerprint, catalog, pages, resolver)) => {
```

### crates/pdftract-core/src/font/type3_test_fixtures.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `Ordering``

**Location:** `crates/pdftract-core/src/font/type3_test_fixtures.rs`

**Code Context:**
```rust
8 | use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
```

### crates/pdftract-cli/tests/fixture_discovery.rs

**Total warnings:** 1

#### Line unknown: unnecessary_mut

**Message:** `variable does not need to be mutable`

**Location:** `crates/pdftract-cli/tests/fixture_discovery.rs`

**Code Context:**
```rust
900 |         let mut discovered = discover_all_fixtures();
```

### crates/pdftract-cli/tests/TH-09-inspector-xss.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `Command``

**Location:** `crates/pdftract-cli/tests/TH-09-inspector-xss.rs`

**Code Context:**
```rust
7 | use std::process::{Command, Stdio};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-cli/tests/test_book_chapter.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `super::*``

**Location:** `crates/pdftract-cli/tests/test_book_chapter.rs`

**Code Context:**
```rust
542 |     use super::*;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-cli/tests/test_header_flag.rs

**Total warnings:** 1

#### Line unknown: unused_variables

**Message:** `unused variable: `stderr``

**Location:** `crates/pdftract-cli/tests/test_header_flag.rs`

**Code Context:**
```rust
356 |     let stderr = String::from_utf8_lossy(&output.stderr);
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-cli/tests/multi_output_validation.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `std::path::PathBuf``

**Location:** `crates/pdftract-cli/tests/multi_output_validation.rs`

**Code Context:**
```rust
6 | use std::path::PathBuf;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-cli/tests/TH-08-log-audit.rs

**Total warnings:** 1

#### Line unknown: unused_variables

**Message:** `unused variable: `pdf_str``

**Location:** `crates/pdftract-cli/tests/TH-08-log-audit.rs`

**Code Context:**
```rust
246 |     let pdf_str = String::from_utf8_lossy(&pdf_bytes);
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/test_lzw_debug.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `PdfDict``

**Location:** `crates/pdftract-core/tests/test_lzw_debug.rs`

**Code Context:**
```rust
2 | use pdftract_core::parser::object::{PdfDict, PdfObject};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/examples/test_pages_check.rs

**Total warnings:** 1

#### Line unknown: unused_variables

**Message:** `unused variable: `resolver``

**Location:** `crates/pdftract-core/examples/test_pages_check.rs`

**Code Context:**
```rust
7 |         Ok((fp, cat, pages, resolver)) => {
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/debug_fingerprint.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `std::sync::Arc``

**Location:** `crates/pdftract-core/tests/debug_fingerprint.rs`

**Code Context:**
```rust
87 |     use std::sync::Arc;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/test_sdk_smoke.rs

**Total warnings:** 1

#### Line unknown: unused_variables

**Message:** `unused variable: `text``

**Location:** `crates/pdftract-core/tests/test_sdk_smoke.rs`

**Code Context:**
```rust
91 |     let text = result.unwrap();
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/cjk_encoding.rs

**Total warnings:** 1

#### Line unknown: unknown

**Message:** `fields `name` and `description` are never read`

**Location:** `crates/pdftract-core/tests/cjk_encoding.rs`

**Code Context:**
```rust
17 | struct CjkFixture {
18 |     name: &'static str,
```

### crates/pdftract-core/tests/verify_proptest_catches_bugs.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `Path``

**Location:** `crates/pdftract-core/tests/verify_proptest_catches_bugs.rs`

**Code Context:**
```rust
67 |     use std::path::{Path, PathBuf};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/schema_validate_fixtures.rs

**Total warnings:** 1

#### Line unknown: unused_variables

**Message:** `unused variable: `error``

**Location:** `crates/pdftract-core/tests/schema_validate_fixtures.rs`

**Code Context:**
```rust
100 |         Err(error) => {
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/debug_page_parsing.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `Catalog``

**Location:** `crates/pdftract-core/tests/debug_page_parsing.rs`

**Code Context:**
```rust
4 | use pdftract_core::parser::catalog::{parse_catalog, Catalog};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/json_schema.rs

**Total warnings:** 1

#### Line unknown: unknown

**Message:** `function `format_validation_error` is never used`

**Location:** `crates/pdftract-core/tests/json_schema.rs`

**Code Context:**
```rust
47 | fn format_validation_error(error: &jsonschema::ValidationError) -> String {
= note: `#[warn(dead_code)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/test_page_access.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `anyhow::Result``

**Location:** `crates/pdftract-core/tests/test_page_access.rs`

**Code Context:**
```rust
14 | use anyhow::Result;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/test_helpers/process_guard.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `std::io::ErrorKind``

**Location:** `crates/pdftract-core/tests/test_helpers/process_guard.rs`

**Code Context:**
```rust
219 |                 use std::io::ErrorKind;
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/test_helpers/mod.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused imports: `DEFAULT_PROCESS_PATTERNS`, `OrphanedProcessError`, `OrphanedProcessGuard`, `kill_orphaned_processes`, `kill_processes_matching_patterns`, `verify_no_orphaned_processes`, and `verify_no_processes_matching_patterns``

**Location:** `crates/pdftract-core/tests/test_helpers/mod.rs`

**Code Context:**
```rust
6 |     verify_no_orphaned_processes,
7 |     verify_no_processes_matching_patterns,
```

### crates/pdftract-core/tests/stream_decoder_fixtures.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `PdfDict``

**Location:** `crates/pdftract-core/tests/stream_decoder_fixtures.rs`

**Code Context:**
```rust
8 | use pdftract_core::parser::object::{PdfDict, PdfObject};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/fingerprint_debug_content_edit.rs

**Total warnings:** 1

#### Line unknown: unused_imports

**Message:** `unused import: `PdfSource``

**Location:** `crates/pdftract-core/tests/fingerprint_debug_content_edit.rs`

**Code Context:**
```rust
4 | use pdftract_core::parser::stream::{FileSource, PdfSource as ParserPdfSource};
= note: `#[warn(unused_imports)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/test_type3_integration.rs

**Total warnings:** 1

#### Line unknown: unused_variables

**Message:** `unused variable: `name``

**Location:** `crates/pdftract-core/tests/test_type3_integration.rs`

**Code Context:**
```rust
265 |     for (name, entry) in &glyph_dict {
= note: `#[warn(unused_variables)]` (part of `#[warn(unused)]`) on by default
```

### crates/pdftract-core/tests/memory_guard.rs

**Total warnings:** 1

#### Line unknown: unknown

**Message:** `variant `UnsupportedPlatform` is never constructed`

**Location:** `crates/pdftract-core/tests/memory_guard.rs`

**Code Context:**
```rust
59 | pub enum MemoryGuardError {
60 |     /// Platform does not support memory limits (e.g., Windows).
61 |     UnsupportedPlatform,
```

## Files Without Warnings

The following test directories/files appear to have no compiler warnings:

- ✅ `tests/integration_test.rs`
- ✅ `tests/smoke_test.rs`
- ✅ `tests/test_assertion_methods.rs`
- ✅ `tests/test_extract_content_stream_bytes.rs`
- ✅ `tests/test_helpers.rs`
- ✅ `tests/test_import_path.rs`
- ✅ `tests/test_parse_fixture.rs`
