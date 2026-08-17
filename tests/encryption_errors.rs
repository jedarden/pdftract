//! Encryption error integration tests.
//!
//! Tests encryption-related error cases in the pdftract CLI:
//! - ENCRYPTION_UNSUPPORTED errors for unknown handlers (e.g., Adobe LiveCycle)
//! - Exit code verification (exit 3 for encryption failures)
//! - Diagnostic message verification
//! - Password handling (empty password vs user-supplied)
//!
//! # References
//! - Plan line 258, Failure Mode Taxonomy: ENCRYPTION_UNSUPPORTED
//! - Plan line 732: Input | Encryption-unsupported | /Encrypt dict with unknown handler
//! - Plan line 765: ENCRYPTION_UNSUPPORTED diagnostic code

#![cfg(feature = "decrypt")]

use std::path::PathBuf;
use std::process::Command;

// Import exit code constants from CLI module
use pdftract_cli::hash::{EXIT_ENCRYPTED, EXIT_SUCCESS};

// Import diagnostic types for error message validation
use pdftract_core::diagnostics::{DiagCode, Diagnostic};

// Import encryption error types for testing (now exported from pdftract_core)
use pdftract_core::{DecryptionError, DecryptError};

// ============================================================================
// Test Constants and Fixtures
// ============================================================================

/// Exit code for encryption errors (should match EXIT_ENCRYPTED from CLI)
const EXPECTED_ENCRYPTION_EXIT_CODE: i32 = 3;

/// Expected encrypted fixture files for testing
const ENCRYPTED_FIXTURES: &[&str] = &[
    "livecycle.pdf",
    // Add more fixtures as they become available:
    // "EC-04-rc4-encrypted.pdf",
    // "EC-05-aes128-encrypted.pdf",
    // "EC-06-aes256-encrypted.pdf",
    // "EC-empty-password.pdf",
];

// ============================================================================
// Test Helper Functions
// ============================================================================

/// Get the path to the pdftract binary (cargo build output)
fn pdftract_bin() -> PathBuf {
    // The binary should be built at target/debug/pdftract or target/release/pdftract
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/pdftract");

    // Fall back to release if debug doesn't exist
    if !path.exists() {
        let mut release_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
        release_path.push("../../target/release/pdftract");
        return release_path;
    }

    path
}

/// Path to encrypted fixtures directory
fn encrypted_fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/encrypted")
}

/// Get the path to a specific encrypted fixture
fn encrypted_fixture(name: &str) -> PathBuf {
    encrypted_fixtures_dir().join(name)
}

/// Assert that a command output contains encryption-related diagnostics
fn assert_encryption_diagnostic(output: &std::process::Output, fixture_name: &str) {
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{} {}", stderr, stdout);

    assert!(
        combined.contains("ENCRYPTION_UNSUPPORTED") || combined.contains("Unsupported encryption"),
        "Output for {} should contain ENCRYPTION_UNSUPPORTED diagnostic, got: {}",
        fixture_name,
        combined
    );
}

/// Assert that a command exits with the encryption error code
fn assert_encryption_exit_code(output: &std::process::Output, fixture_name: &str) {
    let exit_code = output.status.code().unwrap_or(0);
    assert_eq!(
        exit_code, EXPECTED_ENCRYPTION_EXIT_CODE,
        "{} should exit with code {} for ENCRYPTION_UNSUPPORTED, got: {}",
        fixture_name, EXPECTED_ENCRYPTION_EXIT_CODE, exit_code
    );
}

// ============================================================================
// Test Modules
// ============================================================================

/// Module for testing unsupported encryption handlers
mod unsupported_handlers {
    use super::*;

