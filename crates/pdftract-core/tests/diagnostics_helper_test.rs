//! Unit tests for the shared diagnostics test helper
//! (`tests/test_helpers/diagnostics.rs`, bead bf-53knlx).
//!
//! Every helper contract is pinned here: extraction from all three real
//! output shapes, the missing/null-is-empty rule, the malformed-carrier
//! errors, and the search functions other tests will rely on.

mod test_helpers;

use serde_json::{json, Value};
use test_helpers::diagnostics::{
    any_code_contains, codes, contains_code, count_by_code, diagnostics, diagnostics_from_str,
    find_by_code, Diagnostic, DiagnosticsError,
};

/// The real zero-diagnostic capture shape: a schema-1.0 document whose JSON
/// has no `errors` key at all (mirrors `REAL_CAPTURE` in `output::unmapped`
/// tests — the documented empty-extraction output).
const ZERO_DIAGNOSTIC_CAPTURE: &str = r#"{
  "schema_version": "1.0",
  "metadata": {
    "page_count": 0,
    "cache_status": "skipped",
    "span_count": 0,
    "block_count": 0
  },
  "pages": [],
  "threads": []
}"#;

// ---- valid inputs: the three real diagnostics shapes ----

#[test]
fn extracts_object_form_errors_array() {
    let parsed = json!({
        "schema_version": "1.0",
        "errors": [
            {
                "code": "FONT_GLYPH_UNMAPPED",
                "message": "Character code 01 could not be resolved to Unicode (font ID: CustomNoMap)",
                "severity": "warning",
                "page_index": 0
            },
            {
                "code": "STREAM_DECODE_ERROR",
                "message": "truncated FlateDecode stream",
                "severity": "error"
            }
        ]
    });

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert_eq!(
        diags,
        vec![
            Diagnostic {
                code: "FONT_GLYPH_UNMAPPED".to_owned(),
                message: Some(
                    "Character code 01 could not be resolved to Unicode (font ID: CustomNoMap)"
                        .to_owned()
                ),
                severity: Some("warning".to_owned()),
                page_index: Some(0),
            },
            Diagnostic {
                code: "STREAM_DECODE_ERROR".to_owned(),
                message: Some("truncated FlateDecode stream".to_owned()),
                severity: Some("error".to_owned()),
                page_index: None,
            },
        ],
        "Expected every `errors` entry converted with code, message, severity, page_index."
    );
}

#[test]
fn extracts_string_form_metadata_diagnostics() {
    let parsed = json!({
        "metadata": {
            "page_count": 1,
            "diagnostics": [
                "Character code 01 could not be resolved to Unicode (font ID: CustomNoMap)"
            ]
        }
    });

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert_eq!(diags.len(), 1, "Expected the one string entry.");
    assert_eq!(
        diags[0].code, "Character code 01 could not be resolved to Unicode (font ID: CustomNoMap)",
        "Expected string-form `code` to be the entire message — extract.rs pushes bare \
         messages with no code prefix, so any first-colon split would mangle them."
    );
    assert_eq!(
        diags[0].message, None,
        "String-form entries carry no separate message."
    );
}

#[test]
fn extracts_string_form_top_level_diagnostics() {
    // NDJSON page-frame shape: diagnostics directly on the frame object.
    let parsed = json!({
        "frame": "page",
        "page_index": 2,
        "diagnostics": ["STRUCT_MISSING_KEY: xref entry 7", "plain warning"]
    });

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert_eq!(
        codes(&diags),
        vec!["STRUCT_MISSING_KEY: xref entry 7", "plain warning"],
        "Expected each top-level string entry preserved verbatim and in order."
    );
}

#[test]
fn errors_location_wins_when_multiple_present() {
    let parsed = json!({
        "errors": [{"code": "A_FROM_ERRORS"}],
        "metadata": {"diagnostics": ["from metadata"]},
        "diagnostics": ["from top level"]
    });

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert_eq!(
        codes(&diags),
        vec!["A_FROM_ERRORS"],
        "Expected the first present location (`errors`) to win so extraction is deterministic."
    );
}

#[test]
fn metadata_null_falls_through_to_top_level() {
    let parsed = json!({
        "metadata": null,
        "diagnostics": ["recovered"]
    });

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert_eq!(
        codes(&diags),
        vec!["recovered"],
        "Expected `metadata: null` to be skipped, not treated as an error or a terminator."
    );
}

// ---- missing / null fields are empty, never errors ----

