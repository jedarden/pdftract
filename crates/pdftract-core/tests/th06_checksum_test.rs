//! TH-06 supply-chain gate tests for build-time data file checksums.
//!
//! This test module verifies that the build.rs checksum verification works
//! correctly. It tests both the normal case (all checksums match) and the
//! tampering case (checksum mismatch aborts the build).
//!
//! Bead: pdftract-1xf4d (TH-06 supply-chain gate)
//! Plan: line 909 (Build-time data files checksum pin)

use std::fs;
use std::path::Path;

/// Helper to compute SHA-256 checksum of a file.
fn compute_sha256(path: &Path) -> String {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();
    let contents = fs::read(path).unwrap();
    hasher.update(&contents);
    format!("{:x}", hasher.finalize())
}

/// Test that tampering with a build-time data file aborts the build.
///
/// This test verifies the TH-06 supply-chain gate by:
/// 1. Backing up the original std14-metrics.json
/// 2. Tampering with it (writing a single byte change)
/// 3. Attempting to build pdftract-core (should fail with checksum error)
/// 4. Restoring the original file
#[test]
fn test_tampering_detection() {
    // Skip this test in CI environments where we don't want to modify build files
    if std::env::var("CI").is_ok() {
        println!("Skipping tampering test in CI environment");
        return;
    }

    let build_dir = Path::new("crates/pdftract-core/build");
    let test_file = build_dir.join("std14-metrics.json");
    let backup_file = build_dir.join("std14-metrics.json.backup");

    // Skip if the test file doesn't exist
    if !test_file.exists() {
        println!("Skipping tampering test: {} not found", test_file.display());
        return;
    }

    // Backup the original file
    let original_contents = fs::read(&test_file).unwrap();
    fs::write(&backup_file, &original_contents).unwrap();

    // Tamper with the file (change a single byte)
    let mut tampered_contents = original_contents.clone();
    if !tampered_contents.is_empty() {
        tampered_contents[0] = tampered_contents[0].wrapping_add(1);
    }
    fs::write(&test_file, &tampered_contents).unwrap();

    // Verify the checksum actually changed
    let original_checksum = compute_sha256(&backup_file);
    let tampered_checksum = compute_sha256(&test_file);
    assert_ne!(
        original_checksum, tampered_checksum,
        "Tampering should change the checksum"
    );

    // Attempt to build pdftract-core - should fail with checksum error
    let output = std::process::Command::new("cargo")
        .args(["build", "--package", "pdftract-core"])
        .output()
        .unwrap();

    // Restore the original file immediately
    fs::write(&test_file, &original_contents).unwrap();
    fs::remove_file(&backup_file).unwrap();

    // Verify the build failed due to checksum mismatch
    let stderr = String::from_utf8_lossy(&output.stderr);
    let stdout = String::from_utf8_lossy(&output.stdout);

    let combined_output = format!("{}{}", stderr, stdout);

    // The build should fail (non-zero exit code)
    assert!(
        !output.status.success(),
        "Build should fail when checksums don't match.\nstdout:\n{}\nstderr:\n{}",
        stdout, stderr
    );

    // The error message should mention checksum verification
    assert!(
        combined_output.contains("checksum")
            || combined_output.contains("Checksum")
            || combined_output.contains("CHECKSUMS"),
        "Error message should mention checksum verification.\nOutput:\n{}",
        combined_output
    );

    // Verify the file was restored correctly
    let restored_checksum = compute_sha256(&test_file);
    assert_eq!(
        original_checksum, restored_checksum,
        "File should be restored to original state"
    );
}

/// Test that normal build succeeds when all checksums match.
///
/// This is a sanity check that the checksum verification doesn't
/// incorrectly fail when all files are intact.
#[test]
fn test_normal_build_checksums_pass() {
    // This test just verifies that a clean build succeeds
    // If checksums are wrong, the build will fail and this test will fail
    let output = std::process::Command::new("cargo")
        .args(["check", "--package", "pdftract-core"])
        .output()
        .unwrap();

    let stderr = String::from_utf8_lossy(&output.stderr);

    // The build should succeed
    assert!(
        output.status.success(),
        "Normal build should succeed when checksums match.\nstderr:\n{}",
        stderr
    );

    // Should not contain checksum error messages
    assert!(
        !stderr.contains("Checksum verification failed"),
        "Normal build should not report checksum failures.\nstderr:\n{}",
        stderr
    );
}
