use std::path::PathBuf;

use pdftract_core::extract::{result_to_json, ExtractionMetadata, ExtractionResult};
use pdftract_core::options::ReceiptsMode;
use pdftract_core::output::json::result_to_output;
use pdftract_core::output::ndjson::{self, FooterFrame, HeaderFrame, NdjsonFrame, PageFrame};
use pdftract_core::schema::SpanJson;
use pdftract_core::{extract_pdf, extract_pdf_ndjson, ExtractionOptions, PageResult};
use serde_json::Value;

fn vector_fixture() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/test-minimal.pdf")
}

fn assert_span_contract(value: &Value, expected: &SpanJson) {
    assert_eq!(value["text"], expected.text);

    let bbox = value["bbox"]
        .as_array()
        .expect("provenance span must include a bbox");
    assert_eq!(bbox.len(), 4);
    for coordinate in bbox {
        assert!(
            coordinate.as_f64().is_some_and(f64::is_finite),
            "bbox coordinates must be finite: {bbox:?}"
        );
    }
    assert!(
        bbox[0].as_f64().unwrap() < bbox[2].as_f64().unwrap()
            && bbox[1].as_f64().unwrap() < bbox[3].as_f64().unwrap(),
        "bbox must have positive area: {bbox:?}"
    );
    assert_eq!(value["bbox"], serde_json::json!(expected.bbox));

    let confidence = value["confidence"]
        .as_f64()
        .expect("provenance span must include confidence");
    assert!(
        (0.0..=1.0).contains(&confidence),
        "confidence must be normalized: {confidence}"
    );
    let source = value["confidence_source"]
        .as_str()
        .expect("provenance span must include confidence_source");
    assert!(
        matches!(source, "native" | "ocr" | "heuristic"),
        "unexpected documented provenance source: {source}"
    );
    assert_eq!(
        value["confidence_source"],
        serde_json::to_value(&expected.confidence_source).unwrap()
    );
    assert_eq!(value, &serde_json::to_value(expected).unwrap());
}

fn span(text: &str, bbox: [f64; 4], confidence: f64, source: &str) -> SpanJson {
    SpanJson {
        text: text.to_string(),
        bbox,
        font: "ContractFont".to_string(),
        size: 12.0,
        color: Some("#000000".to_string()),
        rendering_mode: Some(0),
        confidence: Some(confidence),
        confidence_source: Some(source.to_string()),
        lang: Some("en".to_string()),
        flags: Vec::new(),
        receipt: None,
        column: None,
    }
}

fn page(index: usize, page_type: &str, spans: Vec<SpanJson>) -> PageResult {
    PageResult {
        index,
        page_number: (index + 1) as u32,
        page_label: None,
        width: Some(612.0),
        height: Some(792.0),
        rotation: Some(0),
        page_type: Some(page_type.to_string()),
        spans,
        blocks: Vec::new(),
        tables: Vec::new(),
        annotations: Vec::new(),
        error: None,
    }
}

fn synthetic_result() -> ExtractionResult {
    let pages = vec![
        page(
            0,
            "text",
            vec![span(
                "vector text",
                [40.0, 700.0, 140.0, 720.0],
                1.0,
                "native",
            )],
        ),
        page(
            1,
            "scanned",
            vec![span("ocr text", [50.0, 600.0, 120.0, 620.0], 0.87, "ocr")],
        ),
        page(
            2,
            "mixed",
            vec![
                span("vector region", [40.0, 500.0, 150.0, 520.0], 1.0, "native"),
                span("ocr region", [300.0, 400.0, 390.0, 420.0], 0.84, "ocr"),
                span(
                    "repaired region",
                    [300.0, 350.0, 430.0, 370.0],
                    0.65,
                    "heuristic",
                ),
            ],
        ),
    ];

    ExtractionResult {
        fingerprint: "provenance-contract".to_string(),
        metadata: ExtractionMetadata {
            page_count: pages.len(),
            receipts_mode: ReceiptsMode::Off,
            span_count: pages.iter().map(|page| page.spans.len()).sum(),
            block_count: 0,
            cache_status: None,
            cache_age_seconds: None,
            error_count: 0,
            reading_order_algorithm: None,
            diagnostics: Vec::new(),
            diagnostics_detailed: Vec::new(),
            profile_name: None,
            profile_version: None,
            profile_fields: None,
        },
        pages,
        signatures: Vec::new(),
        form_fields: Vec::new(),
        links: Vec::new(),
        attachments: Vec::new(),
        threads: Vec::new(),
        javascript_actions: Vec::new(),
    }
}

