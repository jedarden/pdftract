//! Common test fixtures and helper functions for encryption testing
//!
//! This module provides shared utilities for encryption-related tests across
//! the pdftract codebase. It includes:
//! - Test fixture path resolution
//! - Binary path resolution
//! - Common test constants
//! - Helper functions for running pdftract and validating output
//! - Mock data builders for encryption dictionaries
//!
//! # Usage
//!
//! ```rust
//! use crate::encryption_fixtures::*;
//!
//! let bin = pdftract_bin();
//! let fixture = encrypted_fixture("livecycle.pdf");
//! let output = run_pdftract_extract(&bin, &fixture, None);
//! assert_encryption_exit_code(&output, &fixture);
//! ```

use std::path::{Path, PathBuf};
use std::process::Command;
use std::fs;

// Re-export exit code constants from CLI module
pub use pdftract_cli::hash::{EXIT_ENCRYPTED, EXIT_SUCCESS};

// Re-export diagnostic types for error message validation
pub use pdftract_core::diagnostics::{DiagCode, Diagnostic};

// Re-export encryption error types for testing
pub use pdftract_core::encryption::decryptor::DecryptionError;

// ============================================================================
// Test Constants
// ============================================================================

/// Exit code for encryption errors (should match EXIT_ENCRYPTED from CLI)
pub const EXPECTED_ENCRYPTION_EXIT_CODE: i32 = 3;

/// Expected encrypted fixture files for testing
pub const ENCRYPTED_FIXTURES: &[&str] = &[
    "livecycle.pdf",
    "EC-04-rc4-encrypted.pdf",
    "EC-05-aes128-encrypted.pdf",
    "EC-06-aes256-encrypted.pdf",
    "EC-empty-password.pdf",
];

/// Test password for encrypted fixtures (when available)
pub const TEST_PASSWORD: &str = "test123";

/// Wrong password for testing error handling
pub const WRONG_PASSWORD: &str = "wrongpassword";

// ============================================================================
// Path Resolution Functions
// ============================================================================

/// Get the workspace root directory
pub fn workspace_root() -> PathBuf {
    let manifest_dir = std::env::var("CARGO_MANIFEST_DIR")
        .unwrap_or_else(|_| ".".to_string());
    let path = PathBuf::from(manifest_dir);

    // If we're in tests/ directory, parent() is workspace root
    // If we're in crates/pdftract-cli/tests/, go up two levels
    if path.ends_with("tests") {
        path.parent().unwrap().to_path_buf()
    } else if path.ends_with("pdftract-cli") {
        path.parent().unwrap().parent().unwrap().to_path_buf()
    } else if path.ends_with("pdftract-core") {
        path.parent().unwrap().parent().unwrap().to_path_buf()
    } else {
        path
    }
}

/// Get the path to the pdftract binary (cargo build output)
pub fn pdftract_bin() -> PathBuf {
    let mut path = workspace_root();
    path.push("target/debug/pdftract");

    // Fall back to release if debug doesn't exist
    if !path.exists() {
        let mut release_path = workspace_root();
        release_path.push("target/release/pdftract");
        return release_path;
    }

    path
}

/// Path to encrypted fixtures directory
pub fn encrypted_fixtures_dir() -> PathBuf {
    workspace_root().join("tests/fixtures/encrypted")
}

/// Get the path to a specific encrypted fixture
pub fn encrypted_fixture(name: &str) -> PathBuf {
    encrypted_fixtures_dir().join(name)
}

/// Path to general fixtures directory
pub fn fixtures_dir() -> PathBuf {
    workspace_root().join("tests/fixtures")
}

/// Get the path to a general fixture (not encrypted)
pub fn fixture(name: &str) -> PathBuf {
    fixtures_dir().join(name)
}

// ============================================================================
// Test Execution Helpers
// ============================================================================

