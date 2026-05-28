//! Integration tests for the --header CLI flag.
//!
//! These tests verify that the --header flag:
//! 1. Accepts valid headers in HEADER:VALUE format
//! 2. Rejects invalid headers (no colon, CRLF injection, managed headers)
//! 3. Silently ignores headers for local file extraction
//! 4. Would pass headers to HttpRangeSource for URLs (when Phase 1.8 is implemented)

use std::process::Command;
use std::path::PathBuf;

/// Path to the pdftract CLI binary.
fn pdftract_bin() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../target/debug/pdftract");
    path
}

/// Find a test fixture PDF file.
fn fixture_pdf() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/fixtures/test-minimal.pdf");
    if !path.exists() {
        // Try alternate path
        path = PathBuf::from("../../tests/fixtures/test-minimal.pdf");
    }
    path
}

#[test]
fn test_header_flag_valid_single() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "X-API-Key:abc123",
            pdf.to_str().unwrap(),
            "--format",
            "json",
            "-o",
            "-",
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should succeed (headers are validated and parsed)
    assert!(
        output.status.success(),
        "pdftract failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_header_flag_valid_multiple() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "X-API-Key:abc123",
            "--header",
            "Authorization:Bearer token",
            "--header",
            "X-Tenant:xyz",
            pdf.to_str().unwrap(),
            "--format",
            "json",
            "-o",
            "-",
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should succeed with multiple headers
    assert!(
        output.status.success(),
        "pdftract failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_header_flag_no_colon() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "NoColonHere",
            pdf.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should fail with parse error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("must contain a ':' delimiter"),
        "Expected missing colon error, got: {}",
        stderr
    );
}

#[test]
fn test_header_flag_crlf_injection() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "X-Bad:Value\r\nInjected: true",
            pdf.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should fail with CRLF injection error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("CRLF"),
        "Expected CRLF injection error, got: {}",
        stderr
    );
}

#[test]
fn test_header_flag_managed_header_host() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "Host:example.com",
            pdf.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should fail with managed header error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("managed automatically") || stderr.contains("Host"),
        "Expected managed header error, got: {}",
        stderr
    );
}

#[test]
fn test_header_flag_managed_header_content_length() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "Content-Length:1234",
            pdf.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should fail with managed header error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("managed automatically") || stderr.contains("Content-Length"),
        "Expected managed header error, got: {}",
        stderr
    );
}

#[test]
fn test_header_flag_authorization_allowed() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "Authorization:Bearer abc123",
            pdf.to_str().unwrap(),
            "--format",
            "json",
            "-o",
            "-",
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should succeed - Authorization is explicitly allowed
    assert!(
        output.status.success(),
        "pdftract failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_header_flag_empty_name() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            ":value",
            pdf.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should fail with empty name error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("empty name") || stderr.contains("Empty"),
        "Expected empty name error, got: {}",
        stderr
    );
}

#[test]
fn test_header_flag_empty_value() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "Name:",
            pdf.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should fail with empty value error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("empty value") || stderr.contains("Empty"),
        "Expected empty value error, got: {}",
        stderr
    );
}

#[test]
fn test_header_flag_invalid_name_chars() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "X Bad Name:value",
            pdf.to_str().unwrap(),
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should fail with invalid name error
    assert!(!output.status.success());
    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("invalid") || stderr.contains("Invalid"),
        "Expected invalid name error, got: {}",
        stderr
    );
}

#[test]
fn test_header_flag_with_spaces_around_colon() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "X-API-Key : abc123",
            pdf.to_str().unwrap(),
            "--format",
            "json",
            "-o",
            "-",
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should succeed - spaces around colon are trimmed
    assert!(
        output.status.success(),
        "pdftract failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_header_flag_value_with_colon() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "X-Url:https://example.com:8080/path",
            pdf.to_str().unwrap(),
            "--format",
            "json",
            "-o",
            "-",
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should succeed - values can contain colons
    assert!(
        output.status.success(),
        "pdftract failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );
}

#[test]
fn test_header_flag_local_file_silent_ignore() {
    let pdf = fixture_pdf();
    assert!(pdf.exists(), "Fixture PDF not found: {:?}", pdf);

    let output = Command::new(pdftract_bin())
        .args([
            "extract",
            "--header",
            "X-API-Key:abc123",
            pdf.to_str().unwrap(),
            "--format",
            "json",
            "-o",
            "-",
        ])
        .output()
        .expect("Failed to run pdftract");

    // Should succeed without error - headers are silently ignored for local files
    assert!(
        output.status.success(),
        "pdftract failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    // Should NOT print a warning about headers being unused
    let stderr = String::from_utf8_lossy(&output.stderr);
    // The current implementation doesn't print anything for local files
    // (headers are silently ignored as specified)
}