    /// Test that ENCRYPTION_UNSUPPORTED is emitted for unknown encryption handlers
    ///
    /// This test verifies that when pdftract encounters an encrypted PDF with an
    /// unknown handler (e.g., Adobe LiveCycle policy server), it:
    /// 1. Emits ENCRYPTION_UNSUPPORTED diagnostic
    /// 2. Exits with code 3
    /// 3. Does not crash or hang
    #[test]
    fn test_encryption_unsupported_livecycle() {
        let bin = pdftract_bin();
        let livecycle_pdf = encrypted_fixture("livecycle.pdf");

        // Ensure binary exists
        assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

        // Ensure fixture exists
        assert!(livecycle_pdf.exists(), "livecycle.pdf fixture not found at {:?}", livecycle_pdf);

        // Run pdftract extract on the encrypted file
        let output = Command::new(&bin)
            .args(["extract", livecycle_pdf.to_str().unwrap()])
            .output()
            .expect("Failed to execute pdftract extract");

        // Verify exit code and diagnostic
        assert_encryption_exit_code(&output, "livecycle.pdf");
        assert_encryption_diagnostic(&output, "livecycle.pdf");

        println!("✓ livecycle.pdf correctly produced ENCRYPTION_UNSUPPORTED");
        println!("Exit code: {:?}", output.status.code());
    }
}

/// Module for testing exit codes for encrypted PDFs
mod exit_codes {
    use super::*;

    /// Test exit code 3 for encrypted PDFs without password
    ///
    /// Verifies that pdftract exits with code 3 when:
    /// - PDF is encrypted with owner password only
    /// - Empty password is attempted
    /// - User did not supply a password via --password
    #[test]
    fn test_exit_code_3_no_password() {
        let bin = pdftract_bin();
        let livecycle_pdf = encrypted_fixture("livecycle.pdf");

        assert!(bin.exists(), "pdftract binary not found at {:?}", bin);
        assert!(livecycle_pdf.exists(), "livecycle.pdf fixture not found");

        let output = Command::new(&bin)
            .args(["extract", livecycle_pdf.to_str().unwrap()])
            .output()
            .expect("Failed to execute pdftract extract");

        assert_encryption_exit_code(&output, "livecycle.pdf");
        println!("✓ Exit code 3 correctly returned for encrypted PDF without password");
    }
}

/// Module for testing password handling
mod password_handling {
    use super::*;

    /// Test that wrong password also produces ENCRYPTION_UNSUPPORTED
    #[test]
    fn test_wrong_password_encryption_unsupported() {
        let bin = pdftract_bin();
        let livecycle_pdf = encrypted_fixture("livecycle.pdf");

        assert!(bin.exists(), "pdftract binary not found at {:?}", bin);
        assert!(livecycle_pdf.exists(), "livecycle.pdf fixture not found");

        // Supply an incorrect password
        let output = Command::new(&bin)
            .args(["extract", "--password", "wrongpassword", livecycle_pdf.to_str().unwrap()])
            .output()
            .expect("Failed to execute pdftract extract with wrong password");

        assert_encryption_exit_code(&output, "livecycle.pdf");
        assert_encryption_diagnostic(&output, "livecycle.pdf");

        println!("✓ Wrong password correctly produces ENCRYPTION_UNSUPPORTED");
    }
}

/// Module for testing encryption error consistency across fixtures
mod consistency {
    use super::*;

    /// Test that encrypted fixtures produce consistent error behavior
    ///
    /// This is a parameterized test that checks multiple encrypted fixtures
    /// for consistent error handling.
    #[test]
    fn test_encryption_error_consistency() {
        let bin = pdftract_bin();
        let fixtures_dir = encrypted_fixtures_dir();

        assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

        for fixture_name in ENCRYPTED_FIXTURES {
            let fixture_path = encrypted_fixture(fixture_name);

            if !fixture_path.exists() {
                println!("Skipping {} (not found)", fixture_name);
                continue;
            }

            let output = Command::new(&bin)
                .args(["extract", fixture_path.to_str().unwrap()])
                .output()
                .expect(&format!("Failed to execute pdftract extract on {}", fixture_name));

            // Should fail with non-zero exit code
            assert!(!output.status.success(),
                "{} should fail extraction (exit code: {:?})",
                fixture_name, output.status.code());

            // Verify exit code and diagnostic
            assert_encryption_exit_code(&output, fixture_name);
            assert_encryption_diagnostic(&output, fixture_name);

            println!("✓ {} correctly handled as encrypted file", fixture_name);
        }
    }
}

// ============================================================================
// Legacy standalone tests (to be migrated into modules above)
// ============================================================================