/// Run pdftract extract on a file with optional password
pub fn run_pdftract_extract(
    bin: &Path,
    pdf_path: &Path,
    password: Option<&str>,
) -> std::process::Output {
    let mut cmd = Command::new(bin);
    cmd.args(["extract", pdf_path.to_str().unwrap()]);

    if let Some(pwd) = password {
        cmd.args(["--password", pwd]);
    }

    cmd.output().expect("Failed to execute pdftract extract")
}

/// Run pdftract extract and return TestExecutionResult
pub fn run_pdftract_extract_test(
    bin: &Path,
    pdf_path: &Path,
    password: Option<&str>,
) -> TestExecutionResult {
    let fixture_name = pdf_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let output = run_pdftract_extract(bin, pdf_path, password);
    TestExecutionResult::with_fixture(output, fixture_name)
}

/// Run pdftract extract with stdin password
pub fn run_pdftract_extract_with_stdin_password(
    bin: &Path,
    pdf_path: &Path,
    password: &str,
) -> std::process::Output {
    Command::new(bin)
        .args(["extract", pdf_path.to_str().unwrap()])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to execute pdftract extract with stdin password")
}

/// Run pdftract extract with stdin password and return TestExecutionResult
pub fn run_pdftract_extract_stdin_test(
    bin: &Path,
    pdf_path: &Path,
    password: &str,
) -> TestExecutionResult {
    let fixture_name = pdf_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("unknown");

    let output = run_pdftract_extract_with_stdin_password(bin, pdf_path, password);
    TestExecutionResult::with_fixture(output, fixture_name)
}

// ============================================================================
// Test Execution Result Struct
// ============================================================================

/// Result of running a CLI command for testing purposes.
///
/// This struct wraps `std::process::Output` and provides convenient
/// assertion methods for validating command execution results in tests.
#[derive(Debug, Clone)]
pub struct TestExecutionResult {
    /// The underlying process output
    pub output: std::process::Output,
    /// Optional fixture name for better error messages
    pub fixture_name: Option<String>,
}

impl TestExecutionResult {
    /// Create a new TestExecutionResult from process output
    pub fn new(output: std::process::Output) -> Self {
        Self {
            output,
            fixture_name: None,
        }
    }

    /// Create a new TestExecutionResult with associated fixture name
    pub fn with_fixture(output: std::process::Output, fixture_name: &str) -> Self {
        Self {
            output,
            fixture_name: Some(fixture_name.to_string()),
        }
    }

    /// Get the exit code (returns None if process terminated by signal)
    pub fn exit_code(&self) -> Option<i32> {
        self.output.status.code()
    }

    /// Get stdout as a String
    pub fn stdout(&self) -> String {
        String::from_utf8_lossy(&self.output.stdout).to_string()
    }

    /// Get stderr as a String
    pub fn stderr(&self) -> String {
        String::from_utf8_lossy(&self.output.stderr).to_string()
    }

    /// Check if the command succeeded (exit code 0)
    pub fn success(&self) -> bool {
        self.output.status.success()
    }

    /// Get combined stdout and stderr
    pub fn combined_output(&self) -> String {
        format!("{} {}", self.stdout(), self.stderr())
    }

    // ========================================================================
    // Assertion Methods
    // ========================================================================

    /// Assert that stderr contains specific text
    pub fn assert_stderr_contains(&self, text: &str) -> &Self {
        let stderr = self.stderr();
        assert!(
            stderr.contains(text),
            "Expected stderr to contain '{}', got: {}",
            text,
            stderr
        );
        self
    }

    /// Assert that stdout contains specific text
    pub fn assert_stdout_contains(&self, text: &str) -> &Self {
        let stdout = self.stdout();
        assert!(
            stdout.contains(text),
            "Expected stdout to contain '{}', got: {}",
            text,
            stdout
        );
        self
    }

