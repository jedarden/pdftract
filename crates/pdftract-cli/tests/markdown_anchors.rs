//! CLI-level regression coverage for Markdown positional anchors.

use pdftract_core::markdown::{
    page_to_markdown_with_options, parse_anchors, MarkdownOptions, ANCHOR_REGEX_PATTERN,
};
use pdftract_core::schema::BlockJson;
use regex::Regex;
use std::io::Write;
use std::process::Command;
use tempfile::NamedTempFile;

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

#[test]
fn cli_markdown_anchors_round_trip_all_json_block_metadata() {
    let fixture = markdown_anchor_fixture();
    let path = fixture
        .path()
        .to_str()
        .expect("fixture path should be UTF-8");

    let json_output = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .args(["extract", "--no-cache", "--json", "-", path])
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
    assert_eq!(pages.len(), 2, "fixture must exercise multiple pages");

    let expected: Vec<(usize, usize, [f64; 4], String)> = pages
        .iter()
        .enumerate()
        .flat_map(|(page_position, page)| {
            let page_index = page["index"].as_u64().unwrap_or(page_position as u64) as usize;
            page["blocks"]
                .as_array()
                .expect("each JSON page should contain blocks")
                .iter()
                .enumerate()
                .map(move |(block_index, block)| {
                    let bbox = block["bbox"]
                        .as_array()
                        .expect("each JSON block should contain a bbox");
                    assert_eq!(bbox.len(), 4, "each JSON block bbox has four components");
                    (
                        page_index,
                        block_index,
                        [
                            bbox[0].as_f64().expect("bbox x0 should be numeric"),
                            bbox[1].as_f64().expect("bbox y0 should be numeric"),
                            bbox[2].as_f64().expect("bbox x1 should be numeric"),
                            bbox[3].as_f64().expect("bbox y1 should be numeric"),
                        ],
                        block["kind"]
                            .as_str()
                            .expect("each JSON block should contain a kind")
                            .to_string(),
                    )
                })
        })
        .collect();
    let block_texts: Vec<&str> = pages
        .iter()
        .flat_map(|page| {
            page["blocks"]
                .as_array()
                .expect("each JSON page should contain blocks")
                .iter()
                .map(|block| {
                    block["text"]
                        .as_str()
                        .expect("each block should contain text")
                })
        })
        .collect();
    for marker in [
        "Markdown Anchor Heading",
        "A paragraph before the list.",
        "- First list item",
        "- Second list item",
        "Table page paragraph",
    ] {
        assert!(
            block_texts.iter().any(|text| text.contains(marker)),
            "fixture block text should contain {marker:?}"
        );
    }
    assert!(
        expected.iter().any(|(_, _, _, kind)| kind == "table"),
        "fixture must include a table block"
    );
    assert!(
        expected
            .iter()
            .any(|(page, block, _, _)| *page == 1 && *block == 0),
        "the second page must restart block indexing at zero"
    );

    let markdown_output = Command::new(env!("CARGO_BIN_EXE_pdftract"))
        .args(["extract", "--no-cache", "--md", "-", "--md-anchors", path])
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
    for (anchor, (page, block, bbox, kind)) in anchors.iter().zip(expected) {
        assert_eq!(
            (anchor.page, anchor.block, &anchor.kind),
            (page, block, &kind)
        );
        for (actual, source) in anchor.bbox.iter().zip(bbox) {
            assert!(
                (*actual as f64 - source).abs() <= 0.051,
                "anchor bbox should preserve JSON coordinates to one decimal place: {actual} vs {source}"
            );
        }
    }
}

#[test]
fn markdown_anchor_emission_includes_empty_blocks() {
    let empty = BlockJson {
        kind: "paragraph".to_string(),
        text: String::new(),
        bbox: [12.0, 24.0, 120.0, 36.0],
        level: None,
        table_index: None,
        spans: Vec::new(),
        receipt: None,
    };
    let markdown =
        page_to_markdown_with_options(&[empty], &[], 4, true, &MarkdownOptions::default());
    assert_eq!(
        parse_anchors(&markdown),
        vec![pdftract_core::markdown::Anchor::new(
            4,
            0,
            [12.0, 24.0, 120.0, 36.0],
            "paragraph".to_string(),
        )]
    );
}

