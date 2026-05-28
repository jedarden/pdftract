//! Tests for multi-output CLI validation (bead pdftract-37qim).
//!
//! This test suite verifies that the CLI properly validates multi-output
//! flags according to the rules in Phase 6.6 of the plan.

use std::path::PathBuf;
use std::process::Command;

/// Helper to run pdftract extract with the given arguments.
fn run_extract(args: &[&str]) -> (bool, String, String) {
    let output = Command::new("cargo")
        .args(["run", "--quiet", "--", "extract"])
        .args(args)
        .output()
        .expect("failed to execute pdftract");

    let success = output.status.success();
    let stdout = String::from_utf8_lossy(&output.stdout).to_string();
    let stderr = String::from_utf8_lossy(&output.stderr).to_string();
    (success, stdout, stderr)
}

#[test]
fn test_default_single_json_to_stdout() {
    // Default: no output flags -> single JSON to stdout
    let (success, stdout, _stderr) = run_extract(&["tests/fixtures/empty.pdf"]);
    assert!(success, "extraction should succeed");
    assert!(stdout.contains("{") || stdout.contains("[]"), "should output JSON");
}

#[test]
fn test_json_flag_creates_file() {
    // --json out.json -> creates JSON file
    let output_path = "/tmp/test_json_output.json";
    let _ = std::fs::remove_file(output_path); // Clean up first

    let (success, _stdout, _stderr) = run_extract(&["--json", output_path, "tests/fixtures/empty.pdf"]);
    assert!(success, "extraction should succeed");
    assert!(std::path::Path::new(output_path).exists(), "output file should be created");

    let _ = std::fs::remove_file(output_path); // Clean up
}

#[test]
fn test_json_and_md_flags_create_two_files() {
    // --json a.json --md b.md -> 2 OutputSpecs built
    let json_path = "/tmp/test_multi_a.json";
    let md_path = "/tmp/test_multi_b.md";
    let _ = std::fs::remove_file(json_path);
    let _ = std::fs::remove_file(md_path);

    let (success, _stdout, _stderr) = run_extract(&[
        "--json", json_path,
        "--md", md_path,
        "tests/fixtures/empty.pdf",
    ]);
    assert!(success, "extraction should succeed with two output files");
    assert!(std::path::Path::new(json_path).exists(), "JSON output file should be created");
    assert!(std::path::Path::new(md_path).exists(), "MD output file should be created");

    let _ = std::fs::remove_file(json_path);
    let _ = std::fs::remove_file(md_path);
}

#[test]
fn test_duplicate_json_flag_rejected() {
    // --json a.json --json b.json -> CLI error "duplicate format"
    // Note: clap prevents duplicate flags on the same command line,
    // so we test via --format with duplicate json in the list
    let (success, _stdout, stderr) = run_extract(&[
        "--format", "json,json",
        "-o", "/tmp/out",
        "tests/fixtures/empty.pdf",
    ]);
    assert!(!success, "duplicate format should be rejected");
    assert!(
        stderr.contains("duplicate") || stderr.contains("Duplicate"),
        "error should mention duplicate format"
    );
}

#[test]
fn test_ndjson_conflicts_with_md() {
    // --ndjson --md b.md -> CLI error "--ndjson cannot be combined"
    // This should be caught by clap's conflicts_with_all
    let (success, _stdout, stderr) = run_extract(&[
        "--ndjson",
        "--md", "/tmp/out.md",
        "tests/fixtures/empty.pdf",
    ]);
    assert!(!success, "ndjson with md should be rejected");
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflicts"),
        "error should mention conflict"
    );
}

#[test]
fn test_ndjson_conflicts_with_format_list() {
    // --ndjson --format json,md -> CLI error
    let (success, _stdout, stderr) = run_extract(&[
        "--ndjson",
        "--format", "json",
        "-o", "/tmp/out",
        "tests/fixtures/empty.pdf",
    ]);
    assert!(!success, "ndjson with --format should be rejected");
    assert!(
        stderr.contains("cannot be used with") || stderr.contains("conflicts"),
        "error should mention conflict"
    );
}

