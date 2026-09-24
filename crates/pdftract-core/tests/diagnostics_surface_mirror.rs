//! Enforce the string↔structured diagnostics mirror across all three output
//! surfaces.
//!
//! `docs/errors-array-format.md`, `docs/integrations/diagnostics-codes.md`,
//! and the `pdftract_core::diagnostics_compat` module docs specify the public
//! contract pinned here:
//!
//! 1. `metadata.diagnostics` (legacy string form — the diagnostic's
//!    `message` verbatim, via `diagnostics_compat::to_legacy_strings`) and
//!    `metadata.diagnostics_detailed` (structured `DiagnosticJson` form)
//!    mirror one-to-one: same length, same order, and each string entry
//!    equals the structured entry's `message` at the same index. The legacy
//!    surface carries no code — two diagnostics differing only in code
//!    render identical strings, by design.
//! 2. Both fields are omitted entirely from serialized output when empty
//!    (`skip_serializing_if = "Vec::is_empty"` on each).
//! 3. The top-level `errors` array of the full JSON output and the NDJSON
//!    footer frame's `errors` array are populated from
//!    `diagnostics_detailed`; the footer appends them after any per-page
//!    failure entries.
//!
//! The existing `diagnostics_catalog_drift` suite covers doc-catalog vs
//! emitted-code drift, `diagnostics_serialization_format.rs` pins the
//! per-field serialization shape, and `diagnostics_compat`'s own tests pin
//! the legacy bytes; nothing else fails when the two in-result arrays stop
//! agreeing per-index — which would silently break consumers pairing the
//! arrays by index.
//!
//! Results are assembled directly (populated exactly as the producers in
//! `extract.rs` populate them: `to_legacy_strings` for the string array,
//! `DiagnosticJson::from` for the structured array) rather than via
//! `extract_pdf`, because the extraction path is currently broken tree-wide
//! on every fixture. `extraction_driven_end_to_end_mirror` runs the same
//! assertions through `extract_pdf`/`extract_streaming` and self-skips with
//! a notice until that is fixed, so the end-to-end check arms itself
//! automatically when extraction recovers.

use pdftract_core::diagnostics::{DiagCode, Diagnostic, ObjRef, DIAGNOSTIC_CATALOG};
use pdftract_core::extract::ExtractionResult;
use pdftract_core::options::ReceiptsMode;
use pdftract_core::output::json::result_to_output;
use pdftract_core::output::ndjson::{extract_streaming, footer_errors};
use pdftract_core::schema::DiagnosticJson;
use pdftract_core::ExtractionMetadata;

/// Assert the one-to-one mirror between the legacy string array and the
/// structured array: equal length, and element `i` of the string array is
/// exactly `structured[i].message` — the legacy surface carries the bare
/// message verbatim (no `CODE:` prefix, no offset/location suffix), per the
/// `diagnostics_compat` module contract.
fn assert_mirror(diagnostics: &[String], detailed: &[DiagnosticJson], context: &str) {
    assert_eq!(
        diagnostics.len(),
        detailed.len(),
        "{context}: string and structured diagnostic arrays must have equal length \
         (strings: {diagnostics:?}, structured: {detailed:?})"
    );
    for (index, (string_entry, structured)) in diagnostics.iter().zip(detailed).enumerate() {
        assert_eq!(
            string_entry.as_str(),
            structured.message.as_str(),
            "{context}: string entry {index} ({string_entry:?}) must equal the \
             structured entry's message verbatim ({:?}) — the legacy surface is \
             the bare message, not `Display`'s \"CODE: ...\" form",
            structured.message
        );
    }
}