#[test]
fn missing_diagnostics_fields_yield_empty_array() {
    let parsed = json!({"schema_version": "1.0", "pages": []});

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert!(
        diags.is_empty(),
        "Expected an empty array for output with no diagnostics fields anywhere."
    );
}

#[test]
fn null_errors_field_yields_empty_array() {
    let parsed = json!({"errors": null});

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert!(
        diags.is_empty(),
        "Expected `errors: null` to be treated as absent, matching `output::unmapped::scan_value`."
    );
}

#[test]
fn real_zero_diagnostic_capture_yields_empty_array() {
    let parsed: Value = serde_json::from_str(ZERO_DIAGNOSTIC_CAPTURE)
        .unwrap_or_else(|e| panic!("fixture JSON must parse: {e}"));

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert!(
        diags.is_empty(),
        "The documented real-world zero-diagnostic capture (no `errors` key at all) must \
         extract as empty, not error."
    );
}

// ---- malformed data is a typed error ----

#[test]
fn non_object_root_is_an_error() {
    let parsed = json!["not an object"];

    assert_eq!(
        diagnostics(&parsed),
        Err(DiagnosticsError::RootNotAnObject {
            found: "string".to_owned()
        }),
        "Expected a typed error naming the found JSON type when the root is not an object."
    );

    let parsed = json!([1, 2]);

    assert_eq!(
        diagnostics(&parsed),
        Err(DiagnosticsError::RootNotAnObject {
            found: "array".to_owned()
        }),
        "Expected the same typed error for an array root."
    );
}

#[test]
fn non_array_errors_field_is_an_error() {
    let parsed = json!({"errors": "oops"});

    assert_eq!(
        diagnostics(&parsed),
        Err(DiagnosticsError::FieldNotAnArray {
            path: "errors".to_owned(),
            found: "string".to_owned()
        }),
        "Expected a present-but-non-array `errors` field to be reported as malformed."
    );
}

#[test]
fn non_array_metadata_diagnostics_is_an_error() {
    let parsed = json!({"metadata": {"diagnostics": 42}});

    assert_eq!(
        diagnostics(&parsed),
        Err(DiagnosticsError::FieldNotAnArray {
            path: "metadata.diagnostics".to_owned(),
            found: "number".to_owned()
        }),
        "Expected a non-array `metadata.diagnostics` to be reported with its full path."
    );
}

#[test]
fn non_object_errors_entry_is_an_error() {
    let parsed = json!({"errors": ["not an object"]});

    assert_eq!(
        diagnostics(&parsed),
        Err(DiagnosticsError::EntryNotAnObject { index: 0 }),
        "Expected an `errors` entry that is not an object to be reported with its index."
    );
}

#[test]
fn missing_code_in_errors_entry_is_an_error() {
    let parsed = json!({"errors": [{"message": "no code here"}]});

    assert_eq!(
        diagnostics(&parsed),
        Err(DiagnosticsError::CodeMissing { index: 0 }),
        "Expected an `errors` entry without a `code` to be malformed — `code` is the \
         field every search helper depends on."
    );
}

#[test]
fn non_string_code_in_errors_entry_is_an_error() {
    let parsed = json!({"errors": [{"code": 7}]});

    assert_eq!(
        diagnostics(&parsed),
        Err(DiagnosticsError::CodeNotAString {
            index: 0,
            found: "number".to_owned()
        }),
        "Expected a non-string `code` to be reported with its index and JSON type."
    );
}

#[test]
fn non_string_string_form_entry_is_an_error() {
    let parsed = json!({"diagnostics": ["ok", 5]});

    assert_eq!(
        diagnostics(&parsed),
        Err(DiagnosticsError::EntryNotAString {
            path: "diagnostics".to_owned(),
            index: 1
        }),
        "Expected a non-string entry inside a string-form array to be reported by \
         path and index."
    );
}

#[test]
fn optional_fields_tolerate_missing_null_and_wrong_types() {
    // The `output::unmapped` tolerance policy: metadata fields that are
    // missing, null, or wrongly typed stay None and never malform the entry.
    let parsed = json!({
        "errors": [
            {"code": "OK"},
            {"code": "NULLS", "message": null, "severity": null, "page_index": null},
            {"code": "WRONG_TYPES", "message": 3, "severity": true, "page_index": "zero"}
        ]
    });

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert_eq!(
        diags,
        vec![
            Diagnostic {
                code: "OK".to_owned(),
                message: None,
                severity: None,
                page_index: None
            },
            Diagnostic {
                code: "NULLS".to_owned(),
                message: None,
                severity: None,
                page_index: None
            },
            Diagnostic {
                code: "WRONG_TYPES".to_owned(),
                message: None,
                severity: None,
                page_index: None
            },
        ],
        "Expected optional-field oddities tolerated as None while every entry stays \
         extractable."
    );
}

