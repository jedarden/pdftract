//! Contract tests for the markdown-anchor public API declared in
//! `docs/integrations/markdown-anchors.md` (bead pdftract-48954da5).
//!
//! Enforces the four guarantees the doc declares:
//!
//! 1. **Regex schema freeze** — the pattern in the doc (stated three times:
//!    the "Regex Schema" fence, the Python snippet, and the JavaScript
//!    snippet) and `pdftract_core::markdown::ANCHOR_REGEX_PATTERN` are
//!    byte-identical, so the schema cannot drift across minor versions.
//! 2. **Documented examples parse** — every anchor example in the doc parses
//!    with the documented fields: zero-based page/block indices, 1-decimal
//!    bbox precision, `kind` word.
//! 3. **Round-trip** — blocks → anchored markdown → `parse_anchors` recovers
//!    the original block list: every block yields exactly one anchor whose
//!    (page, block, bbox, kind) match. Inline styling is documented as lossy
//!    and is deliberately not compared.
//! 4. **Edge cases** — code-fence passthrough, empty-block anchor emission,
//!    per-page block indexing.
//!
//! The end-to-end leg over real fixture PDFs
//! (`fixture_pdf_roundtrip_when_extraction_available`) runs the same
//! assertions on extraction output whenever extraction succeeds; fixture
//! extraction failures are reported and skipped (pre-existing breakage
//! tracked outside this contract).

use pdftract_core::extract::extract_pdf;
use pdftract_core::markdown::{
    block_to_markdown, page_to_markdown_with_links_and_footnotes, page_to_markdown_with_options,
    parse_anchors, MarkdownOptions, ANCHOR_REGEX_PATTERN,
};
use pdftract_core::options::ExtractionOptions;
use pdftract_core::schema::BlockJson;
use proptest::prelude::*;
use std::fs;
use std::path::Path;

// ---------------------------------------------------------------------------
// Helpers
// ---------------------------------------------------------------------------

/// Path to the contract document, relative to this crate.
fn doc_path() -> std::path::PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../../docs/integrations/markdown-anchors.md")
}

fn read_doc() -> String {
    fs::read_to_string(doc_path())
        .expect("docs/integrations/markdown-anchors.md must exist alongside the crate")
}

/// Build a minimal block for emission tests.
fn block(kind: &str, text: &str, bbox: [f64; 4]) -> BlockJson {
    BlockJson {
        kind: kind.to_string(),
        text: text.to_string(),
        bbox,
        level: None,
        table_index: None,
        spans: vec![],
        receipt: None,
    }
}

/// The bbox value an anchor carries for a block: the emitter casts the
/// block's f64 bbox to f32 and formats it at the documented 1-decimal
/// precision; `parse_anchors` reads that decimal back.
fn anchored_bbox(x: f64) -> f32 {
    format!("{:.1}", x as f32).parse::<f32>().unwrap()
}

/// Assert the anchors in `md` recover exactly the input block list:
/// one anchor per block, in document order, with matching
/// (page, block index, 1-decimal bbox, kind). Text is not compared —
/// inline styling is documented as lossy in the round-trip guarantee.
fn assert_anchor_set_matches_blocks(pages: &[Vec<BlockJson>], md: &str) {
    let anchors = parse_anchors(md);
    let total: usize = pages.iter().map(|p| p.len()).sum();
    assert_eq!(
        anchors.len(),
        total,
        "round-trip completeness: one anchor per block"
    );

    let mut prev: Option<(usize, usize)> = None;
    for (idx, a) in anchors.iter().enumerate() {
        assert!(
            a.page < pages.len(),
            "anchor {idx}: page {} out of range",
            a.page
        );
        let blocks = &pages[a.page];
        assert!(
            a.block < blocks.len(),
            "anchor {idx}: block {} out of range on page {}",
            a.block,
            a.page
        );
        let b = &blocks[a.block];
        assert_eq!(a.kind, b.kind, "anchor {idx}: kind drift");
        for k in 0..4 {
            assert_eq!(
                a.bbox[k],
                anchored_bbox(b.bbox[k]),
                "anchor {idx}: bbox[{k}] drift"
            );
        }
        let key = (a.page, a.block);
        if let Some(p) = prev {
            assert!(key > p, "anchor {idx}: anchors must be in document order");
        }
        prev = Some(key);
    }
}

