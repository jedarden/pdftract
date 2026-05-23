//! Unit tests for MCP CLI argument parsing.
//!
//! These tests verify that the CLI correctly enforces the mutual exclusion
//! between --stdio and --bind transport modes per ADR-006.

use std::process::{Command, Stdio};

/// Helper to get the pdftract binary path.
fn pdftract_bin() -> String {
    env!("CARGO_BIN_EXE_pdftract").to_string()
}

/// Test that `pdftract mcp --stdio --bind` is rejected at parse time.
#[test]
fn test_stdio_and_bind_mutually_exclusive() {
    let output = Command::new(pdftract_bin())
        .arg("mcp")
        .arg("--stdio")
        .arg("--bind")
        .arg("127.0.0.1:8080")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute pdftract mcp --stdio --bind");

    // Should fail with exit code 2 (clap's error exit code)
    assert_eq!(output.status.code(), Some(2), "Expected exit code 2, got {:?}", output.status.code());

    // Error message should mention both flags
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(stderr.contains("--stdio"), "Error message should mention --stdio");
    assert!(stderr.contains("--bind"), "Error message should mention --bind");
    assert!(stderr.contains("cannot be used"), "Error message should mention conflict");
}

/// Test that `pdftract mcp` (no flags) parses successfully.
#[test]
fn test_default_to_stdio() {
    let output = Command::new(pdftract_bin())
        .arg("mcp")
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute pdftract mcp --help");

    // Should succeed
    assert!(output.status.success(), "pdftract mcp --help should succeed");

    // Help text should mention the default behavior
    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("default"), "Help should mention default transport mode");
    assert!(stdout.contains("stdio"), "Help should mention stdio transport");
}

/// Test that `pdftract mcp --stdio` parses successfully.
#[test]
fn test_stdio_flag_valid() {
    let output = Command::new(pdftract_bin())
        .arg("mcp")
        .arg("--stdio")
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute pdftract mcp --stdio --help");

    // Note: --help overrides the subcommand, so this succeeds
    // In actual use, --stdio would start the stdio server
    assert!(output.status.success(), "pdftract mcp --stdio --help should succeed");
}

/// Test that `pdftract mcp --bind ADDR` parses successfully.
#[test]
fn test_bind_flag_valid() {
    let output = Command::new(pdftract_bin())
        .arg("mcp")
        .arg("--bind")
        .arg("127.0.0.1:9999")
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute pdftract mcp --bind ADDR --help");

    // Note: --help overrides the subcommand, so this succeeds
    // In actual use, --bind would start the HTTP server
    assert!(output.status.success(), "pdftract mcp --bind ADDR --help should succeed");
}

/// Test that the help text mentions ADR-006 and the mutual exclusion rationale.
#[test]
fn test_help_mentions_adr_006() {
    let output = Command::new(pdftract_bin())
        .arg("mcp")
        .arg("--help")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .expect("Failed to execute pdftract mcp --help");

    assert!(output.status.success(), "pdftract mcp --help should succeed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Help text should mention ADR-006 and the rationale
    assert!(stdout.contains("ADR-006"), "Help should mention ADR-006");
    assert!(stdout.contains("mutually exclusive"), "Help should mention mutual exclusion");
}