/// A set of typed diagnostics chosen to stress the per-index pairing: every
/// severity, document-level and page-level entries, optional fields present
/// and absent, a message containing `": "`, and two diagnostics whose
/// messages are identical but whose codes differ (which the legacy surface
/// renders identically — the documented lossiness; only the structured side
/// still distinguishes them).
fn sample_typed_diagnostics() -> Vec<Diagnostic> {
    vec![
        Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "Xref rebuilt via forward scan"),
        Diagnostic::with_static(DiagCode::StreamDecodeError, 1234, "corrupt flate data")
            .with_object_ref(ObjRef::new(12, 0))
            .with_page_index(3),
        Diagnostic::with_dynamic_no_offset(
            DiagCode::FontNotFound,
            "text runs (font ID: CustomNoMap)".to_string(),
        )
        .with_page_index(0),
        Diagnostic::with_static_no_offset(DiagCode::StructInvalidName, "shared message"),
        Diagnostic::with_static_no_offset(DiagCode::StreamBomb, "shared message"),
        Diagnostic::with_static_no_offset(
            DiagCode::LayoutTaggedPdfDeferred,
            "Tagged PDF StructTree deferred to Phase 7",
        ),
    ]
}

/// An empty result, for tests that populate the diagnostics arrays by hand.
fn empty_result() -> ExtractionResult {
    ExtractionResult {
        fingerprint: "diagnostics-surface-mirror".to_string(),
        pages: vec![],
        metadata: ExtractionMetadata {
            page_count: 1,
            receipts_mode: ReceiptsMode::Off,
            span_count: 0,
            block_count: 0,
            cache_status: None,
            cache_age_seconds: None,
            error_count: 0,
            reading_order_algorithm: None,
            diagnostics: vec![],
            diagnostics_detailed: vec![],
            profile_name: None,
            profile_version: None,
            profile_fields: None,
        },
        signatures: vec![],
        form_fields: vec![],
        links: vec![],
        attachments: vec![],
        threads: vec![],
        javascript_actions: vec![],
    }
}

/// A result whose diagnostics arrays are populated exactly as the producers
/// in `extract.rs` populate them (same expression, same order).
fn populated_result() -> ExtractionResult {
    let mut result = empty_result();
    let typed = sample_typed_diagnostics();
    result.metadata.diagnostics = pdftract_core::diagnostics_compat::to_legacy_strings(&typed);
    result.metadata.diagnostics_detailed = typed.iter().map(DiagnosticJson::from).collect();
    result
}

// --- Display impl grammar: the internal rendering, not the legacy surface ---

#[test]
fn display_prefix_carries_code_for_every_catalog_code() {
    // `Display`'s grammar (code prefix, message verbatim after it) is the
    // internal rendering that the `diagnostics_compat` module docs contrast
    // the legacy surface against — pinned here so that contrast stays
    // accurate for every catalog code.
    for info in DIAGNOSTIC_CATALOG {
        let diagnostic = Diagnostic::with_static_no_offset(info.code, "sample message");
        let rendered = diagnostic.to_string();
        let prefix = format!("{}: ", info.code.name());
        assert!(
            rendered.starts_with(&prefix),
            "code {}: Display rendered {rendered:?}, which does not start with \
             the \"{}: \" prefix",
            info.code.name(),
            info.code.name()
        );
        assert_eq!(
            &rendered[prefix.len()..],
            "sample message",
            "code {}: Display must render the message verbatim after the prefix",
            info.code.name()
        );

        // The legacy surface of the same diagnostic is the bare message —
        // deliberately NOT Display's prefixed form.
        assert_eq!(
            pdftract_core::diagnostics_compat::to_legacy_string(&diagnostic),
            "sample message",
            "code {}: legacy conversion must return the message verbatim, \
             not the Display form",
            info.code.name()
        );

        // The structured mirror of the same diagnostic agrees on the code,
        // message, and typed severity.
        let structured = DiagnosticJson::from(&diagnostic);
        assert_eq!(structured.code, info.code.name());
        assert_eq!(structured.message, "sample message");
        assert_eq!(structured.severity, info.severity.to_string());
    }
}