#[test]
fn vector_provenance_round_trips_across_document_surfaces() {
    let path = vector_fixture();
    let options = ExtractionOptions::default();
    let result = extract_pdf(&path, &options).expect("vector fixture should extract");
    assert_eq!(result.pages.len(), 1);

    let page = &result.pages[0];
    assert_eq!(page.index, 0);
    assert_eq!(page.page_number, 1);
    assert!(!page.spans.is_empty(), "vector fixture must produce a span");
    let expected = &page.spans[0];
    assert_eq!(expected.confidence_source.as_deref(), Some("native"));
    let expected_json = serde_json::to_value(expected).unwrap();
    assert_span_contract(&expected_json, expected);

    let full = serde_json::to_value(result_to_output(&result)).unwrap();
    let full_page = &full["pages"][0];
    assert_eq!(full_page["page_index"], 0);
    assert_eq!(full_page["page_number"], 1);
    assert_eq!(full_page["spans"][0], expected_json);
    assert_span_contract(&full_page["spans"][0], expected);

    let compact = result_to_json(&result);
    assert_eq!(compact["pages"][0]["index"], 0);
    assert_eq!(compact["pages"][0]["spans"][0], expected_json);
    assert_span_contract(&compact["pages"][0]["spans"][0], expected);

    let mut legacy_ndjson = Vec::new();
    extract_pdf_ndjson(&path, &options, &mut legacy_ndjson).expect("legacy NDJSON should extract");
    let legacy_lines: Vec<Value> = String::from_utf8(legacy_ndjson)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("legacy NDJSON line must be JSON"))
        .collect();
    assert_eq!(legacy_lines.len(), 1);
    assert_eq!(legacy_lines[0]["index"], 0);
    assert_eq!(legacy_lines[0]["spans"][0], expected_json);
    assert_span_contract(&legacy_lines[0]["spans"][0], expected);

    let mut framed_ndjson = Vec::new();
    ndjson::extract_streaming(&path, &options, &mut framed_ndjson)
        .expect("framed NDJSON should extract");
    let frame_lines: Vec<Value> = String::from_utf8(framed_ndjson)
        .unwrap()
        .lines()
        .map(|line| serde_json::from_str(line).expect("framed NDJSON line must be JSON"))
        .collect();
    assert_eq!(frame_lines.len(), 3);
    let header: HeaderFrame = serde_json::from_value(frame_lines[0].clone()).unwrap();
    assert_eq!(header.total_pages, 1);
    let page_frame: PageFrame = serde_json::from_value(frame_lines[1].clone()).unwrap();
    assert_eq!(page_frame.page_index, 0);
    assert_eq!(page_frame.spans[0], *expected);
    assert_span_contract(
        &serde_json::to_value(&page_frame.spans[0]).unwrap(),
        expected,
    );
    let footer: FooterFrame = serde_json::from_value(frame_lines[2].clone()).unwrap();
    assert!(footer.errors.is_empty());

    let streamed: Vec<PageResult> = pdftract_core::sdk::extract_stream(&path, &options)
        .unwrap()
        .map(|page| page.expect("SDK stream page should extract"))
        .collect();
    assert_eq!(streamed.len(), 1);
    assert_eq!(streamed[0].index, 0);
    assert_eq!(streamed[0].page_number, 1);
    assert_eq!(streamed[0].spans[0], *expected);

    let matches = pdftract_core::sdk::search(&path, &expected.text, false, false, false)
        .expect("search should extract the vector fixture");
    assert_eq!(matches.len(), 1);
    assert_eq!(matches[0].page_index, 0);
    assert_eq!(matches[0].span_index, 0);
    assert_eq!(matches[0].text, expected.text);
    assert_eq!(matches[0].bbox, expected.bbox);
}

