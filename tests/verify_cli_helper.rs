//! Verification test for CLI invocation helper
//!
//! This test demonstrates that the CLI invocation helper functions work correctly
//! and can be used in integration tests.

use std::path::PathBuf;
use super::test_helpers::{invoke_cli_on_fixture, invoke_cli_on_fixture_with_output, Fixtures};

#[test]
fn test_cli_invocation_helper_basic() {
    println!("Testing CLI invocation helper functionality...\n");

    // Test 1: Verify helper function creates correct Command structure
    println!("Test 1: Command structure verification");
    let fixture_path = PathBuf::from("test-minimal.pdf");
    let cmd = invoke_cli_on_fixture(&fixture_path);

    let program = cmd.get_program().to_string_lossy().to_string();
    println!("  Program: {}", program);

    let args: Vec<_> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();
    println!("  Arguments: {:?}", args);

    assert_eq!(program, "pdftract", "CLI program should be 'pdftract', got '{}'. This ensures the correct binary is being tested.", program);
    assert_eq!(args.len(), 2, "CLI command should have exactly 2 arguments (extract, --json), got {}. Argument count mismatch indicates the helper function is not constructing commands correctly.", args.len());
    assert_eq!(args[0], "extract", "First argument should be 'extract', got '{}'. This verifies the pdftract subcommand is correct.", args[0]);
    assert_eq!(args[1], "--json", "Second argument should be '--json', got '{}'. This verifies the output format flag is correct.", args[1]);
    println!("  ✓ Command structure is correct\n");
}

#[test]
fn test_cli_invocation_helper_with_output() {
    println!("Test 2: Command with output path verification");

    let fixture_path = PathBuf::from("test-minimal.pdf");
    let output_path = PathBuf::from("output.json");
    let cmd = invoke_cli_on_fixture_with_output(&fixture_path, &output_path);

    let args: Vec<_> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();
    println!("  Arguments: {:?}", args);

    assert_eq!(args.len(), 3, "CLI command with output should have exactly 3 arguments (extract, --json, output_path), got {}. This ensures the output path is properly passed to the CLI.", args.len());
    assert_eq!(args[0], "extract", "First argument should be 'extract', got '{}'. Verifies pdftract subcommand is correct.", args[0]);
    assert_eq!(args[1], "--json", "Second argument should be '--json', got '{}'. Verifies output format is JSON.", args[1]);
    assert_eq!(args[2], output_path, "Third argument should be output path '{}', got '{}'. This ensures the output file path is correctly passed.", output_path, args[2]);
    println!("  ✓ Command with output path is correct\n");
}

#[test]
fn test_cli_invocation_helper_with_real_fixture() {
    println!("Test 3: Real fixture path integration");

    let fixtures = Fixtures::new();
    let real_fixture = fixtures.get("test-minimal.pdf");

    if real_fixture.exists() {
        println!("  Fixture found: {}", real_fixture.display());

        let cmd = invoke_cli_on_fixture(&real_fixture);
        let program = cmd.get_program().to_string_lossy().to_string();
        let args: Vec<_> = cmd.get_args().map(|s| s.to_string_lossy().to_string()).collect();

        println!("  Would execute: {} {} {} {}",
                 program, args[0], args[1], real_fixture.display());
        println!("  ✓ Command created successfully for real fixture\n");
    } else {
        println!("  Note: Fixture not found (expected if running outside test environment)\n");
    }
}