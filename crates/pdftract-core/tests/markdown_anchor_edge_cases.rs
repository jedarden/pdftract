//! Focused edge-case coverage for the public Markdown anchor API.

use pdftract_core::markdown::{
    page_to_markdown_with_links_and_footnotes, page_to_markdown_with_options, parse_anchors,
    Anchor, MarkdownOptions,
};
use pdftract_core::schema::BlockJson;
use std::path::Path;

fn block(kind: &str, text: &str, bbox: [f64; 4]) -> BlockJson {
    BlockJson {
        kind: kind.to_string(),
        text: text.to_string(),
        bbox,
        level: None,
        table_index: None,
        spans: Vec::new(),
        receipt: None,
    }
}

#[test]
fn parse_anchors_round_trips_negative_bounding_box_coordinates() {
    let expected = Anchor::new(2, 4, [-72.0, -36.5, 594.0, 755.5], "paragraph".to_string());

    let markdown = expected.to_comment();
    assert_eq!(parse_anchors(&markdown), vec![expected]);
}

#[test]
fn parse_anchors_skips_malformed_fields_without_skipping_later_valid_anchors() {
    let markdown = concat!(
        "<!-- pdftract: page=0 block=0 bbox=[1.0,2.0,3.0,4.0] kind=heading -->\n",
        "<!-- pdftract: page=0 block=1 bbox=[1.0,2.0,3.0] kind=paragraph -->\n",
        "<!-- pdftract: page=0 block=2 bbox=[1.0,,3.0,4.0] kind=paragraph -->\n",
        "<!-- pdftract: page=0 block=3 bbox=[1.0,2.0,3.0,4.0,5.0] kind=paragraph -->\n",
        "<!-- pdftract: page=-1 block=4 bbox=[1.0,2.0,3.0,4.0] kind=paragraph -->\n",
        "<!-- pdftract: page=0 block=5 bbox=[5.0,6.0,7.0,8.0] kind=paragraph -->\n",
    );

    let anchors = parse_anchors(markdown);
    assert_eq!(
        anchors,
        vec![
            Anchor::new(0, 0, [1.0, 2.0, 3.0, 4.0], "heading".to_string()),
            Anchor::new(0, 5, [5.0, 6.0, 7.0, 8.0], "paragraph".to_string()),
        ]
    );
}

#[test]
fn parse_anchors_excludes_comments_from_long_and_nested_fences() {
    let inside_backtick_fence =
        "<!-- pdftract: page=7 block=7 bbox=[1.0,2.0,3.0,4.0] kind=code -->";
    let inside_tilde_fence = "<!-- pdftract: page=8 block=8 bbox=[5.0,6.0,7.0,8.0] kind=code -->";
    let markdown = format!(
        "<!-- pdftract: page=0 block=0 bbox=[0.0,0.0,1.0,1.0] kind=paragraph -->\n\
         ````markdown\n\
         ```\n\
         {inside_backtick_fence}\n\
         ```\n\
         ````\n\
         ~~~rust\n\
         {inside_tilde_fence}\n\
         ~~~\n\
         <!-- pdftract: page=0 block=1 bbox=[2.0,2.0,3.0,3.0] kind=paragraph -->\n"
    );

    let anchors = parse_anchors(&markdown);
    assert_eq!(anchors.len(), 2);
    assert_eq!((anchors[0].page, anchors[0].block), (0, 0));
    assert_eq!((anchors[1].page, anchors[1].block), (0, 1));
    assert!(!anchors.iter().any(|anchor| anchor.page > 0));
}

#[test]
fn page_emission_keeps_empty_blocks_and_restarts_indexes_per_page() {
    let pages = [
        vec![
            block("paragraph", "first", [1.0, 2.0, 3.0, 4.0]),
            block("paragraph", "", [5.0, 6.0, 7.0, 8.0]),
        ],
        vec![block("heading", "second page", [9.0, 10.0, 11.0, 12.0])],
    ];
    let mut markdown = String::new();

    for (page_index, blocks) in pages.iter().enumerate() {
        markdown.push_str(&page_to_markdown_with_options(
            blocks,
            &[],
            page_index,
            true,
            &MarkdownOptions::default(),
        ));
    }

    let anchors = parse_anchors(&markdown);
    assert_eq!(
        anchors
            .iter()
            .map(|anchor| (anchor.page, anchor.block))
            .collect::<Vec<_>>(),
        vec![(0, 0), (0, 1), (1, 0)]
    );

    let empty_anchor = markdown
        .lines()
        .position(|line| line.contains("page=0 block=1"))
        .expect("empty block anchor should be emitted");
    let next_anchor = markdown
        .lines()
        .skip(empty_anchor + 1)
        .position(|line| line.contains("page=1 block=0"))
        .map(|offset| empty_anchor + 1 + offset)
        .expect("next page anchor should be emitted");
    assert!(
        markdown
            .lines()
            .skip(empty_anchor + 1)
            .take(next_anchor - empty_anchor - 1)
            .all(|line| line.trim().is_empty()),
        "an empty block must not acquire content before the next anchor"
    );
}

#[test]
fn extracted_pdf_pages_round_trip_through_markdown_anchors() {
    let fixture =
        Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/test-minimal.pdf");
    let extracted = pdftract_core::extract_pdf(&fixture, &Default::default())
        .expect("the stable minimal fixture should extract");
    assert!(!extracted.pages.is_empty(), "fixture should contain pages");

    let mut markdown = String::new();
    let mut expected = Vec::new();
    for (position, page) in extracted.pages.iter().enumerate() {
        let options = MarkdownOptions {
            include_page_breaks: position + 1 < extracted.pages.len(),
            ..Default::default()
        };
        markdown.push_str(&page_to_markdown_with_links_and_footnotes(
            &page.blocks,
            &page.spans,
            &page.tables,
            &[],
            page.index,
            true,
            &options,
            None,
        ));
        expected.extend(
            page.blocks.iter().enumerate().map(|(block_index, block)| {
                (page.index, block_index, block.kind.clone(), block.bbox)
            }),
        );
    }

    let anchors = parse_anchors(&markdown);
    assert_eq!(anchors.len(), expected.len());
    for (anchor, (page, block, kind, bbox)) in anchors.iter().zip(expected) {
        assert_eq!(anchor.page, page);
        assert_eq!(anchor.block, block);
        assert_eq!(anchor.kind, kind);
        for (actual, source) in anchor.bbox.iter().zip(bbox) {
            assert!(
                (*actual - source as f32).abs() <= 0.051,
                "anchor bbox should preserve source coordinates to one decimal place"
            );
        }
    }
}