    /// Assert that the exit code matches a specific value
    pub fn assert_exit_code(&self, expected: i32) -> &Self {
        let actual = self.exit_code();
        let context = self.fixture_name.as_deref().unwrap_or("command");
        assert_eq!(
            actual,
            Some(expected),
            "Expected {} to exit with code {}, got {:?}",
            context,
            expected,
            actual
        );
        self
    }

    /// Assert that the command succeeded (exit code 0)
    pub fn assert_success(&self) -> &Self {
        let context = self.fixture_name.as_deref().unwrap_or("command");
        assert!(
            self.success(),
            "Expected {} to succeed, got exit code: {:?}",
            context,
            self.exit_code()
        );
        self
    }

    /// Assert that the command failed (non-zero exit code)
    pub fn assert_failure(&self) -> &Self {
        let context = self.fixture_name.as_deref().unwrap_or("command");
        assert!(
            !self.success(),
            "Expected {} to fail, got exit code: {:?}",
            context,
            self.exit_code()
        );
        self
    }

    /// Assert that combined output contains specific text
    pub fn assert_output_contains(&self, text: &str) -> &Self {
        let combined = self.combined_output();
        assert!(
            combined.contains(text),
            "Expected output to contain '{}', got: {}",
            text,
            combined
        );
        self
    }

    // ========================================================================
    // Encryption-Specific Assertion Methods
    // ========================================================================

    /// Assert that this is an unsupported encryption error
    ///
    /// Checks for:
    /// - Exit code 3 (EXPECTED_ENCRYPTION_EXIT_CODE)
    /// - Error message containing "ENCRYPTION_UNSUPPORTED" or "Unsupported encryption"
    pub fn assert_unsupported_encryption(&self) -> &Self {
        let context = self.fixture_name.as_deref().unwrap_or("encrypted PDF");
        let combined = self.combined_output();

        // Check exit code
        self.assert_exit_code(EXPECTED_ENCRYPTION_EXIT_CODE);

        // Check error message
        assert!(
            combined.contains("ENCRYPTION_UNSUPPORTED")
                || combined.contains("Unsupported encryption")
                || combined.contains("unsupported encryption handler"),
            "Expected {} to emit ENCRYPTION_UNSUPPORTED diagnostic, got: {}",
            context,
            combined
        );

        self
    }

    /// Assert that a password is required/missing error
    ///
    /// Checks for:
    /// - Exit code 3 (EXPECTED_ENCRYPTION_EXIT_CODE)
    /// - Error message indicating password requirement
    pub fn assert_password_required(&self) -> &Self {
        let context = self.fixture_name.as_deref().unwrap_or("encrypted PDF");
        let combined = self.combined_output();

        // Check exit code
        self.assert_exit_code(EXPECTED_ENCRYPTION_EXIT_CODE);

        // Check for password-related error message
        assert!(
            combined.contains("password")
                || combined.contains("Password")
                || combined.contains("PASSWORD_REQUIRED"),
            "Expected {} to emit password-required diagnostic, got: {}",
            context,
            combined
        );

        self
    }

    /// Assert that the provided password was wrong
    ///
    /// Checks for:
    /// - Exit code 3 (EXPECTED_ENCRYPTION_EXIT_CODE)
    /// - Error message indicating wrong/invalid password
    pub fn assert_wrong_password(&self) -> &Self {
        let context = self.fixture_name.as_deref().unwrap_or("encrypted PDF");
        let combined = self.combined_output();

        // Check exit code
        self.assert_exit_code(EXPECTED_ENCRYPTION_EXIT_CODE);

        // Check for wrong password error message
        assert!(
            combined.contains("wrong password")
                || combined.contains("incorrect password")
                || combined.contains("invalid password")
                || combined.contains("Password incorrect"),
            "Expected {} to emit wrong-password diagnostic, got: {}",
            context,
            combined
        );

        self
    }

