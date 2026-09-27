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

#[test]
fn cli_anchor_metadata_matches_json_blocks_and_restarts_per_page() {
    let json_output = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .args(["extract", "--no-cache", "--json", "-", FIXTURE])
        .output()
        .expect("pdftract JSON extraction should run");
    assert!(
        json_output.status.success(),
        "CLI JSON extraction failed: {}",
        String::from_utf8_lossy(&json_output.stderr)
    );
    let json: serde_json::Value =
        serde_json::from_slice(&json_output.stdout).expect("CLI JSON output should be valid JSON");
    let pages = json["pages"]
        .as_array()
        .expect("CLI JSON output should contain pages");
    let expected: Vec<(usize, usize, String)> = pages
        .iter()
        .enumerate()
        .flat_map(|(page_position, page)| {
            page["blocks"]
                .as_array()
                .expect("each JSON page should contain blocks")
                .iter()
                .enumerate()
                .map(move |(block_index, block)| {
                    (
                        page["index"].as_u64().unwrap_or(page_position as u64) as usize,
                        block_index,
                        block["kind"]
                            .as_str()
                            .expect("each JSON block should contain a kind")
                            .to_string(),
                    )
                })
        })
        .collect();

    let markdown_output = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .args([
            "extract",
            "--no-cache",
            "--md",
            "-",
            "--md-anchors",
            FIXTURE,
        ])
        .output()
        .expect("pdftract Markdown extraction should run");
    assert!(
        markdown_output.status.success(),
        "CLI Markdown extraction failed: {}",
        String::from_utf8_lossy(&markdown_output.stderr)
    );
    let markdown = String::from_utf8(markdown_output.stdout).expect("Markdown should be UTF-8");
    let anchors = parse_anchors(&markdown);

    assert_eq!(anchors.len(), expected.len());
    for (anchor, (page, block, kind)) in anchors.iter().zip(expected) {
        assert_eq!(
            (anchor.page, anchor.block, anchor.kind.as_str()),
            (page, block, kind.as_str())
        );
    }
}
