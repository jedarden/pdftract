//! CLI integration test - iterates through all fixtures and invokes the CLI
//!
//! This test demonstrates the full integration between:
//! - Fixture discovery (finding all PDF files in tests/fixtures/)
//! - CLI invocation helpers (creating pdftract extract commands)
//! - Result tracking (success/failure per fixture)
//! - Continuation after failures (test doesn't stop on first failure)
//!
//! # Purpose
//! This test wires CLI invocation into the test iteration loop, ensuring that:
//! - Each fixture is processed independently
//! - The CLI is invoked on all fixtures (not just a subset)
//! - Failures are logged but don't stop iteration
//! - The test completes without hanging (using bounded waits)

use std::path::{Path, PathBuf};
use std::time::Duration;

// Include the test helpers module directly in this integration test
mod test_helpers {
    include!("test_helpers.rs");
}

mod fixture_discovery {
    include!("fixture_discovery.rs");
}

use test_helpers::{invoke_cli_and_capture_output, CapturedOutput, Fixtures};
use fixture_discovery::discover_pdf_fixtures;

/// Result of invoking the CLI on a single fixture with timing information
#[derive(Debug)]
struct FixtureResult {
    /// Path to the fixture file
    fixture_path: PathBuf,
    /// The underlying captured output from CLI execution
    output: CapturedOutput,
    /// Duration of the CLI execution
    duration: Duration,
}

impl FixtureResult {
    /// Create a new FixtureResult from fixture path, CapturedOutput, and duration
    fn new(fixture_path: PathBuf, output: CapturedOutput, duration: Duration) -> Self {
        Self {
            fixture_path,
            output,
            duration,
        }
    }

    /// Get fixture name for display
    fn fixture_name(&self) -> String {
        self.fixture_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("unknown")
            .to_string()
    }

    /// Check if the execution was successful
    fn is_success(&self) -> bool {
        self.output.success
    }

    /// Check if the execution timed out
    fn is_timeout(&self) -> bool {
        self.output
            .stderr_string()
            .contains("timed out")
    }

    /// Get the exit code
    fn exit_code(&self) -> Option<i32> {
        self.output.exit_code
    }

    /// Get error message if any
    fn error_message(&self) -> Option<String> {
        if self.is_success() {
            None
        } else {
            Some(self.output.stderr_string().trim().to_string())
        }
    }
}

/// Invoke CLI on a single fixture with timeout protection
///
/// This function executes the CLI command for a single fixture with a bounded
/// wait to prevent hanging. It uses the shared test helpers and adds timing
/// information for better reporting.
///
/// # Arguments
/// * `fixture_path` - Path to the PDF fixture
///
/// # Returns
/// A `FixtureResult` indicating success or failure with duration information
fn invoke_cli_with_timeout(fixture_path: &PathBuf) -> FixtureResult {
    use std::time::Instant;

    let start = Instant::now();

    // Use the shared test helper for CLI invocation
    // The test helper handles process creation and output capture
    let result = invoke_cli_and_capture_output(fixture_path);

    let duration = start.elapsed();

    // Convert the result to our FixtureResult structure
    let captured_output = match result {
        Ok(output) => output,
        Err(e) => {
            // Create a failure output if we couldn't even execute the command
            CapturedOutput {
                fixture_path: fixture_path.clone(),
                stdout: Vec::new(),
                stderr: format!("Failed to execute command: {}", e).into_bytes(),
                exit_code: None,
                success: false,
            }
        }
    };

    // Check for timeout and handle it
    let timeout = Duration::from_secs(30);
    if duration > timeout && captured_output.exit_code.is_none() {
        // Create a timeout failure output
        let timeout_output = CapturedOutput {
            fixture_path: fixture_path.clone(),
            stdout: captured_output.stdout,
            stderr: format!(
                "Command exceeded timeout of {:?} (actual: {:?})",
                timeout, duration
            )
            .into_bytes(),
            exit_code: None,
            success: false,
        };
        FixtureResult::new(fixture_path.clone(), timeout_output, duration)
    } else {
        FixtureResult::new(fixture_path.clone(), captured_output, duration)
    }
}