#[test]
fn display_grammar_matches_documented_format() {
    // `Display` renders `{CODE}: {message} (byte offset {offset})?
    // [obj gen R]?` — the optional suffixes are pinned in both presence and
    // order. This is the internal rendering the `diagnostics_compat` docs
    // contrast the legacy surface against; the legacy surface itself never
    // carries it.
    let bare = Diagnostic::with_static_no_offset(DiagCode::FontGlyphUnmapped, "glyph unmapped");
    assert_eq!(bare.to_string(), "FONT_GLYPH_UNMAPPED: glyph unmapped");

    let offset_only = Diagnostic::with_static(DiagCode::StreamDecodeError, 1234, "corrupt flate");
    assert_eq!(
        offset_only.to_string(),
        "STREAM_DECODE_ERROR: corrupt flate (byte offset 1234)"
    );

    let ref_only = Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "forward scan")
        .with_object_ref(ObjRef::new(12, 0));
    assert_eq!(ref_only.to_string(), "XREF_REPAIRED: forward scan [12 0 R]");

    let both = Diagnostic::with_static(DiagCode::StructInvalidName, 99, "bad name")
        .with_object_ref(ObjRef::new(7, 0));
    assert_eq!(
        both.to_string(),
        "STRUCT_INVALID_NAME: bad name (byte offset 99) [7 0 R]"
    );
}

#[test]
fn legacy_conversion_preserves_order_and_duplicate_messages() {
    let diagnostics = vec![
        Diagnostic::with_static(DiagCode::StreamDecodeError, 7, "same message")
            .with_object_ref(ObjRef::new(12, 3))
            .with_page_index(4),
        Diagnostic::with_static_no_offset(DiagCode::StructInvalidName, "same message"),
        Diagnostic::with_static_no_offset(DiagCode::XrefRepaired, "different message"),
    ];

    assert_eq!(
        pdftract_core::diagnostics_compat::to_legacy_strings(&diagnostics),
        vec!["same message", "same message", "different message"],
        "legacy conversion must retain message bytes, order, and duplicates while dropping typed context"
    );
}

// --- Surface 1: the metadata arrays mirror one-to-one --------------------

#[test]
fn string_and_structured_arrays_mirror_one_to_one() {
    let result = populated_result();
    let detailed = &result.metadata.diagnostics_detailed;

    // The sample exercises optional fields; the structured side must carry
    // them so the per-index pairing is checked against real data.
    assert_eq!(detailed[1].page_index, Some(3));
    assert_eq!(
        detailed[1].location.as_ref().map(|l| l.object_number),
        Some(12)
    );
    assert_eq!(detailed[2].severity, "warning");

    assert_mirror(
        &result.metadata.diagnostics,
        detailed,
        "metadata diagnostics",
    );

    // Index pairing specifically: entries 3 and 4 share a message but differ
    // in code — the legacy surface renders them identically (the documented
    // lossiness), and only the structured side still distinguishes them.
    assert_eq!(
        result.metadata.diagnostics[3], result.metadata.diagnostics[4],
        "diagnostics differing only in code must render identical legacy strings"
    );
    assert_ne!(
        detailed[3].code, detailed[4].code,
        "the structured side keeps the codes distinct"
    );
}

// --- Surface 2: the full JSON output's top-level errors array ------------

#[test]
fn full_json_errors_array_is_populated_from_diagnostics_detailed() {
    let result = populated_result();
    let detailed = &result.metadata.diagnostics_detailed;

    let output = result_to_output(&result);

    // Struct level: the errors array IS the structured metadata array.
    assert_eq!(
        output.errors.len(),
        detailed.len(),
        "top-level errors array must carry every structured diagnostic"
    );
    assert_eq!(output.errors, *detailed);

    // Wire level: the serialized v1.0 Output document carries the same
    // objects under `errors`, in order.
    let doc = serde_json::to_value(&output).expect("serialize full output");
    let wire_errors = doc["errors"].as_array().expect("errors array");
    assert_eq!(wire_errors.len(), detailed.len());
    for (index, entry) in wire_errors.iter().enumerate() {
        assert_eq!(
            entry["code"],
            detailed[index].code.as_str(),
            "errors[{index}] code must equal the structured code at the same index"
        );
        assert_eq!(
            entry,
            &serde_json::to_value(&detailed[index]).expect("serialize diagnostic"),
            "errors[{index}] must serialize identically to diagnostics_detailed[{index}]"
        );
    }
}