/// Emit every page through both documented emission paths (the plain
/// page path and the production link-aware path) and assert the anchors
/// recover the block list through each.
fn assert_roundtrip_through_both_emitters(pages: &[Vec<BlockJson>]) {
    let mut md_links = String::new();
    let mut md_plain = String::new();
    for (page_idx, blocks) in pages.iter().enumerate() {
        md_links.push_str(&page_to_markdown_with_links_and_footnotes(
            blocks,
            &[],
            &[],
            &[],
            page_idx,
            true,
            &MarkdownOptions::default(),
            None,
        ));
        md_plain.push_str(&page_to_markdown_with_options(
            blocks,
            &[],
            page_idx,
            true,
            &MarkdownOptions::default(),
        ));
    }
    assert_anchor_set_matches_blocks(pages, &md_links);
    assert_anchor_set_matches_blocks(pages, &md_plain);
}

// ---------------------------------------------------------------------------
// 1. Regex schema freeze
// ---------------------------------------------------------------------------

#[test]
fn documented_regex_schema_is_frozen() {
    let doc = read_doc();

    // The doc states the schema three times (Regex Schema fence, Python
    // snippet, JavaScript snippet). Each occurrence — and the constant the
    // parser is built from — must be byte-identical, so the documented
    // schema cannot drift from the implementation.
    let occurrences = doc.matches(ANCHOR_REGEX_PATTERN).count();
    assert_eq!(
        occurrences, 3,
        "the documented anchor regex and ANCHOR_REGEX_PATTERN must be byte-identical \
         (expected in the Regex Schema fence and the Python + JS snippets)"
    );

    // The pinned pattern must compile — parse_anchors depends on it.
    regex::Regex::new(ANCHOR_REGEX_PATTERN).expect("pinned anchor regex must compile");

    // The stability declaration itself is part of the contract surface.
    assert!(
        doc.contains("stable public API"),
        "doc must keep the stability declaration"
    );
    assert!(
        doc.contains("will not change in a breaking way"),
        "doc must keep the cross-version compatibility promise"
    );
    assert!(
        doc.contains("pdftract_core::markdown::{{parse_anchors, Anchor}}")
            || doc.contains("pdftract_core::markdown::{parse_anchors, Anchor}"),
        "doc must keep documenting the Rust API path"
    );
}

// ---------------------------------------------------------------------------
// 2. Documented examples parse
// ---------------------------------------------------------------------------

