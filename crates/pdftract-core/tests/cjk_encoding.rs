//! CJK encoding tests for Phase 2.3.
//!
//! Tests CJK text extraction from PDFs with various CJK encodings:
//! - GB18030 (Simplified Chinese)
//! - Shift-JIS (Japanese)
//! - EUC-KR (Korean)
//! - Big5 (Traditional Chinese)
//!
//! Reference: Plan section 2.3 CJK Encoding (line 1389-1415)

use pdftract_core::document::PdfExtractor;
use std::fs;
use std::path::Path;

/// Test fixture describing a CJK PDF and its expected text output.
struct CjkFixture {
    name: &'static str,
    pdf_path: &'static str,
    truth_path: &'static str,
    description: &'static str,
}

/// Get all CJK fixtures with their configuration.
fn get_fixtures() -> Vec<CjkFixture> {
    vec![
        CjkFixture {
            name: "chinese-gb18030",
            pdf_path: "../../../tests/fixtures/cjk/cjk-chinese-gb18030.pdf",
            truth_path: "../../../tests/fixtures/cjk/cjk-chinese-gb18030.txt",
            description: "Simplified Chinese with GB18030 encoding",
        },
        CjkFixture {
            name: "japanese-shiftjis",
            pdf_path: "../../../tests/fixtures/cjk/cjk-japanese-shiftjis.pdf",
            truth_path: "../../../tests/fixtures/cjk/cjk-japanese-shiftjis.txt",
            description: "Japanese with Shift-JIS encoding",
        },
        CjkFixture {
            name: "korean-euckr",
            pdf_path: "../../../tests/fixtures/cjk/cjk-korean-euckr.pdf",
            truth_path: "../../../tests/fixtures/cjk/cjk-korean-euckr.txt",
            description: "Korean with EUC-KR encoding",
        },
        CjkFixture {
            name: "tc-big5",
            pdf_path: "../../../tests/fixtures/cjk/cjk-tc-big5.pdf",
            truth_path: "../../../tests/fixtures/cjk/cjk-tc-big5.txt",
            description: "Traditional Chinese with Big5 encoding",
        },
    ]
}

/// Test a single CJK fixture.
fn test_cjk_fixture(fixture: &CjkFixture) -> Result<String, Box<dyn std::error::Error>> {
    let pdf_path = Path::new(fixture.pdf_path);

    // Open the PDF
    let extractor =
        PdfExtractor::open(pdf_path).map_err(|e| format!("Failed to open PDF: {}", e))?;

    // Extract text from first page (all CJK fixtures have single pages)
    let page_extraction = extractor
        .extract_page(0)
        .map_err(|e| format!("Failed to extract page: {}", e))?;

    // Concatenate text from all blocks
    let extracted_text: String = page_extraction
        .blocks
        .iter()
        .map(|block| block.text.as_str())
        .collect::<Vec<&str>>()
        .join("");

    Ok(extracted_text)
}

#[test]
fn test_cjk_gb18030_chinese() {
    let fixture = &get_fixtures()[0];
    let result = test_cjk_fixture(fixture);

    assert!(
        result.is_ok(),
        "GB18030 fixture should extract successfully: {:?}",
        result.err()
    );

    let extracted = result.unwrap();
    let expected = fs::read_to_string(fixture.truth_path).expect("Failed to read ground truth");

    assert_eq!(
        extracted.trim(),
        expected.trim(),
        "GB18030 extracted text should match ground truth"
    );
}

#[test]
fn test_cjk_shiftjis_japanese() {
    let fixture = &get_fixtures()[1];
    let result = test_cjk_fixture(fixture);

    assert!(
        result.is_ok(),
        "Shift-JIS fixture should extract successfully: {:?}",
        result.err()
    );

    let extracted = result.unwrap();
    let expected = fs::read_to_string(fixture.truth_path).expect("Failed to read ground truth");

    assert_eq!(
        extracted.trim(),
        expected.trim(),
        "Shift-JIS extracted text should match ground truth"
    );
}

#[test]
fn test_cjk_euckr_korean() {
    let fixture = &get_fixtures()[2];
    let result = test_cjk_fixture(fixture);

    assert!(
        result.is_ok(),
        "EUC-KR fixture should extract successfully: {:?}",
        result.err()
    );

    let extracted = result.unwrap();
    let expected = fs::read_to_string(fixture.truth_path).expect("Failed to read ground truth");

    assert_eq!(
        extracted.trim(),
        expected.trim(),
        "EUC-KR extracted text should match ground truth"
    );
}

#[test]
fn test_cjk_big5_traditional_chinese() {
    let fixture = &get_fixtures()[3];
    let result = test_cjk_fixture(fixture);

    assert!(
        result.is_ok(),
        "Big5 fixture should extract successfully: {:?}",
        result.err()
    );

    let extracted = result.unwrap();
    let expected = fs::read_to_string(fixture.truth_path).expect("Failed to read ground truth");

    assert_eq!(
        extracted.trim(),
        expected.trim(),
        "Big5 extracted text should match ground truth"
    );
}

#[test]
fn test_all_cjk_fixtures_exist() {
    for fixture in get_fixtures() {
        assert!(
            Path::new(fixture.pdf_path).exists(),
            "CJK fixture PDF should exist: {}",
            fixture.pdf_path
        );
        assert!(
            Path::new(fixture.truth_path).exists(),
            "CJK fixture ground truth should exist: {}",
            fixture.truth_path
        );
    }
}