    /// Assert that the command produced encryption-related diagnostics
    ///
    /// This is a more general check that looks for any encryption-related
    /// error messages in the output.
    pub fn assert_encryption_diagnostic(&self) -> &Self {
        let context = self.fixture_name.as_deref().unwrap_or("encrypted PDF");
        let combined = self.combined_output();

        assert!(
            combined.contains("ENCRYPTION_UNSUPPORTED")
                || combined.contains("Unsupported encryption")
                || combined.contains("encrypted")
                || combined.contains("ENCRYPTION"),
            "Expected {} to contain encryption-related diagnostic, got: {}",
            context,
            combined
        );

        self
    }

    /// Assert that the output is empty (no extraction occurred)
    ///
    /// Useful for verifying that encrypted PDFs don't leak content
    pub fn assert_empty_output(&self) -> &Self {
        let stdout = self.stdout();
        let context = self.fixture_name.as_deref().unwrap_or("command");

        assert!(
            stdout.is_empty() || !stdout.contains("\"pages\""),
            "Expected {} to produce no extraction output, got: {}",
            context,
            stdout
        );

        self
    }
}

impl From<std::process::Output> for TestExecutionResult {
    fn from(output: std::process::Output) -> Self {
        Self::new(output)
    }
}

// ============================================================================
// Legacy Assertion Helpers (backwards compatibility)
// ============================================================================

/// Assert that a command output contains encryption-related diagnostics
pub fn assert_encryption_diagnostic(output: &std::process::Output, fixture_name: &str) {
    let result = TestExecutionResult::with_fixture(output.clone(), fixture_name);
    result.assert_encryption_diagnostic();
}

/// Assert that a command exits with the encryption error code
pub fn assert_encryption_exit_code(output: &std::process::Output, fixture_name: &str) {
    let result = TestExecutionResult::with_fixture(output.clone(), fixture_name);
    result.assert_exit_code(EXPECTED_ENCRYPTION_EXIT_CODE);
}

/// Assert that a command exits with success code
pub fn assert_success_exit_code(output: &std::process::Output) {
    let result = TestExecutionResult::new(output.clone());
    result.assert_success();
}

/// Assert that a command failed with non-zero exit code
pub fn assert_failure_exit_code(output: &std::process::Output) {
    let result = TestExecutionResult::new(output.clone());
    result.assert_failure();
}

/// Assert output contains specific message
pub fn assert_output_contains(output: &std::process::Output, message: &str) {
    let result = TestExecutionResult::new(output.clone());
    result.assert_output_contains(message);
}

// ============================================================================
// Fixture Validation Helpers
// ============================================================================

/// Verify a fixture file exists
pub fn assert_fixture_exists(fixture_path: &Path) {
    assert!(
        fixture_path.exists(),
        "Fixture not found: {}",
        fixture_path.display()
    );
}

/// Verify a fixture file is a valid PDF (basic structure check)
pub fn assert_valid_pdf_structure(fixture_path: &Path) {
    let content = fs::read(fixture_path)
        .expect(&format!("Failed to read fixture: {}", fixture_path.display()));

    // Check for PDF magic number (%PDF-)
    assert!(
        content.starts_with(b"%PDF-"),
        "Fixture {} is not a valid PDF (missing %PDF- header)",
        fixture_path.display()
    );

    // Check for EOF marker
    let content_str = String::from_utf8_lossy(&content);
    assert!(
        content_str.contains("%%EOF"),
        "Fixture {} is not a valid PDF (missing %%EOF marker)",
        fixture_path.display()
    );
}

/// Verify all encrypted fixture files exist
pub fn assert_encrypted_fixtures_exist() {
    let fixture_dir = encrypted_fixtures_dir();
    assert!(
        fixture_dir.exists(),
        "Encrypted fixtures directory not found: {}",
        fixture_dir.display()
    );

    for fixture_name in ENCRYPTED_FIXTURES {
        let path = encrypted_fixture(fixture_name);
        if !path.exists() {
            println!("Skipping {} (not found)", fixture_name);
        }
    }
}

// ============================================================================
// Mock Data Builders (for core encryption tests)
// ============================================================================