// ---- search / iteration helpers ----

#[test]
fn find_by_code_iterates_over_all_matches() {
    let diags = vec![
        Diagnostic {
            code: "A".to_owned(),
            message: Some("first".to_owned()),
            severity: None,
            page_index: Some(0),
        },
        Diagnostic {
            code: "B".to_owned(),
            message: None,
            severity: None,
            page_index: None,
        },
        Diagnostic {
            code: "A".to_owned(),
            message: Some("second".to_owned()),
            severity: None,
            page_index: Some(3),
        },
    ];

    let messages: Vec<&str> = find_by_code(&diags, "A")
        .map(|d| d.message.as_deref().unwrap_or_default())
        .collect();

    assert_eq!(
        messages,
        vec!["first", "second"],
        "Expected iteration over every exact-code match, in array order."
    );
}

#[test]
fn contains_and_count_by_code() {
    let diags = vec![
        Diagnostic {
            code: "FONT_GLYPH_UNMAPPED".to_owned(),
            message: None,
            severity: None,
            page_index: None,
        },
        Diagnostic {
            code: "OTHER".to_owned(),
            message: None,
            severity: None,
            page_index: None,
        },
        Diagnostic {
            code: "FONT_GLYPH_UNMAPPED".to_owned(),
            message: None,
            severity: None,
            page_index: None,
        },
    ];

    assert!(
        contains_code(&diags, "FONT_GLYPH_UNMAPPED"),
        "Expected exact-code presence check to hit."
    );
    assert!(
        !contains_code(&diags, "GLYPH_UNMAPPED"),
        "Expected the exact-code check NOT to substring-match — `any_code_contains` is \
         the substring search."
    );
    assert_eq!(
        count_by_code(&diags, "FONT_GLYPH_UNMAPPED"),
        2,
        "Expected duplicates counted (emit-once-per-glyph regressions show up here)."
    );
}

#[test]
fn substring_search_reaches_string_form_entries() {
    let parsed = json!({
        "metadata": {
            "diagnostics": [
                "Character code 01 could not be resolved to Unicode (font ID: CustomNoMap)"
            ]
        }
    });

    let diags = diagnostics(&parsed).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert!(
        any_code_contains(&diags, "could not be resolved to Unicode"),
        "Expected the substring search to reach string-form entries whose `code` is the \
         full bare message."
    );
    assert!(
        !any_code_contains(&diags, "FONT_GLYPH_UNMAPPED"),
        "Expected no false hit: this bare-message form never contains the code token."
    );
}

// ---- text-parsing convenience wrapper ----

#[test]
fn diagnostics_from_str_parses_then_extracts() {
    let text = r#"{"errors": [{"code": "STREAM_DECODE_ERROR", "message": "truncated"}]}"#;

    let diags = diagnostics_from_str(text).unwrap_or_else(|e| panic!("extract failed: {e}"));

    assert_eq!(
        codes(&diags),
        vec!["STREAM_DECODE_ERROR"],
        "Expected parse+extract in one step to behave like the two-step form."
    );
}

#[test]
fn diagnostics_from_str_rejects_invalid_json() {
    let result = diagnostics_from_str("{not json");

    assert!(
        result.is_err(),
        "Expected invalid JSON text to surface the serde error instead of panicking."
    );
}

#[test]
fn error_display_messages_are_actionable() {
    let cases = [
        (
            DiagnosticsError::RootNotAnObject {
                found: "string".to_owned(),
            },
            "found string",
        ),
        (
            DiagnosticsError::FieldNotAnArray {
                path: "errors".to_owned(),
                found: "number".to_owned(),
            },
            "diagnostics field `errors` is not an array (found number)",
        ),
        (DiagnosticsError::EntryNotAnObject { index: 3 }, "index 3"),
        (
            DiagnosticsError::CodeNotAString {
                index: 1,
                found: "boolean".to_owned(),
            },
            "index 1 has a `code` that is not a string (found boolean)",
        ),
    ];

    for (err, expected_fragment) in cases {
        let text = err.to_string();
        assert!(
            text.contains(expected_fragment),
            "Expected Display of {err:?} to contain {expected_fragment:?}, got: {text}"
        );
    }
}