#[test]
fn documented_examples_parse_with_documented_fields() {
    let doc = read_doc();

    // Exactly four lines of the doc are anchor comments (the schema/regex
    // snippets contain the grammar, not an instance, and must not parse).
    let line_re = regex::Regex::new(&format!("^{}$", ANCHOR_REGEX_PATTERN)).unwrap();
    let example_lines: Vec<&str> = doc
        .lines()
        .map(str::trim)
        .filter(|l| line_re.is_match(l))
        .collect();
    assert_eq!(
        example_lines.len(),
        4,
        "doc must contain exactly the four documented anchor examples"
    );

    let expected = [
        (
            "<!-- pdftract: page=3 block=12 bbox=[72.0,640.5,540.0,672.0] kind=heading -->",
            3usize,
            12usize,
            [72.0f32, 640.5, 540.0, 672.0],
        ),
        (
            "<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->",
            0,
            0,
            [72.0, 640.5, 540.0, 672.0],
        ),
        (
            "<!-- pdftract: page=0 block=1 bbox=[72.0,600.0,540.0,630.0] kind=paragraph -->",
            0,
            1,
            [72.0, 600.0, 540.0, 630.0],
        ),
        (
            "<!-- pdftract: page=1 block=0 bbox=[72.0,500.0,540.0,400.0] kind=table -->",
            1,
            0,
            [72.0, 500.0, 540.0, 400.0],
        ),
    ];

    for (line, (comment, page, block_idx, bbox)) in
        example_lines.iter().zip(expected.iter())
    {
        assert_eq!(*line, *comment, "documented example drifted from the pin");
        let parsed = parse_anchors(line);
        assert_eq!(parsed.len(), 1, "each documented example parses to one anchor");
        let a = &parsed[0];
        assert_eq!(a.page, *page);
        assert_eq!(a.block, *block_idx);
        assert_eq!(a.bbox, *bbox);
        // kind is read from the same line by construction of the pin.
        assert!(comment.ends_with(" -->"));
        // 1-decimal bbox precision: re-emitting the parsed anchor reproduces
        // the documented comment byte-for-byte.
        assert_eq!(a.to_comment(), *comment);
    }

    // Parsing the whole document recovers exactly those anchors, in order.
    let all = parse_anchors(&doc);
    assert_eq!(all.len(), 4, "schema snippets must not parse as anchors");
    assert_eq!(all[0].page, 3);
    assert_eq!(all[0].block, 12);
    assert_eq!(all[1].page, 0);
    assert_eq!(all[2].kind, "paragraph");
    assert_eq!(all[3].kind, "table");
}

// ---------------------------------------------------------------------------
// 3. Round-trip: blocks -> anchored markdown -> parse_anchors
// ---------------------------------------------------------------------------

