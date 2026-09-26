//! Contract assertions for the SDK search result consumed by language bindings.

use std::fs;
use std::path::{Path, PathBuf};

use pdftract_core::sdk;
use serde_json::{json, Value};

const SEARCH_TOKEN: &str = "PYTHON_SEARCH_REGRESSION";
const ABSENT_TOKEN: &str = "PYTHON_SEARCH_REGRESSION_ABSENT";
const EXPECTED_BBOX: [f64; 4] = [72.0, 720.0, 244.8000030517578, 732.0];

fn workspace_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .to_path_buf()
}

#[test]
fn sdk_search_preserves_public_match_shape() {
    let fixture = workspace_root().join("crates/pdftract-py/tests/fixtures/search_regression.pdf");
    let expected_path = workspace_root()
        .join("crates/pdftract-py/tests/fixtures/search_regression.expected.json");
    assert!(
        fixture.is_file(),
        "search fixture is missing: {}",
        fixture.display()
    );
    assert!(
        expected_path.is_file(),
        "search expectation is missing: {}",
        expected_path.display()
    );

    let matches = sdk::search(&fixture, SEARCH_TOKEN, false, false, false)
        .expect("searching the deterministic fixture should succeed");
    assert!(
        !matches.is_empty(),
        "the known fixture token must produce a match"
    );

    // These fields are the stable shape projected by the Python binding. Keep
    // the assertions explicit so a native change cannot silently drop or
    // rename a field that public SDK consumers rely on.
    let first = &matches[0];
    assert_eq!(
        first.page_index, 0,
        "the token is on the fixture's only page"
    );
    assert_eq!(first.span_index, 0, "the token is the fixture's first span");
    assert_eq!(first.text, SEARCH_TOKEN);
    assert_eq!(first.bbox, EXPECTED_BBOX);

    let [x0, y0, x1, y1] = first.bbox;
    assert!(x0.is_finite() && y0.is_finite() && x1.is_finite() && y1.is_finite());
    assert!(
        x0 < x1 && y0 < y1,
        "bbox must have positive width and height"
    );

    let absent_matches = sdk::search(&fixture, ABSENT_TOKEN, false, false, false)
        .expect("searching for an absent token should succeed");
    assert!(
        absent_matches.is_empty(),
        "an absent token must not produce any matches"
    );

    let expected: Value = serde_json::from_str(
        &fs::read_to_string(&expected_path).expect("read deterministic search expectation"),
    )
    .expect("parse deterministic search expectation");
    let sdk_matches: Vec<Value> = matches
        .iter()
        .map(|search_match| {
            json!({
                "page_index": search_match.page_index,
                "span_index": search_match.span_index,
                "text": search_match.text,
                "bbox": search_match.bbox,
            })
        })
        .collect();
    assert_eq!(expected["pattern"], SEARCH_TOKEN);
    assert_eq!(expected["matches"], Value::Array(sdk_matches));
}
