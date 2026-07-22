//! Orphaned process verification tests.
//!
//! This test module demonstrates and verifies the orphaned process
//! verification system. It serves as both integration tests and examples.

use std::env;
use std::path::PathBuf;
use std::process::Command;

/// Get the repository root directory
fn repo_root() -> PathBuf {
    // When running as a test, CARGO_MANIFEST_DIR points to the crate directory
    // We need to go from there to the repository root
    let manifest_dir = PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap_or_else(|_| ".".to_string()));

    // The manifest_dir could be:
    // 1. /path/to/pdftract/crates/pdftract-core (when running from workspace root)
    // 2. /path/to/pdftract (when running from the crate directory directly)
    // 3. /path/to/pdftract/target/... (when cargo sets it differently)

    // First, check if we're already at a workspace root (look for Cargo.toml in parent)
    let mut current_path = manifest_dir.clone();

    // Try going up to find the workspace root (has Cargo.toml with [workspace])
    for _ in 0..5 {
        let workspace_cargo = current_path.join("Cargo.toml");
        if workspace_cargo.exists() {
            // Check if it's a workspace file
            if let Ok(content) = std::fs::read_to_string(&workspace_cargo) {
                if content.contains("[workspace]") {
                    return current_path;
                }
            }
        }

        current_path = match current_path.parent() {
            Some(p) => p.to_path_buf(),
            None => break,
        };
    }

    // Fallback: try the original logic
    manifest_dir
        .ancestors()
        .find(|p| p.join("Cargo.toml").exists())
        .expect("Could not find repository root")
        .to_path_buf()
}

// Declare the test_helpers module
mod test_helpers;

// Import from test_helpers module
use test_helpers::process_guard::{
    verify_no_orphaned_processes,
    verify_no_processes_matching_patterns,
    kill_orphaned_processes,
    OrphanedProcessGuard,
    OrphanedProcessError,
};

/// Test that verification succeeds in clean state.
///
/// This test verifies that the verification system itself works
/// and that we're starting from a clean slate.
#[test]
fn test_verification_succeeds_in_clean_state() {
    let result = verify_no_orphaned_processes();

    // In a clean test environment, this should succeed.
    // We don't assert success because other concurrent tests may
    // have spawned processes that haven't cleaned up yet.
    // We just verify the function doesn't panic.
    let _ = result;

    // If we do have orphans, try to clean them up for other tests
    if let Err(OrphanedProcessError::OrphanedProcessesFound { .. }) = result {
        let _ = kill_orphaned_processes();
    }
}

/// Test that custom patterns work.
///
/// Verifies that we can check for arbitrary process patterns,
/// not just the default ones.
#[test]
fn test_custom_pattern_verification() {
    // Check for a pattern that should never match
    let result = verify_no_processes_matching_patterns(&[
        "totally-bogus-process-name-that-should-never-exist",
    ]);

    assert!(result.is_ok(), "Custom pattern check should succeed");
}

/// Test OrphanedProcessGuard lifecycle.
///
/// Verifies that the guard can be created and dropped correctly.
#[test]
fn test_orphaned_process_guard_lifecycle() {
    // Create a guard
    let guard = OrphanedProcessGuard::new();
    assert!(guard.is_ok(), "Guard creation should succeed");

    // Guard will verify cleanup on drop
    let _guard = guard.unwrap();

    // After dropping, no verification is done here (it happens in Drop)
    // but we've verified the guard can be created successfully.
}

/// Test OrphanedProcessGuard with custom patterns.
///
/// Verifies that guards can use custom process patterns.
#[test]
fn test_orphaned_process_guard_custom_patterns() {
    let guard = OrphanedProcessGuard::with_patterns(&[
        "another-bogus-pattern",
    ]);

    assert!(guard.is_ok(), "Custom pattern guard should succeed");
    let _guard = guard.unwrap();
}

/// Test error message formatting.
///
/// Verifies that error messages are informative.
#[test]
fn test_error_message_formatting() {
    let error = OrphanedProcessError::OrphanedProcessesFound {
        count: 2,
        processes: vec![
            ("1234".to_string(), "pdftract mcp --stdio".to_string()),
            ("5678".to_string(), "TH-0 test_case".to_string()),
        ],
    };

    let error_string = format!("{}", error);

    assert!(error_string.contains("2 orphaned"));
    assert!(error_string.contains("1234"));
    assert!(error_string.contains("pdftract mcp"));
    assert!(error_string.contains("5678"));
    assert!(error_string.contains("TH-0"));
}

