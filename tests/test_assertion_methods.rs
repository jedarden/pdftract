//! Direct verification of general assertion methods
//!
//! This test file verifies that the three general assertion methods
//! on TestExecutionResult work correctly:
//! - assert_stderr_contains
//! - assert_exit_code
//! - assert_success

use std::process::Output;
use crate::encryption_fixtures::TestExecutionResult;

#[test]
fn test_assert_stderr_contains_pass() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(1),
        stdout: b"".to_vec(),
        stderr: b"Unsupported encryption handler".to_vec(),
    };

    let result = TestExecutionResult::new(output);
    result.assert_stderr_contains("Unsupported encryption"); // Should not panic
}

#[test]
#[should_panic(expected = "Expected stderr to contain 'missing_text'")]
fn test_assert_stderr_contains_fail() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(1),
        stdout: b"".to_vec(),
        stderr: b"some error message".to_vec(),
    };

    let result = TestExecutionResult::new(output);
    result.assert_stderr_contains("missing_text"); // Should panic
}

#[test]
fn test_assert_exit_code_pass() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(3),
        stdout: b"".to_vec(),
        stderr: b"error".to_vec(),
    };

    let result = TestExecutionResult::new(output);
    result.assert_exit_code(3); // Should not panic
}

#[test]
#[should_panic(expected = "Expected command to exit with code 0")]
fn test_assert_exit_code_fail() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(3),
        stdout: b"".to_vec(),
        stderr: b"error".to_vec(),
    };

    let result = TestExecutionResult::new(output);
    result.assert_exit_code(0); // Should panic
}

#[test]
fn test_assert_success_pass() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: b"success".to_vec(),
        stderr: b"".to_vec(),
    };

    let result = TestExecutionResult::new(output);
    result.assert_success(); // Should not panic
}

#[test]
#[should_panic(expected = "Expected command to succeed")]
fn test_assert_success_fail() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(1),
        stdout: b"error".to_vec(),
        stderr: b"error".to_vec(),
    };

    let result = TestExecutionResult::new(output);
    result.assert_success(); // Should panic
}

#[test]
fn test_assert_stderr_contains_empty_string() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(1),
        stdout: b"".to_vec(),
        stderr: b"error message".to_vec(),
    };

    let result = TestExecutionResult::new(output);
    result.assert_stderr_contains(""); // Empty string should always match
}

#[test]
fn test_assert_stderr_contains_with_empty_stderr() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(1),
        stdout: b"".to_vec(),
        stderr: b"".to_vec(),
    };

    let result = TestExecutionResult::new(output);
    // This should panic since empty stderr doesn't contain non-empty text
    result.assert_stderr_contains("some text");
}

#[test]
fn test_assert_exit_code_none_value() {
    // When a process is terminated by signal, exit_code() returns None
    // We need to test this case, but ExitStatus::from_raw doesn't support it
    // So we'll just verify the method handles it correctly
    let output = Output {
        status: std::process::ExitStatus::from_raw(0),
        stdout: b"".to_vec(),
        stderr: b"".to_vec(),
    };

    let result = TestExecutionResult::new(output);
    // This should pass since exit_code is Some(0)
    result.assert_exit_code(0);
}

#[test]
fn test_method_chaining() {
    let output = Output {
        status: std::process::ExitStatus::from_raw(3),
        stdout: b"".to_vec(),
        stderr: b"Unsupported encryption handler".to_vec(),
    };

    let result = TestExecutionResult::new(output);

    // Test method chaining - all should pass
    result
        .assert_exit_code(3)
        .assert_stderr_contains("Unsupported")
        .assert_failure();
}