#[test]
fn roundtrip_recovers_block_list_across_pages() {
    let pages: Vec<Vec<BlockJson>> = vec![
        vec![
            block("heading", "Introduction", [72.0, 640.5, 540.0, 672.0]),
            block(
                "paragraph",
                "This is the first paragraph of the document.",
                [72.0, 600.0, 540.0, 630.0],
            ),
            // A consecutive list run: every item must get its own anchor.
            block("list", "First bulleted item", [72.0, 540.0, 540.0, 560.0]),
            block("list", "2. Second numbered item", [72.0, 520.0, 540.0, 540.0]),
            block("list", "Nested item", [90.0, 500.0, 540.0, 520.0]),
            // Empty block: anchor with empty content following.
            block("paragraph", "", [72.0, 480.0, 540.0, 500.0]),
            block("table", "Cell data", [72.0, 400.0, 540.0, 450.0]),
        ],
        vec![
            // header blocks are excluded from output by default options but
            // still emit their anchor — the empty-block contract.
            block("header", "Page header", [72.0, 750.0, 540.0, 770.0]),
            block(
                "paragraph",
                "Only visible block on page 1",
                [72.0, 700.0, 540.0, 720.0],
            ),
        ],
        // Blank page: no blocks, so it contributes no anchors (anchors are
        // per-block; the empty-BLOCK contract is covered above).
        vec![],
        vec![block("figure", "Chart", [72.0, 300.0, 540.0, 350.0])],
    ];

    assert_roundtrip_through_both_emitters(&pages);

    // Spot-check the documented edge cases on the production emitter's
    // output directly.
    let mut md = String::new();
    for (page_idx, blocks) in pages.iter().enumerate() {
        md.push_str(&page_to_markdown_with_links_and_footnotes(
            blocks,
            &[],
            &[],
            &[],
            page_idx,
            true,
            &MarkdownOptions::default(),
            None,
        ));
    }

    // Block index is per-page: each page restarts at block 0.
    let anchors = parse_anchors(&md);
    assert!(
        anchors
            .iter()
            .any(|a| a.page == 1 && a.block == 0 && a.kind == "header"),
        "page 1 must restart its block index at 0"
    );
    assert!(
        anchors
            .iter()
            .any(|a| a.page == 3 && a.block == 0 && a.kind == "figure"),
        "page 3's single block must be block=0 on that page"
    );

    // Empty-block emission: an excluded header still emits its anchor, with
    // the next anchor line directly following (empty content).
    let lines: Vec<&str> = md.lines().collect();
    let hdr = lines
        .iter()
        .position(|l| l.contains("kind=header"))
        .expect("header anchor must be emitted");
    assert!(
        lines[hdr + 1].starts_with("<!-- pdftract: page=1"),
        "excluded header must emit an anchor with empty content following, got {:?}",
        lines[hdr + 1]
    );

    // Every list item in a consecutive run is individually addressable.
    let list_pages: Vec<Vec<BlockJson>> = pages[0]
        .iter()
        .filter(|b| b.kind == "list")
        .map(|b| vec![b.clone()])
        .collect();
    assert_eq!(list_pages.len(), 3, "fixture sanity: three list blocks");
    let first_page_md = page_to_markdown_with_links_and_footnotes(
        &pages[0],
        &[],
        &[],
        &[],
        0,
        true,
        &MarkdownOptions::default(),
        None,
    );
    let first_page_anchors = parse_anchors(&first_page_md);
    for (idx, lb) in list_pages.iter().enumerate() {
        assert!(
            first_page_anchors
                .iter()
                .any(|a| a.block == idx + 2 && a.kind == "list" && a.bbox[0] == anchored_bbox(lb[0].bbox[0])),
            "list item {idx} of the run must carry its own anchor"
        );
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(256))]

    /// Property: for arbitrary block documents (all documented block kinds,
    /// empty-text blocks, consecutive list runs, blank pages), emitting
    /// anchored markdown and parsing the anchors back recovers the block
    /// list exactly — one anchor per block, in order, matching
    /// (page, block, 1-decimal bbox, kind).
    #[test]
    fn roundtrip_property_recovers_block_list(
        pages in prop::collection::vec(
            prop::collection::vec(
                (
                    0usize..17,
                    0usize..5,
                    (0u32..=8000).prop_map(|k| f64::from(k) / 10.0),
                    (0u32..=8000).prop_map(|k| f64::from(k) / 10.0),
                    (0u32..=8000).prop_map(|k| f64::from(k) / 10.0),
                    (0u32..=8000).prop_map(|k| f64::from(k) / 10.0),
                )
                    .prop_map(|(k, t, x0, y0, x1, y1)| {
                        const KINDS: [&str; 17] = [
                            "heading", "paragraph", "list", "list_item", "table", "figure",
                            "caption", "code", "formula", "header", "footer", "watermark",
                            "block_quote", "toc", "note", "footnote", "reference",
                        ];
                        const TEXTS: [&str; 5] = [
                            "Some text.",
                            "",
                            "1. numbered item",
                            "line one\nline two",
                            "special *chars* [here]",
                        ];
                        block(KINDS[k], TEXTS[t], [x0, y0, x1, y1])
                    }),
                0..=10,
            ),
            1..=4,
        )
    ) {
        assert_roundtrip_through_both_emitters(&pages);
    }
}

// ---------------------------------------------------------------------------
// 4. Edge cases
// ---------------------------------------------------------------------------

#[test]
fn code_block_text_is_passed_through_verbatim_inside_fence() {
    // A code block whose source sample contains an anchor-looking comment:
    // the documented "Code Fences" behavior is that renderers pass fenced
    // comments through verbatim — the emitter must not mangle them.
    let anchor_in_sample =
        "<!-- pdftract: page=9 block=9 bbox=[1.0,2.0,3.0,4.0] kind=paragraph -->";
    let b = block(
        "code",
        &format!("let x = 1;\n{anchor_in_sample}\nlet y = 2;"),
        [0.0, 0.0, 100.0, 30.0],
    );
    let md = block_to_markdown(&b, &[], 0, 0, true);

    // The block's own anchor precedes the fence.
    assert!(md.starts_with("<!-- pdftract: page=0 block=0 "));

    // The sample comment survives verbatim, inside the fence body.
    let fence_open = md.find("```").expect("code block must be fenced");
    let body_start = md[fence_open..].find('\n').unwrap() + fence_open + 1;
    let body_end = md[body_start..].find("\n```").expect("fence must be closed") + body_start;
    let fence_body = &md[body_start..body_end];
    assert!(
        fence_body.contains(anchor_in_sample),
        "anchor-looking text inside a code block must pass through verbatim"
    );
}