#[test]
fn documented_provenance_modes_preserve_identity_geometry_and_confidence() {
    let result = synthetic_result();
    let full = serde_json::to_value(result_to_output(&result)).unwrap();
    let compact = result_to_json(&result);

    for (index, page) in result.pages.iter().enumerate() {
        let full_page = &full["pages"][index];
        assert_eq!(full_page["page_index"], index);
        assert_eq!(full_page["page_number"], index + 1);
        assert_eq!(full_page["type"], page.page_type.as_deref().unwrap());
        assert_eq!(compact["pages"][index]["index"], index);

        for (span_index, expected) in page.spans.iter().enumerate() {
            assert_span_contract(&full_page["spans"][span_index], expected);
            assert_span_contract(&compact["pages"][index]["spans"][span_index], expected);
        }

        let frame = NdjsonFrame::Page(PageFrame::new(
            page.index,
            page.page_type.clone().unwrap(),
            page.spans.clone(),
            page.blocks.clone(),
            page.tables.clone(),
        ));
        let frame_json = serde_json::to_value(&frame).unwrap();
        assert_eq!(frame_json["frame"], "page");
        assert_eq!(frame_json["page_index"], index);
        for (span_index, expected) in page.spans.iter().enumerate() {
            assert_span_contract(&frame_json["spans"][span_index], expected);
        }
        let decoded: NdjsonFrame = serde_json::from_value(frame_json).unwrap();
        assert_eq!(decoded, frame);
    }
}

#[cfg(feature = "ocr")]
mod hybrid_contract {
    use super::*;
    use image::GrayImage;
    use pdftract_core::classify::{CellIndex, PageClassification};
    use pdftract_core::hybrid::{process_hybrid_page, HybridSpan, HybridSpanSource, OcrCallback};
    use std::collections::BTreeSet;

    struct ContractOcr;

    impl OcrCallback for ContractOcr {
        fn ocr_cell(
            &self,
            _cell_image: &GrayImage,
            _cell: CellIndex,
            _dpi: u32,
        ) -> Result<Vec<HybridSpan>, String> {
            Ok(vec![HybridSpan::ocr(
                [420.0, 100.0, 500.0, 120.0],
                0.84,
                "OCR body".to_string(),
            )])
        }
    }

    #[test]
    fn hybrid_routing_keeps_vector_and_ocr_provenance() {
        let mut scanned_cells = BTreeSet::new();
        scanned_cells.insert(CellIndex::new(7, 7).flat());
        let classification = PageClassification::hybrid(0.92, scanned_cells);
        let vector = HybridSpan::vector(
            [40.0, 700.0, 180.0, 720.0],
            0.99,
            "Vector header".to_string(),
        );

        let merged = process_hybrid_page(
            &GrayImage::new(612, 792),
            612.0,
            792.0,
            &classification,
            std::slice::from_ref(&vector),
            72,
            &ContractOcr,
        );

        let vector_span = merged
            .iter()
            .find(|span| span.source == HybridSpanSource::Vector)
            .expect("hybrid output must retain the vector span");
        assert_eq!(vector_span.text, "Vector header");
        assert_eq!(vector_span.bbox, vector.bbox);
        assert!((0.0..=1.0).contains(&vector_span.confidence));

        let ocr_span = merged
            .iter()
            .find(|span| span.source == HybridSpanSource::Ocr)
            .expect("hybrid output must retain the OCR span");
        assert_eq!(ocr_span.text, "OCR body");
        assert_eq!(ocr_span.bbox, [420.0, 100.0, 500.0, 120.0]);
        assert!((0.0..=1.0).contains(&ocr_span.confidence));
    }
}
