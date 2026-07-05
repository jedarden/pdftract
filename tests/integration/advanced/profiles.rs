//! Profile validation integration tests (Phase 7 exit gate).
//!
//! Tests profile validation and resolution order:
//! - Invalid profiles are rejected with clear error messages
//! - Valid profiles pass validation
//! - Profile resolution order matches specification: built-in < user < --profile-dir

#![cfg(feature = "profiles")]

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

/// Path to fixtures directory
fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/profiles")
}

/// Test that invalid profiles are rejected with error messages
#[test]
fn test_invalid_profiles_rejected() {
    let invalid_dir = fixtures_dir().join("invalid");
    let bin = pdftract_bin();

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    let files = fs::read_dir(&invalid_dir).expect("Failed to read invalid profiles directory");

    for entry in files {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        // Skip non-YAML files
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }

        let filename = path.file_name().unwrap().to_string_lossy().to_string();

        // Run pdftract profiles validate
        let output = Command::new(&bin)
            .args(["profiles", "validate", path.to_str().unwrap()])
            .output()
            .expect("Failed to execute pdftract profiles validate");

        // Should fail (non-zero exit code)
        assert!(!output.status.success(),
            "Profile {} should fail validation but succeeded (exit code: {:?})",
            filename, output.status.code());

        // Should have error output
        let stderr = String::from_utf8_lossy(&output.stderr);
        assert!(!stderr.trim().is_empty(),
            "Profile {} should produce error output, but got empty stderr",
            filename);

        // For secret-key.yaml, should mention forbidden key
        if filename.contains("secret-key") {
            assert!(stderr.to_lowercase().contains("forbidden")
                || stderr.to_lowercase().contains("secret")
                || stderr.to_lowercase().contains("key"),
                "secret-key.yaml should produce forbidden key error, got: {}", stderr);
        }

        // For malformed.yaml, should have YAML parse error
        if filename.contains("malformed") {
            assert!(stderr.to_lowercase().contains("yaml")
                || stderr.to_lowercase().contains("parse")
                || stderr.to_lowercase().contains("syntax"),
                "malformed.yaml should produce YAML parse error, got: {}", stderr);
        }

        println!("✓ {} correctly rejected: {}", filename, stderr.trim());
    }
}

/// Test that valid profiles pass validation
#[test]
fn test_valid_profiles_accepted() {
    let valid_dir = fixtures_dir().join("valid");
    let bin = pdftract_bin();

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    let files = fs::read_dir(&valid_dir).expect("Failed to read valid profiles directory");

    for entry in files {
        let entry = entry.expect("Failed to read directory entry");
        let path = entry.path();

        // Skip non-YAML files
        if path.extension().and_then(|s| s.to_str()) != Some("yaml") {
            continue;
        }

        let filename = path.file_name().unwrap().to_string_lossy().to_string();

        // Run pdftract profiles validate
        let output = Command::new(&bin)
            .args(["profiles", "validate", path.to_str().unwrap()])
            .output()
            .expect("Failed to execute pdftract profiles validate");

        // Should succeed (exit code 0)
        assert!(output.status.success(),
            "Profile {} should pass validation but failed (exit code: {:?}), stderr: {}",
            filename, output.status.code(), String::from_utf8_lossy(&output.stderr));

        // Should have success output
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(stdout.to_lowercase().contains("valid"),
            "Profile {} should produce 'valid' output, got: {}", filename, stdout);

        println!("✓ {} correctly accepted: {}", filename, stdout.trim());
    }
}

/// Test profile resolution order: built-in < user < --profile-dir
#[test]
fn test_profile_resolution_order() {
    let bin = pdftract_bin();
    let resolution_dir = fixtures_dir().join("resolution");

    // Ensure binary exists
    assert!(bin.exists(), "pdftract binary not found at {:?}", bin);

    // Test 1: List built-in profiles (should include invoice with priority 50)
    let output = Command::new(&bin)
        .args(["profiles", "list"])
        .output()
        .expect("Failed to execute pdftract profiles list");

    assert!(output.status.success(), "profiles list failed");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("invoice"), "Built-in invoice profile should be listed");

    // Test 2: List with --profile-dir (should show custom invoice with priority 100)
    let output = Command::new(&bin)
        .args(["profiles", "list", "--profile-dir", resolution_dir.to_str().unwrap()])
        .output()
        .expect("Failed to execute pdftract profiles list with --profile-dir");

    assert!(output.status.success(), "profiles list with --profile-dir failed");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should still show invoice profile
    assert!(stdout.contains("invoice"), "Invoice profile should be listed");

    // The custom profile should have higher priority
    assert!(stdout.contains("100"),
        "Custom invoice profile with priority 100 should be visible");

    // Should indicate override
    assert!(stdout.contains("override"),
        "Custom profile should indicate it overrides built-in");

    println!("✓ Profile resolution order verified");
    println!("Output with --profile-dir:\n{}", stdout);

    // Test 3: Validate the custom profile
    let custom_profile = resolution_dir.join("custom-invoice.yaml");
    let output = Command::new(&bin)
        .args(["profiles", "validate", custom_profile.to_str().unwrap()])
        .output()
        .expect("Failed to execute pdftract profiles validate");

    assert!(output.status.success(), "Custom profile should validate");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.to_lowercase().contains("valid"),
        "Custom profile validation should succeed, got: {}", stdout);

    println!("✓ Custom profile validated successfully");
}

/// Test that all invalid fixtures produce specific error types
#[test]
fn test_invalid_fixture_error_types() {
    let invalid_dir = fixtures_dir().join("invalid");
    let bin = pdftract_bin();

    let tests = vec![
        ("secret-key.yaml", vec!["forbidden", "key", "secret"]),
        ("malformed.yaml", vec!["yaml", "parse", "syntax", "error"]),
        ("unknown-key.yaml", vec!["unknown", "key", "invalid", "field"]),
        ("bad-combinator.yaml", vec!["combinator", "match", "invalid"]),
    ];

    for (filename, expected_keywords) in tests {
        let path = invalid_dir.join(filename);
        let output = Command::new(&bin)
            .args(["profiles", "validate", path.to_str().unwrap()])
            .output()
            .expect("Failed to execute pdftract profiles validate");

        assert!(!output.status.success(), "{} should fail validation", filename);

        let stderr = String::from_utf8_lossy(&output.stderr).to_lowercase();
        let stdout = String::from_utf8_lossy(&output.stdout).to_lowercase();
        let combined = format!("{} {}", stderr, stdout);

        let has_keyword = expected_keywords.iter()
            .any(|kw| combined.contains(kw));

        assert!(has_keyword,
            "{} should contain one of {:?}, got: {}",
            filename, expected_keywords, combined);

        println!("✓ {} produced expected error type", filename);
    }
}