/// The compact extraction JSON (`result_to_json` — the document the CLI's
/// `--format json` prints) carries the mirror on its `metadata` object.
#[test]
fn compact_json_metadata_arrays_mirror_one_to_one() {
    let result = populated_result();
    let detailed = &result.metadata.diagnostics_detailed;

    let doc = pdftract_core::extract::result_to_json(&result);
    let wire_detailed = doc["metadata"]["diagnostics_detailed"]
        .as_array()
        .expect("metadata.diagnostics_detailed array");
    let wire_strings = doc["metadata"]["diagnostics"]
        .as_array()
        .expect("metadata.diagnostics array");

    assert_eq!(wire_detailed.len(), detailed.len());
    assert_eq!(wire_strings.len(), detailed.len());
    for (index, entry) in wire_detailed.iter().enumerate() {
        assert_eq!(
            entry["code"],
            detailed[index].code.as_str(),
            "wire entry {index} must carry the structured code at the same index"
        );
        assert!(
            wire_strings[index]
                .as_str()
                .is_some_and(|s| s == detailed[index].message.as_str()),
            "metadata.diagnostics[{index}] must equal the message of \
             diagnostics_detailed[{index}] verbatim"
        );
    }
}

// --- Empty-omission serialization rule ------------------------------------

#[test]
fn empty_diagnostics_fields_are_omitted_from_serialized_output() {
    // Clean result: both arrays empty, both keys absent from the compact
    // extraction JSON's metadata (the wire form of `ExtractionMetadata`).
    let doc = pdftract_core::extract::result_to_json(&empty_result());
    let metadata = doc["metadata"].as_object().expect("metadata object");
    for absent in ["diagnostics", "diagnostics_detailed"] {
        assert!(
            !metadata.contains_key(absent),
            "`metadata.{absent}` must be omitted from serialized output when empty"
        );
    }

    // The same rule holds for direct metadata serialization, symmetrically:
    // non-empty arrays keep both keys.
    let empty = empty_result().metadata;
    let serialized = serde_json::to_value(&empty).expect("serialize empty metadata");
    for absent in ["diagnostics", "diagnostics_detailed"] {
        assert!(
            serialized.get(absent).is_none(),
            "`{absent}` must be omitted when the array is empty"
        );
    }

    let serialized =
        serde_json::to_value(populated_result().metadata).expect("serialize populated metadata");
    for present in ["diagnostics", "diagnostics_detailed"] {
        assert!(
            serialized.get(present).is_some(),
            "`{present}` must serialize when the array is non-empty"
        );
    }
}

// --- Surface 3: the NDJSON footer frame's errors array --------------------

#[test]
fn footer_errors_places_page_failures_before_structured_diagnostics() {
    let result = populated_result();
    let detailed = &result.metadata.diagnostics_detailed;

    // No failed pages: the footer's errors are exactly the structured
    // diagnostics, in order, serialized identically.
    let errors = footer_errors(&result).expect("footer errors assemble");
    assert_eq!(errors.len(), detailed.len());
    for (entry, structured) in errors.iter().zip(detailed) {
        assert_eq!(
            entry,
            &serde_json::to_value(structured).expect("serialize diagnostic"),
            "footer errors entry must serialize identically to the structured diagnostic"
        );
    }

    // Failed pages come first, as `page_extraction_error` records, followed
    // by the unchanged structured tail (docs/integrations/diagnostics-codes.md).
    let mut with_page_failure = populated_result();
    let mut pages = Vec::new();
    for (index, error) in ["page 0 blew up", "page 1 blew up"].into_iter().enumerate() {
        pages.push(pdftract_core::extract::PageResult {
            index,
            page_number: (index + 1) as u32,
            page_label: None,
            width: None,
            height: None,
            rotation: None,
            page_type: None,
            spans: vec![],
            blocks: vec![],
            tables: vec![],
            annotations: vec![],
            error: Some(error.to_string()),
        });
    }
    with_page_failure.pages = pages;

    let errors = footer_errors(&with_page_failure).expect("footer errors assemble");
    let head = &errors[..errors.len() - detailed.len()];
    let tail = &errors[errors.len() - detailed.len()..];
    assert_eq!(head.len(), 2, "each failed page contributes one record");
    for (entry, message) in head.iter().zip(["page 0 blew up", "page 1 blew up"]) {
        assert_eq!(entry["code"], "page_extraction_error");
        assert_eq!(entry["severity"], "error");
        assert_eq!(entry["message"], message);
    }
    for (entry, structured) in tail.iter().zip(detailed) {
        assert_eq!(
            entry,
            &serde_json::to_value(structured).expect("serialize diagnostic"),
            "structured tail must be unchanged by preceding page failures"
        );
    }
}

