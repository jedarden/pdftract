//! CLI-level regression coverage for Markdown positional anchors.

use pdftract_core::markdown::{parse_anchors, ANCHOR_REGEX_PATTERN};
use regex::Regex;
use std::process::Command;

const FIXTURE: &str = "../../tests/fixtures/test-minimal.pdf";

#[test]
fn cli_markdown_anchors_round_trip_through_parser() {
    let output = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .args([
            "extract",
            "--no-cache",
            "--md",
            "-",
            "--md-anchors",
            FIXTURE,
        ])
        .output()
        .expect("pdftract binary should run");
    assert!(
        output.status.success(),
        "CLI Markdown extraction failed: {}",
        String::from_utf8_lossy(&output.stderr)
    );

    let markdown = String::from_utf8(output.stdout).expect("Markdown output should be UTF-8");
    let anchors = parse_anchors(&markdown);
    assert!(
        !anchors.is_empty(),
        "CLI output should contain at least one anchor"
    );
    assert_eq!(anchors[0].page, 0);
    assert_eq!(anchors[0].block, 0);

    let regex = Regex::new(ANCHOR_REGEX_PATTERN).expect("published anchor regex must compile");
    let anchor_lines: Vec<&str> = markdown
        .lines()
        .filter(|line| line.contains("<!-- pdftract:"))
        .collect();
    assert_eq!(anchor_lines.len(), anchors.len());
    assert!(
        anchor_lines.iter().all(|line| regex.is_match(line)),
        "every CLI anchor must match the published regex"
    );
}
