//! Tests for hash subcommand exit codes.

use std::process::Command;

#[test]
fn test_hash_nonexistent_file() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "pdftract",
            "--",
            "hash",
            "/nonexistent/file.pdf",
        ])
        .output()
        .expect("Failed to run pdftract hash");

    // Exit code 4 for file not found
    assert_eq!(output.status.code(), Some(4));

    // stderr should contain error message
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("not found") || stderr.contains("No such file"));
}

#[test]
fn test_hash_help() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "pdftract", "--", "hash", "--help"])
        .output()
        .expect("Failed to run pdftract hash --help");

    // Help should exit with 0
    assert_eq!(output.status.code(), Some(0));

    // Should show help text
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("Compute the PDF structural fingerprint"));
    assert!(stdout.contains("--password"));
}

#[test]
fn test_hash_url_flag() {
    let output = Command::new("cargo")
        .args(["run", "--bin", "pdftract", "--", "hash", "--help"])
        .output()
        .expect("Failed to run pdftract hash --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("header"));
}

#[cfg(feature = "remote")]
#[test]
fn test_hash_url_not_found() {
    let output = Command::new("cargo")
        .args([
            "run",
            "--bin",
            "pdftract",
            "--features",
            "remote",
            "--",
            "hash",
            "https://nonexistent.invalid/test.pdf",
        ])
        .output()
        .expect("Failed to run pdftract hash");

    // Exit code 4 for URL not found/DNS failure
    let code = output.status.code();
    assert!(
        code == Some(4) || code == Some(5),
        "Expected exit code 4 or 5, got {:?}",
        code
    );
}

#[test]
fn test_hash_basic_invocation() {
    // Test that the hash subcommand is recognized
    let output = Command::new("cargo")
        .args(["run", "--bin", "pdftract", "--", "hash", "--help"])
        .output()
        .expect("Failed to run pdftract hash --help");

    assert!(output.status.success());
}