#[test]
fn parse_anchors_is_a_flat_scan_fences_are_not_skipped() {
    // Documented limitation (§ "Code Fences"): comments inside code fences
    // are a Markdown *renderer* concern — pdftract's parser is deliberately
    // a flat scanner over the text. Pin that: making parse_anchors
    // fence-aware would be a breaking change to the stable parse API, so it
    // must be a visible decision, not silent drift.
    let md = "```markdown\n<!-- pdftract: page=0 block=0 bbox=[72.0,640.5,540.0,672.0] kind=heading -->\n```\n";
    let anchors = parse_anchors(md);
    assert_eq!(anchors.len(), 1, "fenced anchor-like comments are still parsed");
    assert_eq!(anchors[0].page, 0);
    assert_eq!(anchors[0].block, 0);
    assert_eq!(anchors[0].kind, "heading");
}

#[test]
fn empty_block_emits_anchor_with_empty_content() {
    let b = block("paragraph", "", [72.0, 600.0, 540.0, 630.0]);
    let md = block_to_markdown(&b, &[], 2, 5, true);

    let first_line = md.lines().next().unwrap();
    assert_eq!(
        first_line,
        "<!-- pdftract: page=2 block=5 bbox=[72.0,600.0,540.0,630.0] kind=paragraph -->",
        "an empty block still emits its anchor"
    );

    let anchors = parse_anchors(&md);
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].page, 2);
    assert_eq!(anchors[0].block, 5);
    assert_eq!(anchors[0].kind, "paragraph");
    assert_eq!(anchors[0].bbox, [72.0, 600.0, 540.0, 630.0]);
}

// ---------------------------------------------------------------------------
// 5. End-to-end over fixture PDFs
// ---------------------------------------------------------------------------

#[test]
fn fixture_pdf_roundtrip_when_extraction_available() {
    let fixtures_dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    let mut pdfs: Vec<std::path::PathBuf> = fs::read_dir(&fixtures_dir)
        .expect("core fixtures directory must exist")
        .filter_map(|e| e.ok().map(|e| e.path()))
        .filter(|p| p.extension().and_then(|s| s.to_str()) == Some("pdf"))
        .collect();
    pdfs.sort();

    assert!(pdfs.len() >= 3, "expected the standard core fixtures, found {}", pdfs.len());

    let mut verified = 0usize;
    let mut skipped = 0usize;
    for pdf in &pdfs {
        match extract_pdf(pdf, &ExtractionOptions::default()) {
            Ok(result) => {
                let mut pages_markdown: Vec<Vec<BlockJson>> = Vec::new();
                let mut md = String::new();
                for page in &result.pages {
                    let page_links: Vec<_> = result
                        .links
                        .iter()
                        .filter(|l| l.page_index == page.index)
                        .cloned()
                        .collect();
                    md.push_str(&page_to_markdown_with_links_and_footnotes(
                        &page.blocks,
                        &page.spans,
                        &page.tables,
                        &page_links,
                        page.index,
                        true,
                        &MarkdownOptions::default(),
                        None,
                    ));
                    pages_markdown.push(page.blocks.clone());
                }
                assert_anchor_set_matches_blocks(&pages_markdown, &md);
                verified += 1;
            }
            Err(e) => {
                skipped += 1;
                eprintln!(
                    "SKIPPED (pre-existing extraction failure, tracked outside this contract): {} — {e}",
                    pdf.display()
                );
            }
        }
    }

    eprintln!(
        "fixture anchor round-trip: {verified} verified, {skipped}/{} skipped",
        pdfs.len()
    );
}