#[cfg(feature = "decrypt")]
use pdftract_core::parser::object::{PdfDict, PdfObject};

/// Create a PdfDict from a vector of key-value pairs
#[cfg(feature = "decrypt")]
pub fn make_dict(entries: Vec<(&str, PdfObject)>) -> PdfDict {
    entries.into_iter().map(|(k, v)| (k.into(), v)).collect()
}

/// Create a mock trailer dictionary with encryption
#[cfg(feature = "decrypt")]
pub fn make_trailer(encrypt_dict: PdfDict, id: Option<Vec<u8>>) -> PdfDict {
    let mut trailer = make_dict(vec![
        ("/Root", PdfObject::Ref(pdftract_core::parser::object::ObjRef::new(1, 0))),
        ("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict))),
    ]);

    if let Some(id_bytes) = id {
        trailer.insert(
            "/ID".into(),
            PdfObject::Array(Box::new(vec![PdfObject::String(Box::new(id_bytes))]))
        );
    }

    trailer
}

/// Create a mock RC4-40 encryption dictionary (V=1, R=2)
#[cfg(feature = "decrypt")]
pub fn make_rc4_encryption_dict() -> PdfDict {
    make_dict(vec![
        ("/Filter", PdfObject::Name("Standard".into())),
        ("/V", PdfObject::Integer(1)),
        ("/R", PdfObject::Integer(2)),
        ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
    ])
}

/// Create a mock AES-128 encryption dictionary (V=4, R=4)
#[cfg(feature = "decrypt")]
pub fn make_aes128_encryption_dict() -> PdfDict {
    make_dict(vec![
        ("/Filter", PdfObject::Name("Standard".into())),
        ("/V", PdfObject::Integer(4)),
        ("/R", PdfObject::Integer(4)),
        ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ("/StmF", PdfObject::Name("/Identity".into())),
        ("/StrF", PdfObject::Name("/Identity".into())),
    ])
}

/// Create a mock AES-256 encryption dictionary (V=5, R=6)
#[cfg(feature = "decrypt")]
pub fn make_aes256_encryption_dict() -> PdfDict {
    make_dict(vec![
        ("/Filter", PdfObject::Name("Standard".into())),
        ("/V", PdfObject::Integer(5)),
        ("/R", PdfObject::Integer(6)),
        ("/O", PdfObject::String(Box::new(vec![0u8; 48]))),
        ("/U", PdfObject::String(Box::new(vec![0u8; 48]))),
        ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ("/UE", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/OE", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/Perms", PdfObject::String(Box::new({
            let mut perms = [0u8; 16];
            perms[0..4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
            perms.to_vec()
        }))),
    ])
}

/// Create a mock unsupported encryption dictionary
#[cfg(feature = "decrypt")]
pub fn make_unsupported_encryption_dict(filter_name: &str) -> PdfDict {
    make_dict(vec![
        ("/Filter", PdfObject::Name(filter_name.into())),
        ("/V", PdfObject::Integer(1)),
        ("/R", PdfObject::Integer(2)),
    ])
}

// ============================================================================
// Test Suite Builders
// ============================================================================

/// Macro to generate parameterized tests for multiple fixtures
#[macro_export]
macro_rules! test_all_fixtures {
    ($test_name:ident, $test_body:expr) => {
        pub fn $test_name() {
            let bin = pdftract_bin();
            for fixture_name in ENCRYPTED_FIXTURES {
                let fixture_path = encrypted_fixture(fixture_name);

                if !fixture_path.exists() {
                    println!("Skipping {} (not found)", fixture_name);
                    continue;
                }

                $test_body(&bin, &fixture_path, fixture_name);
            }
        }
    };
}

// ============================================================================
// Module Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_workspace_root_resolution() {
        let root = workspace_root();
        assert!(root.exists(), "Workspace root should exist: {}", root.display());
    }

    #[test]
    fn test_encrypted_fixtures_dir_exists() {
        let dir = encrypted_fixtures_dir();
        assert!(dir.exists(), "Encrypted fixtures directory should exist: {}", dir.display());
    }

    #[test]
    fn test_pdftract_bin_path() {
        let bin = pdftract_bin();
        // Don't assert existence here since binary might not be built yet
        assert!(bin.ends_with("pdftract"), "Binary path should end with 'pdftract'");
    }

    #[test]
    fn test_livecycle_fixture_path() {
        let fixture = encrypted_fixture("livecycle.pdf");
        assert!(
            fixture.ends_with("tests/fixtures/encrypted/livecycle.pdf"),
            "Fixture path should resolve correctly: {}",
            fixture.display()
        );
    }

    #[test]
    fn test_constants_defined() {
        assert_eq!(EXPECTED_ENCRYPTION_EXIT_CODE, 3);
        assert!(!ENCRYPTED_FIXTURES.is_empty());
        assert!(!TEST_PASSWORD.is_empty());
        assert!(!WRONG_PASSWORD.is_empty());
    }

    #[test]
    #[cfg(feature = "decrypt")]
    fn test_make_dict_creates_valid_dict() {
        let dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(1)),
        ]);

        assert!(dict.contains_key("/Filter"));
        assert!(dict.contains_key("/V"));
    }

    #[test]
    #[cfg(feature = "decrypt")]
    fn test_rc4_encryption_dict_builder() {
        let dict = make_rc4_encryption_dict();
        assert!(dict.contains_key("/Filter"));
        assert!(dict.contains_key("/V"));
        assert!(dict.contains_key("/R"));
    }

    #[test]
    #[cfg(feature = "decrypt")]
    fn test_aes128_encryption_dict_builder() {
        let dict = make_aes128_encryption_dict();
        assert!(dict.contains_key("/Filter"));
        assert!(dict.contains_key("/V"));
        assert!(dict.contains_key("/StmF"));
    }

    #[test]
    #[cfg(feature = "decrypt")]
    fn test_aes256_encryption_dict_builder() {
        let dict = make_aes256_encryption_dict();
        assert!(dict.contains_key("/Filter"));
        assert!(dict.contains_key("/V"));
        assert!(dict.contains_key("/UE"));
        assert!(dict.contains_key("/OE"));
    }

    // ========================================================================
    // TestExecutionResult Tests
    // ========================================================================

    #[test]
    fn test_execution_result_creation() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"test output".to_vec(),
            stderr: b"test error".to_vec(),
        };

        let result = TestExecutionResult::new(output.clone());
        assert_eq!(result.exit_code(), Some(0));
        assert_eq!(result.stdout(), "test output");
        assert_eq!(result.stderr(), "test error");
        assert!(result.success());
    }

    #[test]
    fn test_execution_result_with_fixture() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(3),
            stdout: b"".to_vec(),
            stderr: b"Unsupported encryption".to_vec(),
        };

        let result = TestExecutionResult::with_fixture(output, "test.pdf");
        assert_eq!(result.exit_code(), Some(3));
        assert_eq!(result.fixture_name, Some("test.pdf".to_string()));
    }

    #[test]
    fn test_execution_result_assert_exit_code() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(3),
            stdout: b"".to_vec(),
            stderr: b"error".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        result.assert_exit_code(3); // Should not panic
    }

    #[test]
    #[should_panic(expected = "Expected command to exit with code 0")]
    fn test_execution_result_assert_exit_code_failure() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(3),
            stdout: b"".to_vec(),
            stderr: b"error".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        result.assert_exit_code(0); // Should panic
    }

    #[test]
    fn test_execution_result_assert_success() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"success".to_vec(),
            stderr: b"".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        result.assert_success(); // Should not panic
    }

    #[test]
    #[should_panic(expected = "Expected command to succeed")]
    fn test_execution_result_assert_success_failure() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(1),
            stdout: b"error".to_vec(),
            stderr: b"error".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        result.assert_success(); // Should panic
    }

    #[test]
    fn test_execution_result_assert_failure() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(1),
            stdout: b"error".to_vec(),
            stderr: b"error".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        result.assert_failure(); // Should not panic
    }

    #[test]
    fn test_execution_result_assert_stderr_contains() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(1),
            stdout: b"".to_vec(),
            stderr: b"Unsupported encryption handler".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        result.assert_stderr_contains("Unsupported encryption"); // Should not panic
    }

    #[test]
    #[should_panic(expected = "Expected stderr to contain")]
    fn test_execution_result_assert_stderr_contains_failure() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(1),
            stdout: b"".to_vec(),
            stderr: b"some error".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        result.assert_stderr_contains("encryption"); // Should panic
    }

    #[test]
    fn test_execution_result_assert_stdout_contains() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"test output".to_vec(),
            stderr: b"".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        result.assert_stdout_contains("test"); // Should not panic
    }

    #[test]
    fn test_execution_result_assert_output_contains() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(1),
            stdout: b"stdout message".to_vec(),
            stderr: b"stderr message".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        result.assert_output_contains("stdout message"); // Should not panic
        result.assert_output_contains("stderr message"); // Should not panic
    }

    #[test]
    fn test_execution_result_combined_output() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"stdout".to_vec(),
            stderr: b"stderr".to_vec(),
        };

        let result = TestExecutionResult::new(output);
        let combined = result.combined_output();
        assert!(combined.contains("stdout"));
        assert!(combined.contains("stderr"));
    }

    #[test]
    fn test_execution_result_assert_unsupported_encryption() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(3),
            stdout: b"".to_vec(),
            stderr: b"Unsupported encryption handler Adobe.APS".to_vec(),
        };

        let result = TestExecutionResult::with_fixture(output, "test.pdf");
        result.assert_unsupported_encryption(); // Should not panic
    }

    #[test]
    fn test_execution_result_assert_password_required() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(3),
            stdout: b"".to_vec(),
            stderr: b"password required".to_vec(),
        };

        let result = TestExecutionResult::with_fixture(output, "encrypted.pdf");
        result.assert_password_required(); // Should not panic
    }

    #[test]
    fn test_execution_result_assert_wrong_password() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(3),
            stdout: b"".to_vec(),
            stderr: b"incorrect password".to_vec(),
        };

        let result = TestExecutionResult::with_fixture(output, "encrypted.pdf");
        result.assert_wrong_password(); // Should not panic
    }

    #[test]
    fn test_execution_result_assert_empty_output() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(3),
            stdout: b"".to_vec(),
            stderr: b"error".to_vec(),
        };

        let result = TestExecutionResult::with_fixture(output, "encrypted.pdf");
        result.assert_empty_output(); // Should not panic
    }

    #[test]
    #[should_panic(expected = "Expected no extraction output")]
    fn test_execution_result_assert_empty_output_failure() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: br#"{"pages": []}"#.to_vec(),
            stderr: b"".to_vec(),
        };

        let result = TestExecutionResult::with_fixture(output, "test.pdf");
        result.assert_empty_output(); // Should panic
    }

    #[test]
    fn test_execution_result_method_chaining() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(3),
            stdout: b"".to_vec(),
            stderr: b"Unsupported encryption".to_vec(),
        };

        let result = TestExecutionResult::with_fixture(output, "test.pdf");

        // Test method chaining
        result
            .assert_failure()
            .assert_exit_code(3)
            .assert_stderr_contains("Unsupported")
            .assert_empty_output();
    }

    #[test]
    fn test_execution_result_from_impl() {
        use std::process::Output;

        let output = Output {
            status: std::process::ExitStatus::from_raw(0),
            stdout: b"test".to_vec(),
            stderr: b"".to_vec(),
        };

        let result: TestExecutionResult = output.into();
        assert_eq!(result.exit_code(), Some(0));
        assert_eq!(result.stdout(), "test");
    }
}
