//! Integration test for comparison mode (Phase 7.9.8).
//!
//! This test verifies that the `--compare` flag works correctly for
//! side-by-side PDF diff viewing in the inspector.

use std::path::PathBuf;

#[test]
fn test_inspect_args_has_compare_field() {
    use pdftract_cli::inspect::args::InspectArgs;

    // Test that InspectArgs can be constructed with compare field
    let args = InspectArgs {
        file: PathBuf::from("test.pdf"),
        port: 7676,
        bind: "127.0.0.1".to_string(),
        auth_token: None,
        no_open: true,
        compare: Some(PathBuf::from("compare.pdf")),
        audit_log: None,
    };

    assert_eq!(args.file, PathBuf::from("test.pdf"));
    assert_eq!(args.compare, Some(PathBuf::from("compare.pdf")));
    assert!(args.no_open);
}

#[test]
fn test_inspect_args_validate_without_compare() {
    use pdftract_cli::inspect::args::InspectArgs;

    let args = InspectArgs {
        file: PathBuf::from("tests/fixtures/minimal.pdf"),
        port: 7676,
        bind: "127.0.0.1".to_string(),
        auth_token: None,
        no_open: true,
        compare: None,
        audit_log: None,
    };

    // Should succeed if file exists
    if args.file.exists() {
        assert!(args.validate().is_ok());
    }
}

#[test]
fn test_diff_summary_serialization() {
    use pdftract_cli::inspect::api::DiffSummary;

    let summary = DiffSummary {
        pages_added: 1,
        pages_removed: 0,
        blocks_added: 5,
        blocks_removed: 2,
        blocks_changed: 3,
        spans_added: 10,
        spans_removed: 4,
        spans_changed: 6,
        reading_order_changed: false,
    };

    let json = serde_json::to_string(&summary).unwrap();
    assert!(json.contains("\"pages_added\":1"));
    assert!(json.contains("\"blocks_added\":5"));
    assert!(json.contains("\"reading_order_changed\":false"));

    // Verify deserialization works
    let deserialized: DiffSummary = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.pages_added, 1);
    assert_eq!(deserialized.blocks_added, 5);
}

#[test]
fn test_page_diff_serialization() {
    use pdftract_cli::inspect::api::PageDiff;

    let diff = PageDiff {
        changed_blocks: vec![0, 2],
        removed_blocks: vec![1],
        added_blocks: vec![3, 4],
        changed_spans: vec![5, 6, 7],
        removed_spans: vec![8],
        added_spans: vec![9, 10],
        reading_order_changed: true,
    };

    let json = serde_json::to_string(&diff).unwrap();
    assert!(json.contains("\"changed_blocks\":[0,2]"));
    assert!(json.contains("\"added_blocks\":[3,4]"));

    // Verify deserialization works
    let deserialized: PageDiff = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.changed_blocks, vec![0, 2]);
    assert_eq!(deserialized.reading_order_changed, true);
}

#[test]
fn test_compare_document_meta_serialization() {
    use pdftract_cli::inspect::api::CompareDocumentMeta;
    use serde_json::json;

    let meta = CompareDocumentMeta {
        a: json!({"pages": []}),
        b: Some(json!({"pages": []})),
        diff_summary: None,
    };

    let json_str = serde_json::to_string(&meta).unwrap();
    assert!(json_str.contains("\"a\":"));
    assert!(json_str.contains("\"b\":"));

    // Verify deserialization works
    let deserialized: CompareDocumentMeta = serde_json::from_str(&json_str).unwrap();
    assert!(deserialized.a.is_object());
    assert!(deserialized.b.is_some());
}

#[test]
fn test_compare_page_data_serialization() {
    use pdftract_cli::inspect::api::{ComparePageData, PageDiff};
    use serde_json::json;

    let page_data = ComparePageData {
        a: Some(json!({"index": 0})),
        b: Some(json!({"index": 0})),
        diff: Some(PageDiff {
            changed_blocks: vec![],
            removed_blocks: vec![],
            added_blocks: vec![],
            changed_spans: vec![],
            removed_spans: vec![],
            added_spans: vec![],
            reading_order_changed: false,
        }),
    };

    let json_str = serde_json::to_string(&page_data).unwrap();
    assert!(json_str.contains("\"a\":"));
    assert!(json_str.contains("\"b\":"));
    assert!(json_str.contains("\"diff\":"));

    // Verify deserialization works
    let deserialized: ComparePageData = serde_json::from_str(&json_str).unwrap();
    assert!(deserialized.a.is_some());
    assert!(deserialized.b.is_some());
    assert!(deserialized.diff.is_some());
}