/// Build a small two-page PDF at test time so this test exercises the CLI's
/// JSON and Markdown extraction paths without depending on a generated binary
/// fixture. The first page contains heading/paragraph/list-shaped text; the
/// second page contains a bordered table and text.
fn markdown_anchor_fixture() -> NamedTempFile {
    let page_one = concat!(
        "BT /F1 24 Tf 72 720 Td (Markdown Anchor Heading) Tj ET\n",
        "BT /F1 12 Tf 72 620 Td (A paragraph before the list.) Tj ET\n",
        "BT /F1 12 Tf 72 520 Td (- First list item) Tj\n",
        "72 -24 Td (- Second list item) Tj ET\n",
    );
    let page_two = concat!(
        "BT /F1 12 Tf 72 720 Td (Table page paragraph) Tj ET\n",
        "50 560 m 500 560 l S\n",
        "50 500 m 500 500 l S\n",
        "50 440 m 500 440 l S\n",
        "50 560 m 50 440 l S\n",
        "275 560 m 275 440 l S\n",
        "500 560 m 500 440 l S\n",
        "BT /F1 12 Tf 72 530 Td (Name) Tj ET\n",
    );
    let pdf = two_page_pdf(&[page_one, page_two]);
    let mut fixture = NamedTempFile::new().expect("temporary fixture should be created");
    fixture
        .write_all(&pdf)
        .expect("temporary fixture should be writable");
    fixture
}

fn two_page_pdf(page_streams: &[&str]) -> Vec<u8> {
    let pages_id = 2;
    let first_page_id = 3;
    let first_content_id = first_page_id + page_streams.len();
    let font_id = first_content_id + page_streams.len();
    let mut objects = vec![
        (1, "<< /Type /Catalog /Pages 2 0 R >>".to_string()),
        (
            pages_id,
            format!(
                "<< /Type /Pages /Kids [{}] /Count {} >>",
                (0..page_streams.len())
                    .map(|i| format!("{} 0 R", first_page_id + i))
                    .collect::<Vec<_>>()
                    .join(" "),
                page_streams.len()
            ),
        ),
    ];
    for (i, _) in page_streams.iter().enumerate() {
        let page_id = first_page_id + i;
        let content_id = first_content_id + i;
        objects.push((
            page_id,
            format!(
                "<< /Type /Page /Parent {pages_id} 0 R /MediaBox [0 0 612 792] /Contents {content_id} 0 R /Resources << /Font << /F1 {font_id} 0 R >> >> >>"
            ),
        ));
    }
    for (i, stream) in page_streams.iter().enumerate() {
        let content_id = first_content_id + i;
        objects.push((
            content_id,
            format!(
                "<< /Length {} >>\nstream\n{}endstream",
                stream.len(),
                stream
            ),
        ));
    }
    objects.push((
        font_id,
        "<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_string(),
    ));

    let max_id = objects.iter().map(|(id, _)| *id).max().unwrap();
    let mut pdf = b"%PDF-1.4\n".to_vec();
    let mut offsets = vec![0usize; max_id + 1];
    for (id, body) in objects {
        offsets[id] = pdf.len();
        pdf.extend_from_slice(format!("{id} 0 obj\n{body}\nendobj\n").as_bytes());
    }
    let xref_offset = pdf.len();
    pdf.extend_from_slice(format!("xref\n0 {}\n", max_id + 1).as_bytes());
    pdf.extend_from_slice(b"0000000000 65535 f \n");
    for offset in offsets.iter().skip(1) {
        pdf.extend_from_slice(format!("{offset:010} 00000 n \n").as_bytes());
    }
    pdf.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref_offset}\n%%EOF\n",
            max_id + 1
        )
        .as_bytes(),
    );
    pdf
}