/// Test that ENCRYPTION_UNSUPPORTED is emitted for unknown encryption handlers
///
/// This test verifies that when pdftract encounters an encrypted PDF with an
/// unknown handler (e.g., Adobe LiveCycle policy server), it:
/// 1. Emits ENCRYPTION_UNSUPPORTED diagnostic
/// 2. Exits with code 3
/// 3. Does not crash or hang
#[test]
#[deprecated = "Use unsupported_handlers::test_encryption_unsupported_livecycle instead"]
fn test_encryption_unsupported_livecycle() {
    let bin = pdftract_bin();
    let livecycle_pdf = encrypted_fixture("livecycle.pdf");

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    // Ensure fixture exists
    assert!(livecycle_pdf.exists(), "livecycle.pdf fixture not found at {:?}", livecycle_pdf);

    // Run pdftract extract on the encrypted file
    let output = Command::new(&bin)
        .args(["extract", livecycle_pdf.to_str().unwrap()])
        .output()
        .expect("Failed to execute pdftract extract");

    // Verify exit code and diagnostic
    assert_encryption_exit_code(&output, "livecycle.pdf");
    assert_encryption_diagnostic(&output, "livecycle.pdf");

    println!("✓ livecycle.pdf correctly produced ENCRYPTION_UNSUPPORTED");
    println!("Exit code: {:?}", output.status.code());
}

/// Test exit code 3 for encrypted PDFs without password
///
/// Verifies that pdftract exits with code 3 when:
/// - PDF is encrypted with owner password only
/// - Empty password is attempted
/// - User did not supply a password via --password
#[test]
#[deprecated = "Use exit_codes::test_exit_code_3_no_password instead"]
fn test_exit_code_3_no_password() {
    let bin = pdftract_bin();
    let livecycle_pdf = encrypted_fixture("livecycle.pdf");

    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);
    assert!(livecycle_pdf.exists(), "livecycle.pdf fixture not found");

    let output = Command::new(&bin)
        .args(["extract", livecycle_pdf.to_str().unwrap()])
        .output()
        .expect("Failed to execute pdftract extract");

    assert_encryption_exit_code(&output, "livecycle.pdf");
    println!("✓ Exit code 3 correctly returned for encrypted PDF without password");
}

/// Test that wrong password also produces ENCRYPTION_UNSUPPORTED
#[test]
#[deprecated = "Use password_handling::test_wrong_password_encryption_unsupported instead"]
fn test_wrong_password_encryption_unsupported() {
    let bin = pdftract_bin();
    let livecycle_pdf = encrypted_fixture("livecycle.pdf");

    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);
    assert!(livecycle_pdf.exists(), "livecycle.pdf fixture not found");

    // Supply an incorrect password
    let output = Command::new(&bin)
        .args(["extract", "--password", "wrongpassword", livecycle_pdf.to_str().unwrap()])
        .output()
        .expect("Failed to execute pdftract extract with wrong password");

    assert_encryption_exit_code(&output, "livecycle.pdf");
    assert_encryption_diagnostic(&output, "livecycle.pdf");

    println!("✓ Wrong password correctly produces ENCRYPTION_UNSUPPORTED");
}

/// Test that encrypted fixtures produce consistent error behavior
///
/// This is a parameterized test that checks multiple encrypted fixtures
/// for consistent error handling.
#[test]
#[deprecated = "Use consistency::test_encryption_error_consistency instead"]
fn test_encryption_error_consistency() {
    let bin = pdftract_bin();
    let fixtures_dir = encrypted_fixtures_dir();

    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    for fixture_name in ENCRYPTED_FIXTURES {
        let fixture_path = encrypted_fixture(fixture_name);

        if !fixture_path.exists() {
            println!("Skipping {} (not found)", fixture_name);
            continue;
        }

        let output = Command::new(&bin)
            .args(["extract", fixture_path.to_str().unwrap()])
            .output()
            .expect(&format!("Failed to execute pdftract extract on {}", fixture_name));

        // Should fail with non-zero exit code
        assert!(!output.status.success(),
            "{} should fail extraction (exit code: {:?})",
            fixture_name, output.status.code());

        // Verify exit code and diagnostic
        assert_encryption_exit_code(&output, fixture_name);
        assert_encryption_diagnostic(&output, fixture_name);

        println!("✓ {} correctly handled as encrypted file", fixture_name);
    }
}
