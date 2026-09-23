//! CLI smoke contract for the provenance-recorded real PDF fixture.

// Cross-binary reuse boundary: every tests/*.rs file compiles as its own
// crate, so a plain `use` cannot reach items defined in a sibling test
// binary. The shared test-support module is included explicitly instead —
// see the module-boundary comment in tests/common/mod.rs.
mod common;

use common::fixture_discovery::discover_all_fixture_infos_result;
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

/// The shared fixture-discovery API (`tests/common/fixture_discovery.rs`) must
/// enumerate this suite's fixture: the hardcoded [`FIXTURE`] path above is only
/// smoke-testable if discovery still reaches that file. Guards against the
/// fixture drifting out of the discovered tree (a rename, or a move under a
/// symlinked *directory*, which the discovery symlink guard deliberately
/// drops) and proves cross-binary reuse of the shared module from a second
/// integration binary.
#[test]
fn smoke_fixture_is_enumerated_by_shared_discovery() {
    let infos = discover_all_fixture_infos_result()
        .expect("shared fixture discovery must enumerate the fixtures tree");
    println!(
        "shared discovery enumerated {} fixtures for CLI invocation",
        infos.len()
    );

    // Discovery returns canonical paths; canonicalize the relative FIXTURE
    // constant the same way before comparing.
    let smoke = std::fs::canonicalize(FIXTURE).expect("smoke fixture must exist");
    let info = infos
        .iter()
        .find(|info| info.path == smoke)
        .unwrap_or_else(|| panic!("shared discovery must enumerate the smoke fixture {smoke:?}"));

    // Derived metadata: name is the file stem, description is non-empty prose.
    assert_eq!(info.name, "test-minimal");
    assert!(
        !info.description.is_empty(),
        "description must be derived for the smoke fixture: {info}"
    );
}