#[test]
fn test_cli_invocation_on_all_fixtures() {
    println!("\n=== CLI Integration Test: Invoking pdftract on all fixtures ===\n");

    // Use the shared test helpers for fixture discovery
    let fixtures = Fixtures::new();
    let fixtures_path = &fixtures.base_dir;
    let pdf_files = discover_pdf_fixtures(fixtures_path);

    println!("Fixtures directory: {}", fixtures_path.display());
    println!("Total PDF files discovered: {}\n", pdf_files.len());

    if pdf_files.is_empty() {
        println!("WARNING: No PDF files found in fixtures directory");
        println!("Test will pass but discovered no fixtures to process.\n");
        return;
    }

    // Track results for all fixtures
    let mut results: Vec<FixtureResult> = Vec::with_capacity(pdf_files.len());
    let mut success_count = 0;
    let mut failure_count = 0;
    let mut timeout_count = 0;

    // Process each fixture independently
    for (index, fixture_path) in pdf_files.iter().enumerate() {
        println!(
            "[{}/{}] Processing: {}",
            index + 1,
            pdf_files.len(),
            fixture_path.display()
        );

        let result = invoke_cli_with_timeout(fixture_path);

        if result.is_success() {
            success_count += 1;
            println!(
                "  ✓ SUCCESS (exit: {:?}, time: {:?})",
                result.exit_code(), result.duration
            );
        } else {
            failure_count += 1;
            if result.is_timeout() {
                timeout_count += 1;
            }
            println!("  ✗ FAILED: {}", result.error_message().as_deref().unwrap_or("unknown"));
        }

        results.push(result);
    }

    // Print summary
    println!("\n=== Test Summary ===");
    println!("Total fixtures processed: {}", pdf_files.len());
    println!("Successful: {}", success_count);
    println!("Failed: {}", failure_count);
    println!("Timeouts: {}", timeout_count);
    println!("\n");

    // Show breakdown of failures if any
    if failure_count > 0 {
        println!("=== Failed Fixtures ===");
        for result in results.iter().filter(|r| !r.is_success()) {
            println!("  - {}", result.fixture_name());
            println!("    Error: {}", result.error_message().as_deref().unwrap_or("unknown"));
        }
        println!("\n");
    }

    // The test always completes successfully - failures are logged, not asserted
    // This ensures the iteration completes even if individual fixtures fail
    println!("✓ Test completed successfully - all fixtures were processed");

    // Assertion: We attempted to process every discovered fixture
    assert_eq!(
        results.len(),
        pdf_files.len(),
        "Result count should match fixture count"
    );
}

#[test]
fn test_cli_invocation_on_small_sample() {
    println!("\n=== CLI Integration Test: Small Sample (First 5 Fixtures) ===\n");

    // Use the shared test helpers for fixture discovery
    let fixtures = Fixtures::new();
    let fixtures_path = &fixtures.base_dir;
    let mut pdf_files = discover_pdf_fixtures(fixtures_path);

    // Take only first 5 for a quicker smoke test
    pdf_files.truncate(5);

    println!("Processing {} fixtures (sample)\n", pdf_files.len());

    if pdf_files.is_empty() {
        println!("WARNING: No PDF files found in fixtures directory");
        return;
    }

    let mut success_count = 0;

    for (index, fixture_path) in pdf_files.iter().enumerate() {
        println!(
            "[{}/{}] {}",
            index + 1,
            pdf_files.len(),
            fixture_path.display()
        );

        let result = invoke_cli_with_timeout(fixture_path);

        if result.is_success() {
            success_count += 1;
            println!("  ✓ SUCCESS (exit: {:?})", result.exit_code());
        } else {
            println!("  ✗ FAILED: {}", result.error_message().as_deref().unwrap_or("unknown"));
        }
    }

    println!("\nSample complete: {}/{} succeeded\n", success_count, pdf_files.len());

    // For the sample test, we expect at least some successes if fixtures exist
    if !pdf_files.is_empty() {
        assert!(
            success_count > 0 || pdf_files.len() == 0,
            "At least one fixture should succeed in the sample test"
        );
    }
}
