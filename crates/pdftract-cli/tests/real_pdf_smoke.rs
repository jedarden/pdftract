//! CLI smoke contract for the provenance-recorded real PDF fixture.

use std::path::Path;
use std::process::Command;

const FIXTURE: &str = "../../tests/fixtures/test-minimal.pdf";

#[test]
fn cli_extracts_fixture_in_text_and_structured_modes() {
    let text = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .args(["extract", "--no-cache", "--text", "-", FIXTURE])
        .output()
        .expect("pdftract binary should run");
    assert!(
        text.status.success(),
        "text extraction failed: {}",
        String::from_utf8_lossy(&text.stderr)
    );
    assert_eq!(
        String::from_utf8_lossy(&text.stdout).trim(),
        "Dummy PDF file"
    );

    let structured = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .args(["extract", "--no-cache", "--json", "-", FIXTURE])
        .output()
        .expect("pdftract binary should run");
    assert!(
        structured.status.success(),
        "structured extraction failed: {}",
        String::from_utf8_lossy(&structured.stderr)
    );

    let document: serde_json::Value = serde_json::from_slice(&structured.stdout)
        .expect("structured output should be valid JSON");
    assert_eq!(document["metadata"]["page_count"], 1);
    let pages = document["pages"]
        .as_array()
        .expect("structured output should contain pages");
    assert_eq!(pages.len(), 1);
    let span_text: String = pages[0]["spans"]
        .as_array()
        .expect("page should contain spans")
        .iter()
        .filter_map(|span| span["text"].as_str())
        .collect();
    assert_eq!(span_text, "Dummy PDF file");

    assert!(Path::new(FIXTURE).is_file());
}
