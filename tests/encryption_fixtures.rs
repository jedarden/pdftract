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
//! use pdftract_tests::encryption_fixtures::*;
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

// ============================================================================
// Assertion Helpers
// ============================================================================

/// Assert that a command output contains encryption-related diagnostics
pub fn assert_encryption_diagnostic(output: &std::process::Output, fixture_name: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{} {}", stderr, stdout);

    assert!(
        combined.contains("ENCRYPTION_UNSUPPORTED")
            || combined.contains("Unsupported encryption")
            || combined.contains("encrypted"),
        "Output for {} should contain encryption-related diagnostic, got: {}",
        fixture_name,
        combined
    );
}

/// Assert that a command exits with the encryption error code
pub fn assert_encryption_exit_code(output: &std::process::Output, fixture_name: &str) {
    let exit_code = output.status.code().unwrap_or(0);
    assert_eq!(
        exit_code, EXPECTED_ENCRYPTION_EXIT_CODE,
        "{} should exit with code {} for ENCRYPTION_UNSUPPORTED, got: {}",
        fixture_name, EXPECTED_ENCRYPTION_EXIT_CODE, exit_code
    );
}

/// Assert that a command exits with success code
pub fn assert_success_exit_code(output: &std::process::Output) {
    assert!(
        output.status.success(),
        "Command should succeed, got exit code: {:?}",
        output.status.code()
    );
}

/// Assert that a command failed with non-zero exit code
pub fn assert_failure_exit_code(output: &std::process::Output) {
    assert!(
        !output.status.success(),
        "Command should fail with non-zero exit code, got: {:?}",
        output.status.code()
    );
}

/// Assert output contains specific message
pub fn assert_output_contains(output: &std::process::Output, message: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{} {}", stderr, stdout);

    assert!(
        combined.contains(message),
        "Output should contain '{}', got: {}",
        message,
        combined
    );
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
}