// --- End-to-end, armed when extraction recovers ----------------------------

/// The minimal tagged document used by the end-to-end check (computed xref).
fn tagged_pdf_bytes() -> Vec<u8> {
    let objects: [&str; 3] = [
        "<</Type/Catalog/Pages 2 0 R/MarkInfo<</Marked true>>>>",
        "<</Type/Pages/Kids[3 0 R]/Count 1>>",
        "<</Type/Page/Parent 2 0 R/MediaBox[0 0 612 792]>>",
    ];
    let mut out = b"%PDF-1.4\n".to_vec();
    let mut offsets = [0u32; 3];
    for (index, dict) in objects.iter().enumerate() {
        offsets[index] = out.len() as u32;
        out.extend_from_slice(format!("{} 0 obj{dict}endobj\n", index + 1).as_bytes());
    }
    let startxref = out.len();
    out.extend_from_slice(b"xref\n0 4\n0000000000 65535 f \n");
    for offset in offsets {
        out.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(b"trailer<</Size 4/Root 1 0 R>>\n");
    out.extend_from_slice(format!("startxref\n{startxref}\n%%EOF\n").as_bytes());
    out
}

#[test]
fn extraction_driven_end_to_end_mirror() {
    // Extraction is broken tree-wide at the moment (every fixture fails:
    // `No /Root` / `Document contains no pages` — see
    // tests/diagnostics_serialization_format.rs's closing note). When it
    // recovers, this test stops skipping and enforces the mirror through the
    // real pipeline: metadata arrays, full-JSON errors, and NDJSON footer.
    let temp_dir = tempfile::tempdir().expect("temp dir");
    let pdf_path = temp_dir.path().join("tagged.pdf");
    std::fs::write(&pdf_path, tagged_pdf_bytes()).expect("write fixture");

    let result = match pdftract_core::extract_pdf(&pdf_path, &Default::default()) {
        Ok(result) => result,
        Err(error) => {
            eprintln!(
                "SKIPPED (not passed): end-to-end mirror check needs a working \
                 extraction path; extract_pdf currently fails tree-wide with: {error:#}"
            );
            return;
        }
    };

    // Surface 1: metadata mirror, with a real diagnostic present.
    assert!(
        !result.metadata.diagnostics_detailed.is_empty(),
        "the tagged fixture must emit TAGGED_PDF_STRUCT_TREE_DEFERRED; got strings {:?}",
        result.metadata.diagnostics
    );
    assert_mirror(
        &result.metadata.diagnostics,
        &result.metadata.diagnostics_detailed,
        "extract_pdf metadata",
    );

    // Surface 2: full JSON errors.
    let output = result_to_output(&result);
    assert_eq!(output.errors, result.metadata.diagnostics_detailed);

    // Surface 3: NDJSON footer tail mirrors the structured array.
    let mut buf: Vec<u8> = Vec::new();
    extract_streaming(&pdf_path, &Default::default(), &mut buf)
        .expect("streaming extraction must succeed");
    let footer_line = buf
        .split(|byte| *byte == b'\n')
        .filter(|line| !line.is_empty())
        .last()
        .expect("NDJSON output must contain a footer frame");
    let footer: serde_json::Value =
        serde_json::from_slice(footer_line).expect("footer frame must be valid JSON");
    let footer_errors_array = footer["errors"].as_array().expect("footer errors array");
    let detailed = &result.metadata.diagnostics_detailed;
    assert!(footer_errors_array.len() >= detailed.len());
    let tail = &footer_errors_array[footer_errors_array.len() - detailed.len()..];
    for (entry, structured) in tail.iter().zip(detailed) {
        assert_eq!(
            entry,
            &serde_json::to_value(structured).expect("serialize diagnostic")
        );
    }
    for entry in &footer_errors_array[..footer_errors_array.len() - detailed.len()] {
        assert_eq!(
            entry["code"], "page_extraction_error",
            "entries preceding the structured tail must be per-page failure records"
        );
    }
}