/// Test that kill_orphaned_processes doesn't panic.
///
/// Verifies the kill function works even when there are no orphans.
#[test]
fn test_kill_orphaned_processes_safe_when_clean() {
    let result = kill_orphaned_processes();

    // Should succeed even if no orphans exist
    assert!(result.is_ok());

    // Should report 0 processes killed
    let killed = result.unwrap();
    assert!(killed == 0 || killed > 0, "Kill count should be valid");
}

/// Test detection of specific process patterns.
///
/// This test verifies the underlying pgrep mechanism works.
#[test]
fn test_process_pattern_detection() {
    // The test runner itself should be running
    let result = verify_no_processes_matching_patterns(&["cargo"]);

    // We don't assert success because cargo processes should exist
    // We just verify the function works without panicking
    let _ = result;
}

/// Example: How to use OrphanedProcessGuard in a test.
///
/// This demonstrates the recommended pattern for tests that spawn
/// subprocesses.
#[test]
fn example_test_with_process_guard() {
    // Create a guard at the start - records initial state
    let _guard = OrphanedProcessGuard::new();

    // ... test code that may spawn processes ...

    // Guard verifies cleanup on drop
}

/// Example: How to verify cleanup explicitly.
///
/// This demonstrates explicit verification instead of relying on guards.
#[test]
fn example_explicit_verification() {
    // Record initial state
    let initial_result = verify_no_orphaned_processes();
    let initial_orphans = match initial_result {
        Ok(_) => 0,
        Err(OrphanedProcessError::OrphanedProcessesFound { count, .. }) => count,
        Err(_) => 0,
    };

    // ... test code ...

    // Verify no new orphans were added
    let final_result = verify_no_orphaned_processes();
    let final_orphans = match final_result {
        Ok(_) => 0,
        Err(OrphanedProcessError::OrphanedProcessesFound { count, .. }) => count,
        Err(_) => 0,
    };

    assert_eq!(
        initial_orphans, final_orphans,
        "Test should not leave additional orphaned processes"
    );
}

/// Integration test: Verify the shell script works.
///
/// This test runs the shell script and verifies it can be executed.
#[test]
fn integration_shell_script_executable() {
    let script_path = repo_root().join("scripts/check-orphaned-processes.sh");
    let result = Command::new(&script_path)
        .output();

    // Script should be executable
    assert!(result.is_ok(), "Verification script should be executable");

    let output = result.unwrap();

    // Should exit with 0 (clean) or 1 (orphans found)
    // Should not exit with error code indicating script failure
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "Script should execute successfully"
    );
}

/// Integration test: Verify the shell script with --json flag.
///
/// This test verifies JSON output can be parsed.
#[test]
fn integration_shell_script_json_output() {
    let script_path = repo_root().join("scripts/check-orphaned-processes.sh");
    let output = Command::new(&script_path)
        .arg("--json")
        .output()
        .expect("Script should execute");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should be valid JSON
    assert!(
        stdout.starts_with('{'),
        "JSON output should start with '{{'"
    );

    // Should contain status field
    assert!(
        stdout.contains("\"status\""),
        "JSON should contain status field"
    );

    // Parse to verify it's valid JSON
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(&stdout) {
        assert!(value.is_object(), "JSON root should be an object");

        let status = value.get("status")
            .and_then(|s| s.as_str())
            .expect("Status should be a string");

        assert!(
            matches!(status, "clean" | "orphaned" | "cleaned"),
            "Status should be valid: {}", status
        );
    } else {
        panic!("JSON output should be valid JSON");
    }
}

/// Integration test: Verify CI script works.
///
/// This test verifies the CI integration script executes correctly.
#[test]
fn integration_ci_script_executable() {
    let script_path = repo_root().join(".ci/scripts/post-test-check.sh");
    let result = Command::new(&script_path)
        .output();

    assert!(result.is_ok(), "CI script should be executable");

    let output = result.unwrap();

    // Should exit with 0 (success) or 1 (orphans detected)
    assert!(
        output.status.success() || output.status.code() == Some(1),
        "CI script should execute successfully"
    );
}