#[test]
fn test_bbox_overlap_score() {
    use pdftract_cli::inspect::api::bbox_overlap_score;

    // Test identical bounding boxes (score should be 1.0)
    let bbox1 = [100.0, 100.0, 200.0, 200.0];
    let bbox2 = [100.0, 100.0, 200.0, 200.0];
    let score = bbox_overlap_score(&bbox1, &bbox2);
    assert!((score - 1.0).abs() < 0.001);

    // Test non-overlapping boxes (score should be 0.0)
    let bbox3 = [0.0, 0.0, 50.0, 50.0];
    let bbox4 = [100.0, 100.0, 150.0, 150.0];
    let score = bbox_overlap_score(&bbox3, &bbox4);
    assert!((score - 0.0).abs() < 0.001);

    // Test partially overlapping boxes (score should be between 0 and 1)
    let bbox5 = [0.0, 0.0, 100.0, 100.0];
    let bbox6 = [50.0, 50.0, 150.0, 150.0];
    let score = bbox_overlap_score(&bbox5, &bbox6);
    assert!(score > 0.0 && score < 1.0);
}

#[test]
fn test_text_similarity_score() {
    use pdftract_cli::inspect::api::text_similarity_score;

    // Test identical text (score should be 1.0)
    let score = text_similarity_score("hello world", "hello world");
    assert!((score - 1.0).abs() < 0.001);

    // Test completely different text (score should be < 0.5)
    let score = text_similarity_score("abcdef", "xyz123");
    assert!(score < 0.5);

    // Test similar text (score should be > 0.5)
    let score = text_similarity_score("hello world", "hello word");
    assert!(score > 0.5);
}

#[test]
fn test_levenshtein_distance() {
    use pdftract_cli::inspect::api::levenshtein_distance;

    // Test identical strings
    assert_eq!(levenshtein_distance("hello", "hello"), 0);

    // Test completely different strings
    assert_eq!(levenshtein_distance("abc", "xyz"), 3);

    // Test one substitution
    assert_eq!(levenshtein_distance("hello", "hallo"), 1);

    // Test one insertion
    assert_eq!(levenshtein_distance("hello", "hello!"), 1);

    // Test one deletion
    assert_eq!(levenshtein_distance("hello", "hell"), 1);
}

#[test]
fn test_block_match_score() {
    use pdftract_cli::inspect::api::block_match_score;

    let block_a = pdftract_core::schema::BlockJson {
        kind: "paragraph".to_string(),
        text: "Hello world".to_string(),
        bbox: [100.0, 100.0, 200.0, 200.0],
        level: None,
        table_index: None,
        spans: vec![],
        receipt: None,
    };

    // Test identical block (high score)
    let block_b = pdftract_core::schema::BlockJson {
        kind: "paragraph".to_string(),
        text: "Hello world".to_string(),
        bbox: [100.0, 100.0, 200.0, 200.0],
        level: None,
        table_index: None,
        spans: vec![],
        receipt: None,
    };
    let score = block_match_score(&block_a, &block_b);
    assert!(score > 0.9);

    // Test different text (lower score)
    let block_c = pdftract_core::schema::BlockJson {
        kind: "paragraph".to_string(),
        text: "Goodbye world".to_string(),
        bbox: [100.0, 100.0, 200.0, 200.0],
        level: None,
        table_index: None,
        spans: vec![],
        receipt: None,
    };
    let score = block_match_score(&block_a, &block_c);
    assert!(score < 0.9 && score > 0.5); // bbox matches but text doesn't
}

#[test]
fn test_span_match_score() {
    use pdftract_cli::inspect::api::span_match_score;

    let span_a = pdftract_core::schema::SpanJson {
        text: "Hello".to_string(),
        bbox: [100.0, 100.0, 150.0, 120.0],
        font: "Helvetica".to_string(),
        size: 12.0,
        color: None,
        rendering_mode: None,
        confidence: None,
        confidence_source: None,
        lang: None,
        flags: vec![],
        receipt: None,
        column: None,
    };

    // Test identical span (high score)
    let span_b = pdftract_core::schema::SpanJson {
        text: "Hello".to_string(),
        bbox: [100.0, 100.0, 150.0, 120.0],
        font: "Helvetica".to_string(),
        size: 12.0,
        color: None,
        rendering_mode: None,
        confidence: None,
        confidence_source: None,
        lang: None,
        flags: vec![],
        receipt: None,
        column: None,
    };
    let score = span_match_score(&span_a, &span_b);
    assert!(score > 0.9);

    // Test different text (lower score)
    let span_c = pdftract_core::schema::SpanJson {
        text: "World".to_string(),
        bbox: [100.0, 100.0, 150.0, 120.0],
        font: "Helvetica".to_string(),
        size: 12.0,
        color: None,
        rendering_mode: None,
        confidence: None,
        confidence_source: None,
        lang: None,
        flags: vec![],
        receipt: None,
        column: None,
    };
    let score = span_match_score(&span_a, &span_c);
    assert!(score < 0.9 && score > 0.4); // bbox matches but text doesn't
}
