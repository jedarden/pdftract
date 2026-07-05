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

use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

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
    let livecycle_pdf = encrypted_fixtures_dir().join("livecycle.pdf");

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    // Ensure fixture exists
    assert!(livecycle_pdf.exists(), "livecycle.pdf fixture not found at {:?}", livecycle_pdf);

    // Run pdftract extract on the encrypted file
    let output = Command::new(&bin)
        .args(["extract", livecycle_pdf.to_str().unwrap()])
        .output()
        .expect("Failed to execute pdftract extract");

    // Should fail with exit code 3 (encryption failure)
    let exit_code = output.status.code().unwrap_or(0);
    assert_eq!(exit_code, 3,
        "pdftract should exit with code 3 for ENCRYPTION_UNSUPPORTED, got: {}",
        exit_code);

    // Should emit ENCRYPTION_UNSUPPORTED diagnostic
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{} {}", stderr, stdout);

    assert!(combined.contains("ENCRYPTION_UNSUPPORTED"),
        "Output should contain ENCRYPTION_UNSUPPORTED diagnostic, got: {}", combined);

    println!("✓ livecycle.pdf correctly produced ENCRYPTION_UNSUPPORTED");
    println!("Exit code: {}", exit_code);
    println!("Output: {}", combined.trim());
}

/// Test exit code 3 for encrypted PDFs without password
///
/// Verifies that pdftract exits with code 3 when:
/// - PDF is encrypted with owner password only
/// - Empty password is attempted
/// - User did not supply a password via --password
#[test]
fn test_exit_code_3_no_password() {
    let bin = pdftract_bin();
    let livecycle_pdf = encrypted_fixtures_dir().join("livecycle.pdf");

    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);
    assert!(livecycle_pdf.exists(), "livecycle.pdf fixture not found");

    let output = Command::new(&bin)
        .args(["extract", livecycle_pdf.to_str().unwrap()])
        .output()
        .expect("Failed to execute pdftract extract");

    // Exit code should be 3 (encryption failure)
    assert_eq!(output.status.code().unwrap_or(0), 3,
        "Expected exit code 3 for encryption failure");

    println!("✓ Exit code 3 correctly returned for encrypted PDF without password");
}

/// Test that wrong password also produces ENCRYPTION_UNSUPPORTED
#[test]
fn test_wrong_password_encryption_unsupported() {
    let bin = pdftract_bin();
    let livecycle_pdf = encrypted_fixtures_dir().join("livecycle.pdf");

    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);
    assert!(livecycle_pdf.exists(), "livecycle.pdf fixture not found");

    // Supply an incorrect password
    let output = Command::new(&bin)
        .args(["extract", "--password", "wrongpassword", livecycle_pdf.to_str().unwrap()])
        .output()
        .expect("Failed to execute pdftract extract with wrong password");

    // Should still fail with exit code 3
    assert_eq!(output.status.code().unwrap_or(0), 3,
        "Expected exit code 3 for encryption failure with wrong password");

    // Should emit ENCRYPTION_UNSUPPORTED
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);
    let combined = format!("{} {}", stderr, stdout);

    assert!(combined.contains("ENCRYPTION_UNSUPPORTED"),
        "Output should contain ENCRYPTION_UNSUPPORTED, got: {}", combined);

    println!("✓ Wrong password correctly produces ENCRYPTION_UNSUPPORTED");
}

/// Test that encrypted fixtures produce consistent error behavior
///
/// This is a parameterized test that checks multiple encrypted fixtures
/// for consistent error handling.
#[test]
fn test_encryption_error_consistency() {
    let bin = pdftract_bin();
    let fixtures_dir = encrypted_fixtures_dir();

    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    // List of encrypted fixtures that should produce ENCRYPTION_UNSUPPORTED
    // These are files that either:
    // 1. Have unknown encryption handlers (like livecycle.pdf)
    // 2. Require passwords we don't have
    let encrypted_fixtures = vec![
        "livecycle.pdf",
        // Add more fixtures here as they become available
    ];

    for fixture_name in encrypted_fixtures {
        let fixture_path = fixtures_dir.join(fixture_name);

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

        // Exit code should be 3 for encryption failures
        let exit_code = output.status.code().unwrap_or(0);
        assert!(exit_code == 3,
            "{} should exit with code 3 for ENCRYPTION_UNSUPPORTED, got: {}",
            fixture_name, exit_code);

        println!("✓ {} correctly handled as encrypted file", fixture_name);
    }
}