#[test]
fn test_md_to_stdout_and_json_to_file() {
    // --md - --json out.json -> 2 specs, MD=Stdout, JSON=File
    let json_path = "/tmp/test_stdout_file.json";
    let _ = std::fs::remove_file(json_path);

    let (success, stdout, _stderr) = run_extract(&[
        "--md", "-",
        "--json", json_path,
        "tests/fixtures/empty.pdf",
    ]);
    assert!(success, "extraction should succeed");
    assert!(!stdout.is_empty(), "should have stdout output (Markdown)");
    assert!(std::path::Path::new(json_path).exists(), "JSON file should be created");

    let _ = std::fs::remove_file(json_path);
}

#[test]
fn test_multiple_stdout_rejected() {
    // --md - --json - -> CLI error "at most one stdout"
    let (success, _stdout, stderr) = run_extract(&[
        "--md", "-",
        "--json", "-",
        "tests/fixtures/empty.pdf",
    ]);
    assert!(!success, "multiple stdout destinations should be rejected");
    assert!(
        stderr.contains("at most one") || stderr.contains("stdout"),
        "error should mention at most one stdout"
    );
}

#[test]
fn test_format_with_output_base() {
    // --format json,md -o out -> 2 specs, out.json + out.md
    let base = "/tmp/test_format_out";
    let json_path = format!("{}.json", base);
    let md_path = format!("{}.md", base);
    let _ = std::fs::remove_file(&json_path);
    let _ = std::fs::remove_file(&md_path);

    let (success, _stdout, _stderr) = run_extract(&[
        "--format", "json,md",
        "-o", base,
        "tests/fixtures/empty.pdf",
    ]);
    assert!(success, "extraction should succeed");
    assert!(std::path::Path::new(&json_path).exists(), "JSON file should be created");
    assert!(std::path::Path::new(&md_path).exists(), "MD file should be created");

    let _ = std::fs::remove_file(&json_path);
    let _ = std::fs::remove_file(&md_path);
}

#[test]
fn test_format_requires_output_base() {
    // --format json (without -o) -> CLI error
    let (success, _stdout, stderr) = run_extract(&[
        "--format", "json",
        "tests/fixtures/empty.pdf",
    ]);
    assert!(!success, "--format without -o should be rejected");
    assert!(
        stderr.contains("requires") || stderr.contains("output"),
        "error should mention --format requires -o"
    );
}

#[test]
fn test_invalid_format_name() {
    // --format invalid,json -> CLI error
    let (success, _stdout, stderr) = run_extract(&[
        "--format", "invalid,json",
        "-o", "/tmp/out",
        "tests/fixtures/empty.pdf",
    ]);
    assert!(!success, "invalid format name should be rejected");
    assert!(
        stderr.contains("invalid format") || stderr.contains("unknown format"),
        "error should mention invalid format"
    );
}

#[test]
fn test_format_text_md_json_creates_three_files() {
    // --format text,md,json -o out -> 3 specs
    let base = "/tmp/test_three_formats";
    let text_path = format!("{}.txt", base);
    let md_path = format!("{}.md", base);
    let json_path = format!("{}.json", base);
    let _ = std::fs::remove_file(&text_path);
    let _ = std::fs::remove_file(&md_path);
    let _ = std::fs::remove_file(&json_path);

    let (success, _stdout, _stderr) = run_extract(&[
        "--format", "text,md,json",
        "-o", base,
        "tests/fixtures/empty.pdf",
    ]);
    assert!(success, "extraction with three formats should succeed");
    assert!(std::path::Path::new(&text_path).exists(), "Text file should be created");
    assert!(std::path::Path::new(&md_path).exists(), "MD file should be created");
    assert!(std::path::Path::new(&json_path).exists(), "JSON file should be created");

    let _ = std::fs::remove_file(&text_path);
    let _ = std::fs::remove_file(&md_path);
    let _ = std::fs::remove_file(&json_path);
}

#[test]
fn test_text_flag_with_dash_for_stdout() {
    // --text - -> plain text to stdout
    let (success, stdout, _stderr) = run_extract(&[
        "--text", "-",
        "tests/fixtures/empty.pdf",
    ]);
    assert!(success, "text to stdout should succeed");
    // Empty PDF might have no text, but we should not get JSON
    assert!(!stdout.contains("{"), "should not output JSON");
}
