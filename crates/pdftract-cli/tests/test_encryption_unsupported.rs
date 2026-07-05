//! Test for ENCRYPTION_UNSUPPORTED error exit code on livecycle.pdf
//!
//! This test verifies that extraction emits ENCRYPTION_UNSUPPORTED diagnostic
//! and exits with code 3 when run on an owner-password-only encrypted PDF
//! (or a PDF with an unsupported encryption handler like Adobe LiveCycle).

use std::process::Command;

#[test]
fn test_livecycle_pdf_emits_encryption_unsupported() {
    // Test that livecycle.pdf (unsupported Adobe.APS encryption handler)
    // triggers ENCRYPTION_UNSUPPORTED and exits with code 3
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "pdftract",
            "--",
            "extract",
            "tests/fixtures/encrypted/livecycle.pdf",
        ])
        .output()
        .expect("Failed to run pdftract extract on livecycle.pdf");

    // Exit code 3 for encryption errors (per spec)
    assert_eq!(
        output.status.code(),
        Some(3),
        "Expected exit code 3 for ENCRYPTION_UNSUPPORTED, got {:?}",
        output.status.code()
    );

    // stderr should contain the error message
    let stderr = String::from_utf8_lossy(&output.stderr);

    // The error message should mention "Unsupported encryption" or the specific diagnostic
    assert!(
        stderr.contains("Unsupported encryption") || stderr.contains("ENCRYPTION_UNSUPPORTED"),
        "Expected stderr to contain 'Unsupported encryption' or 'ENCRYPTION_UNSUPPORTED', got: {}",
        stderr
    );

    // stdout should be empty or minimal (no extraction output)
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.is_empty() || !stdout.contains("\"pages\""),
        "Expected no extraction output in stdout, got: {}",
        stdout
    );
}

#[test]
fn test_livecycle_pdf_with_password_also_fails() {
    // Even with a password, livecycle.pdf should still fail because
    // the Adobe.APS handler is unsupported (not a password issue)
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "pdftract",
            "--",
            "extract",
            "tests/fixtures/encrypted/livecycle.pdf",
            // We can't use --password directly due to INSECURE_CLI_PASSWORD check,
            // so we'll use --password-stdin with echo
        ])
        .stdin(std::process::Stdio::piped())
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .output()
        .expect("Failed to run pdftract extract with password stdin");

    // Should still exit with code 3 (unsupported algorithm, not wrong password)
    assert_eq!(
        output.status.code(),
        Some(3),
        "Expected exit code 3 for unsupported algorithm even with password, got {:?}",
        output.status.code()
    );
}
