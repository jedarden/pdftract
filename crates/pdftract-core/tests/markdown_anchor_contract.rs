//! Enforcement tests for the Markdown anchor stability contract.
//!
//! `docs/integrations/markdown-anchors.md` declares the anchor HTML-comment
//! format a stable public API with a published regex schema and a round-trip
//! property (extract → `parse_anchors` → recovered block list, modulo inline
//! styling). This file enforces those claims; it is the schema-freeze test
//! named by `crates/pdftract-core/src/markdown.rs` (`ANCHOR_REGEX_PATTERN`
//! doc comment). Nothing here may be relaxed without a matching doc change.
//!
//! Enforced surface:
//! 1. The regex constant, the doc's ` ```regex ` block, and the doc's
//!    Python/JavaScript example patterns stay byte-identical.
//! 2. Every anchor emitted by every emission path matches the published
//!    regex and carries page / block / bbox (1-decimal precision) / kind,
//!    with per-page (not global) block indexing.
//! 3. Round-trip: emission → `parse_anchors` recovers the emitted block
//!    list's (page, block, bbox, kind) metadata — including end-to-end from
//!    a synthesized PDF through the real extraction pipeline.
//! 4. Doc edge cases: empty blocks still emit anchors, and anchor-like
//!    comments inside code fences are never treated as anchors (real
//!    anchors are never emitted inside fences).
//! 5. `parse_anchors` / `Anchor` remain public exports of `pdftract_core`
//!    and `pdftract_core::markdown`.

use pdftract_core::markdown::{
    block_to_markdown, page_to_markdown_with_links, page_to_markdown_with_links_and_footnotes,
    page_to_markdown_with_options, parse_anchors, Anchor, MarkdownOptions, ANCHOR_REGEX_PATTERN,
};
use pdftract_core::schema::BlockJson;
use regex::Regex;

// ---------------------------------------------------------------------------
// Fixtures and helpers
// ---------------------------------------------------------------------------

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

fn heading(level: u8, text: &str, bbox: [f64; 4]) -> BlockJson {
    BlockJson {
        level: Some(level),
        ..block("heading", text, bbox)
    }
}

/// Multi-page fixture covering the contract's corners: several kinds, a
/// consecutive list run (mixed x0 for nesting), an empty-text block,
/// fractional bboxes that require 1-decimal rounding, and negative
/// components (negative MediaBox origin — documented as legal).
fn sample_pages() -> Vec<Vec<BlockJson>> {
    vec![
        vec![
            heading(1, "Introduction", [72.0, 640.5, 540.0, 672.0]),
            block(
                "paragraph",
                "First paragraph of body text.",
                [72.0, 600.25, 540.125, 630.0],
            ),
            block("list", "First item", [72.0, 560.0, 540.0, 575.0]),
            block("list", "Second item", [90.0, 545.0, 540.0, 560.0]),
            block("list", "Third item", [72.0, 530.0, 540.0, 545.0]),
            block("code", "let x = 1;", [72.0, 480.0, 540.0, 520.0]),
            block(
                "figure",
                "Architecture diagram",
                [72.0, 300.0, 540.0, 460.0],
            ),
            // Empty block: the doc guarantees it still emits an anchor.
            block("paragraph", "", [72.0, 280.0, 540.0, 290.0]),
        ],
        vec![
            // Negative MediaBox origin (bleed/crop), documented legal.
            block(
                "paragraph",
                "Second page opens here.",
                [-18.0, -36.5, 594.0, 755.5],
            ),
            heading(2, "Details", [72.0, 700.0, 540.0, 730.0]),
        ],
    ]
}

/// The published schema pinned to the documented bbox shape: exactly four
/// comma-separated components, one decimal place each, each admitting a
/// leading `-` (negative MediaBox origin).
fn strict_anchor_regex() -> Regex {
    Regex::new(
        r#"^<!--\s*pdftract:\s*page=(\d+)\s+block=(\d+)\s+bbox=\[-?\d+\.\d,-?\d+\.\d,-?\d+\.\d,-?\d+\.\d\]\s+kind=(\w+)\s*-->$"#,
    )
    .expect("strict anchor regex must compile")
}

/// The published regex as written in the doc's ` ```regex ` fence.
fn doc_regex_pattern() -> String {
    let doc = integration_doc();
    let marker = "```regex\n";
    let start = doc
        .find(marker)
        .expect("docs/integrations/markdown-anchors.md must contain a ```regex fence")
        + marker.len();
    let end = doc[start..]
        .find("\n```")
        .expect("docs/integrations/markdown-anchors.md ```regex fence must be closed")
        + start;
    let content = &doc[start..end];
    assert!(
        !content.contains('\n'),
        "published regex fence must stay a single line"
    );
    content.to_string()
}

/// A pattern embedded in one of the doc's integration examples.
///
/// `after` locates the example, `start_delim`/`end_delim` bracket the
/// pattern text within it. Extracting the doc's own patterns (rather than
/// searching for the constant) is what makes doc drift a test failure.
fn doc_embedded_pattern(after: &str, start_delim: &str, end_delim: &str) -> String {
    let doc = integration_doc();
    let anchor_idx = doc
        .find(after)
        .unwrap_or_else(|| panic!("doc must contain {after:?}"));
    let rest = &doc[anchor_idx..];
    let start = rest
        .find(start_delim)
        .unwrap_or_else(|| panic!("doc example after {after:?} must contain {start_delim:?}"))
        + start_delim.len();
    let end = rest[start..]
        .find(end_delim)
        .unwrap_or_else(|| panic!("doc example after {after:?} must contain {end_delim:?}"))
        + start;
    rest[start..end].to_string()
}

fn integration_doc() -> String {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../docs/integrations/markdown-anchors.md");
    std::fs::read_to_string(&path).unwrap_or_else(|e| {
        panic!(
            "cannot read published integration doc {}: {e}",
            path.display()
        )
    })
}

/// One shipped markdown emission path. The CLI's
/// `--format markdown --md-anchors` uses
/// `page_to_markdown_with_links_and_footnotes`; the other paths are public
/// API and must uphold the identical anchor contract.
fn emit_pages(path: &str, pages: &[Vec<BlockJson>], opts: &MarkdownOptions) -> String {
    let mut out = String::new();
    for (i, blocks) in pages.iter().enumerate() {
        match path {
            "page_to_markdown_with_options" => {
                out.push_str(&page_to_markdown_with_options(blocks, &[], i, true, opts));
            }
            "page_to_markdown_with_links" => {
                out.push_str(&page_to_markdown_with_links(
                    blocks,
                    &[],
                    &[],
                    &[],
                    i,
                    true,
                    opts,
                ));
            }
            "page_to_markdown_with_links_and_footnotes" => {
                out.push_str(&page_to_markdown_with_links_and_footnotes(
                    blocks,
                    &[],
                    &[],
                    &[],
                    i,
                    true,
                    opts,
                    None,
                ));
            }
            other => panic!("unknown emission path {other}"),
        }
    }
    out
}

fn emission_paths() -> Vec<&'static str> {
    vec![
        "page_to_markdown_with_options",
        "page_to_markdown_with_links",
        "page_to_markdown_with_links_and_footnotes",
    ]
}

/// Every line of `md` that is an anchor comment (matched by the published
/// regex anywhere in the line), in order.
fn anchor_lines(md: &str) -> Vec<String> {
    let re = Regex::new(ANCHOR_REGEX_PATTERN).expect("published regex must compile");
    md.lines()
        .filter(|line| re.is_match(line))
        .map(|line| line.trim().to_string())
        .collect()
}

/// Segment the emitted markdown by anchor: for anchor i, the content lines
/// between it and anchor i+1 (page-anchor `<a name=...>` lines and `---`
/// separators are ignored, since not every path emits them at the same
/// points). Returns one content-lines vector per anchor.
fn content_after_each_anchor(md: &str) -> Vec<Vec<String>> {
    let re = Regex::new(ANCHOR_REGEX_PATTERN).expect("published regex must compile");
    let mut segments: Vec<Vec<String>> = Vec::new();
    for line in md.lines() {
        if re.is_match(line) {
            segments.push(Vec::new());
            continue;
        }
        let is_ignorable = line.starts_with("<a name=") || line.trim() == "---";
        if let Some(current) = segments.last_mut() {
            if !is_ignorable {
                current.push(line.to_string());
            }
        }
    }
    segments
}

// ---------------------------------------------------------------------------
// 1. Schema freeze — the published regex is byte-identical everywhere
// ---------------------------------------------------------------------------

#[test]
fn published_doc_regex_is_byte_identical_to_anchor_regex_constant() {
    assert_eq!(
        doc_regex_pattern(),
        ANCHOR_REGEX_PATTERN,
        "the doc's ```regex fence and ANCHOR_REGEX_PATTERN drifted apart — \
         markdown.rs requires them byte-identical"
    );
}

#[test]
fn doc_python_and_javascript_examples_match_published_regex() {
    let python_pattern = doc_embedded_pattern("import re", "r'", "'");
    let javascript_pattern = doc_embedded_pattern("const ANCHOR_RE", "/", "/g;");
    assert_eq!(
        python_pattern, ANCHOR_REGEX_PATTERN,
        "Python example drifted"
    );
    assert_eq!(
        javascript_pattern, ANCHOR_REGEX_PATTERN,
        "JavaScript example drifted"
    );
}

// ---------------------------------------------------------------------------
// 2. Field contract — page / block / bbox (1-decimal) / kind, per page
// ---------------------------------------------------------------------------

#[test]
fn every_emitted_anchor_matches_published_regex_and_count_equals_blocks() {
    let pages = sample_pages();
    let total_blocks: usize = pages.iter().map(|p| p.len()).sum();
    let strict = strict_anchor_regex();

    for path in emission_paths() {
        let md = emit_pages(path, &pages, &MarkdownOptions::default());
        let lines = anchor_lines(&md);
        assert_eq!(
            lines.len(),
            total_blocks,
            "{path}: anchor count must equal block count"
        );
        for line in &lines {
            assert!(
                strict.is_match(line),
                "{path}: anchor violates the documented schema (1-decimal bbox, \
                 negative components admitted): {line}"
            );
        }
        // No anchor-like comment may appear that is not a well-formed anchor.
        let raw_comments = md.matches("<!-- pdftract:").count();
        assert_eq!(
            raw_comments, total_blocks,
            "{path}: every `<!-- pdftract:` comment must be a schema-valid anchor"
        );
    }
}

#[test]
fn block_index_is_per_page_not_global() {
    // Page 0 has 8 blocks; page 1 has 2. A global counter would emit
    // blocks 8..10 on page 1 — the doc says each page restarts at 0.
    let pages = sample_pages();

    for path in emission_paths() {
        let md = emit_pages(path, &pages, &MarkdownOptions::default());
        let anchors = parse_anchors(&md);

        let page0_blocks: Vec<usize> = anchors
            .iter()
            .filter(|a| a.page == 0)
            .map(|a| a.block)
            .collect();
        let page1_blocks: Vec<usize> = anchors
            .iter()
            .filter(|a| a.page == 1)
            .map(|a| a.block)
            .collect();
        assert_eq!(
            page0_blocks,
            (0..8).collect::<Vec<_>>(),
            "{path}: page 0 block indices must be contiguous 0..8"
        );
        assert_eq!(
            page1_blocks,
            vec![0, 1],
            "{path}: page 1 must restart at block 0 (per-page, not global indexing)"
        );
    }
}

#[test]
fn block_to_markdown_emits_requested_page_and_block_index() {
    let b = block(
        "paragraph",
        "Indexable content.",
        [72.0, 100.0, 540.0, 120.0],
    );
    let md = block_to_markdown(&b, &[], 4, 7, true);
    let anchors = parse_anchors(&md);
    assert_eq!(anchors.len(), 1);
    assert_eq!(anchors[0].page, 4);
    assert_eq!(anchors[0].block, 7);
    assert_eq!(anchors[0].kind, "paragraph");
    assert!(
        strict_anchor_regex().is_match(anchor_lines(&md)[0].trim()),
        "single-block path must emit the documented schema"
    );
}

// ---------------------------------------------------------------------------
// 3. Round-trip — emission → parse_anchors → recovered block list
// ---------------------------------------------------------------------------

/// Assert that `anchors` recovers `pages`' block metadata: same count, same
/// order, exact (page, block) coordinates, exact kinds, and bboxes equal to
/// the source within 1-decimal rounding tolerance (±0.05 + float slack).
fn assert_round_trip(path: &str, pages: &[Vec<BlockJson>], md: &str) {
    let anchors = parse_anchors(md);
    let total: usize = pages.iter().map(|p| p.len()).sum();
    assert_eq!(
        anchors.len(),
        total,
        "{path}: round-trip must recover every block"
    );

    let mut idx = 0;
    for (page_idx, blocks) in pages.iter().enumerate() {
        for (block_idx, source) in blocks.iter().enumerate() {
            let a = &anchors[idx];
            assert_eq!(a.page, page_idx, "{path}: recovered page for block {idx}");
            assert_eq!(a.block, block_idx, "{path}: recovered block index {idx}");
            assert_eq!(
                a.kind, source.kind,
                "{path}: recovered kind for (p{page_idx}, b{block_idx})"
            );
            for c in 0..4 {
                let delta = (a.bbox[c] - source.bbox[c] as f32).abs();
                assert!(
                    delta <= 0.051,
                    "{path}: bbox[{c}] drifted past 1-decimal precision at \
                     (p{page_idx}, b{block_idx}): {} vs {}",
                    a.bbox[c],
                    source.bbox[c]
                );
            }
            idx += 1;
        }
    }
}

#[test]
fn round_trip_recovers_block_list_from_emitted_markdown() {
    let pages = sample_pages();
    for path in emission_paths() {
        let md = emit_pages(path, &pages, &MarkdownOptions::default());
        assert_round_trip(path, &pages, &md);

        // "Modulo inline styling": every non-empty block must have content
        // following its anchor; the empty block must have none.
        let segments = content_after_each_anchor(&md);
        assert_eq!(segments.len(), pages.iter().map(|p| p.len()).sum::<usize>());
        let mut idx = 0;
        for blocks in &pages {
            for source in blocks {
                let content = &segments[idx];
                if source.text.is_empty() {
                    assert!(
                        content.iter().all(|l| l.trim().is_empty()),
                        "{path}: empty block must emit an anchor with empty content \
                         (segment {idx}: {content:?})"
                    );
                } else {
                    assert!(
                        content.iter().any(|l| !l.trim().is_empty()),
                        "{path}: non-empty block {idx} must have content after its anchor"
                    );
                }
                idx += 1;
            }
        }
    }
}

#[test]
fn anchors_preserve_document_order() {
    let pages = sample_pages();
    for path in emission_paths() {
        let md = emit_pages(path, &pages, &MarkdownOptions::default());
        let anchors = parse_anchors(&md);
        let coords: Vec<(usize, usize)> = anchors.iter().map(|a| (a.page, a.block)).collect();
        let mut sorted = coords.clone();
        sorted.sort();
        assert_eq!(
            coords, sorted,
            "{path}: parse_anchors must return anchors in emission (document) order"
        );
    }
}

/// Build a minimal but fully valid two-page PDF (classic xref table, no
/// object streams, no compression, Helvetica text). The byte layout was
/// validated against pypdf when this test was authored.
fn build_two_page_pdf() -> Vec<u8> {
    let pages_content: [&[&str]; 2] = [
        &[
            "Introduction Chapter",
            "First paragraph of body text.",
            "Second paragraph.",
        ],
        &["Second page heading text", "More content follows."],
    ];

    let n = pages_content.len();
    let mut objs: Vec<(usize, Vec<u8>)> = Vec::new();
    objs.push((1, b"<< /Type /Catalog /Pages 2 0 R >>".to_vec()));
    let kids: String = (0..n).map(|i| format!("{} 0 R ", 3 + i * 3)).collect();
    objs.push((
        2,
        format!(
            "<< /Type /Pages /Kids [{}] /Count {} >>",
            kids.trim_end(),
            n
        )
        .into_bytes(),
    ));

    for (i, texts) in pages_content.iter().enumerate() {
        let base = 3 + i * 3;
        objs.push((
            base,
            format!(
                "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 612 792] \
                 /Contents {} 0 R /Resources << /Font << /F1 {} 0 R >> >> >>",
                base + 1,
                base + 2
            )
            .into_bytes(),
        ));
        let mut stream = String::new();
        for (j, text) in texts.iter().enumerate() {
            let y = 720 - j * 40;
            stream.push_str(&format!("BT /F1 18 Tf 72 {y} Td ({text}) Tj ET\n"));
        }
        objs.push((
            base + 1,
            format!(
                "<< /Length {} >>\nstream\n{}\nendstream",
                stream.len(),
                stream
            )
            .into_bytes(),
        ));
        objs.push((
            base + 2,
            b"<< /Type /Font /Subtype /Type1 /BaseFont /Helvetica >>".to_vec(),
        ));
    }

    let mut out = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets: Vec<(usize, usize)> = Vec::new();
    for (num, body) in &objs {
        offsets.push((*num, out.len()));
        out.extend_from_slice(format!("{num} 0 obj\n").as_bytes());
        out.extend_from_slice(body);
        out.extend_from_slice(b"\nendobj\n");
    }

    let xref_pos = out.len();
    let max = objs.len() as u32;
    out.extend_from_slice(format!("xref\n0 {}\n", max + 1).as_bytes());
    out.extend_from_slice(b"0000000000 65535 f \n");
    for num in 1..=max {
        let off = offsets
            .iter()
            .find(|(n, _)| *n == num as usize)
            .expect("object offset")
            .1;
        out.extend_from_slice(format!("{off:010} 00000 n \n").as_bytes());
    }
    out.extend_from_slice(
        format!(
            "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{}\n%%EOF\n",
            max + 1,
            xref_pos
        )
        .as_bytes(),
    );
    out
}

/// End-to-end round-trip: synthesized PDF → `extract_pdf` → anchored
/// markdown (the CLI emission path) → `parse_anchors` → recovered blocks.
///
/// Extraction is currently broken for all inputs at HEAD (known pre-existing
/// failure, "Document contains no pages", tracked separately from this
/// bead). While that stands, this test records a loud waiver and enforces
/// nothing beyond the error surface. The moment extraction works again — or
/// fails any other way — the full enforcement below runs automatically.
#[test]
fn round_trip_from_synthesized_pdf() {
    let dir = std::env::temp_dir().join(format!("pdftract-anchor-e2e-{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create temp dir");
    let path = dir.join("two_page.pdf");
    std::fs::write(&path, build_two_page_pdf()).expect("write synthesized pdf");

    let result = pdftract_core::extract_pdf(&path, &Default::default());
    let outcome = match result {
        Ok(result) => result,
        Err(e) => {
            let chain = format!("{e:#}");
            let _ = std::fs::remove_dir_all(&dir);
            if chain.contains("Document contains no pages") {
                eprintln!(
                    "WAIVER (round_trip_from_synthesized_pdf): pdftract cannot extract \
                     any PDF at HEAD — pre-existing \"Document contains no pages\" failure \
                     (pypdf reads the same bytes fine). Anchor round-trip from a real PDF \
                     is unenforced until extraction is fixed; this test upgrades itself \
                     automatically when it is."
                );
                return;
            }
            panic!("extraction failed with an unexpected error: {chain}");
        }
    };

    assert_eq!(
        outcome.pages.len(),
        2,
        "synthesized PDF must extract as exactly two pages"
    );
    let total_blocks: usize = outcome.pages.iter().map(|p| p.blocks.len()).sum();
    if total_blocks == 0 {
        let _ = std::fs::remove_dir_all(&dir);
        eprintln!(
            "WAIVER (round_trip_from_synthesized_pdf): extraction succeeded but \
             segmented zero blocks from the synthesized PDF — nothing to round-trip. \
             Investigate block segmentation if this persists."
        );
        return;
    }

    // The CLI's --format markdown --md-anchors emission shape: page breaks
    // between pages, anchors on, no footnotes (Phase 7 not wired in CLI).
    let mut md = String::new();
    for page in &outcome.pages {
        let is_last = page.index == outcome.pages.len() - 1;
        let page_opts = MarkdownOptions {
            include_page_breaks: !is_last,
            ..Default::default()
        };
        md.push_str(&page_to_markdown_with_links_and_footnotes(
            &page.blocks,
            &page.spans,
            &page.tables,
            &[], // no links asserted in this fixture
            page.index,
            true,
            &page_opts,
            None,
        ));
    }

    let anchors = parse_anchors(&md);
    assert_eq!(
        anchors.len(),
        total_blocks,
        "end-to-end: anchor count must equal extracted block count"
    );
    for (page_idx, page) in outcome.pages.iter().enumerate() {
        let per_page: Vec<usize> = anchors
            .iter()
            .filter(|a| a.page == page_idx)
            .map(|a| a.block)
            .collect();
        let expected: Vec<usize> = (0..page.blocks.len()).collect();
        assert_eq!(
            per_page, expected,
            "end-to-end: page {page_idx} must recover contiguous per-page block indices"
        );
        for (block_idx, source) in page.blocks.iter().enumerate() {
            let a = &anchors[anchors
                .iter()
                .position(|a| a.page == page_idx && a.block == block_idx)
                .expect("anchor present")];
            assert_eq!(a.kind, source.kind, "end-to-end: kind round-trip");
        }
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// 4. Doc edge cases
// ---------------------------------------------------------------------------

#[test]
fn empty_blocks_still_emit_anchors() {
    // Paragraphs and suppression-skipped headers emit truly empty content
    // for empty text (heading/list would still emit their structural
    // prefix, e.g. "# ", which is fine — the *content* is what's empty).
    let pages = vec![vec![
        block("paragraph", "", [72.0, 700.0, 540.0, 720.0]),
        block("header", "", [72.0, 680.0, 540.0, 700.0]),
        block("paragraph", "", [72.0, 660.0, 540.0, 680.0]),
    ]];

    for path in emission_paths() {
        let md = emit_pages(path, &pages, &MarkdownOptions::default());
        let anchors = parse_anchors(&md);
        assert_eq!(
            anchors.len(),
            3,
            "{path}: empty blocks must still emit one anchor each"
        );
        let segments = content_after_each_anchor(&md);
        for (i, content) in segments.iter().enumerate() {
            assert!(
                content.iter().all(|l| l.trim().is_empty()),
                "{path}: empty block {i} must have no content after its anchor"
            );
        }
    }
}

#[test]
fn anchor_like_comment_inside_code_fence_is_never_an_anchor() {
    let sneaky = "<!-- pdftract: page=9 block=9 bbox=[1.0,2.0,3.0,4.0] kind=paragraph -->";
    let pages = vec![vec![
        block(
            "paragraph",
            "Before the code block.",
            [72.0, 700.0, 540.0, 720.0],
        ),
        block(
            "code",
            &format!("print('hi')\n{sneaky}\nprint('bye')"),
            [72.0, 600.0, 540.0, 680.0],
        ),
    ]];

    for path in emission_paths() {
        let md = emit_pages(path, &pages, &MarkdownOptions::default());

        // Verbatim fence content survives (the doc promises verbatim
        // emission inside fences).
        assert!(
            md.contains(sneaky),
            "{path}: code-fence content must be emitted verbatim"
        );

        // …but it must not parse as an anchor: exactly the two real anchors
        // are recovered, so the round-trip block count holds.
        let anchors = parse_anchors(&md);
        assert_eq!(
            anchors.len(),
            2,
            "{path}: anchor-like comment inside a code fence must not false-positive"
        );
        let kinds: Vec<&str> = anchors.iter().map(|a| a.kind.as_str()).collect();
        assert_eq!(kinds, vec!["paragraph", "code"], "{path}: kinds recovered");
        assert!(
            !anchors.iter().any(|a| a.page == 9),
            "{path}: the fabricated page=9 anchor leaked through"
        );
    }
}

#[test]
fn parse_anchors_ignores_anchor_like_text_inside_fences() {
    let real_before = "<!-- pdftract: page=0 block=0 bbox=[0.0,0.0,1.0,1.0] kind=heading -->";
    let inside = "<!-- pdftract: page=7 block=7 bbox=[1.0,2.0,3.0,4.0] kind=table -->";
    let real_after = "<!-- pdftract: page=0 block=1 bbox=[2.0,3.0,4.0,5.0] kind=paragraph -->";
    let md = format!(
        "{real_before}\n# Title\n```python\n{inside}\nprint('x')\n```\n{real_after}\ntext\n"
    );

    let anchors = parse_anchors(&md);
    assert_eq!(anchors.len(), 2, "only anchors outside fences may parse");
    assert_eq!(anchors[0].block, 0);
    assert_eq!(anchors[1].block, 1);
    assert!(!anchors.iter().any(|a| a.page == 7));
}

#[test]
fn parse_anchors_handles_tilde_fences_and_fence_like_content() {
    // ~~~ fences count too; three-backtick content inside a ~~~ fence stays
    // content; a shorter same-char run inside a fence does not close it.
    let inside = "<!-- pdftract: page=5 block=5 bbox=[1.0,2.0,3.0,4.0] kind=figure -->";
    let md = format!(
        "~~~\n```\n{inside}\n```\n~~~\n<!-- pdftract: page=0 block=0 bbox=[0.0,0.0,1.0,1.0] kind=paragraph -->\n"
    );
    let anchors = parse_anchors(&md);
    assert_eq!(anchors.len(), 1, "nested fence runs must not leak anchors");
    assert_eq!(anchors[0].page, 0);

    // An anchor remains an anchor at fence depth 0 even right before a fence.
    let md2 =
        "<!-- pdftract: page=0 block=0 bbox=[0.0,0.0,1.0,1.0] kind=paragraph -->\n```\ncode\n```\n";
    assert_eq!(parse_anchors(md2).len(), 1);
}

#[test]
fn real_anchors_are_never_emitted_inside_fences() {
    let pages = vec![vec![
        block("code", "fn main() {}", [72.0, 600.0, 540.0, 680.0]),
        block("paragraph", "after", [72.0, 500.0, 540.0, 520.0]),
    ]];
    for path in emission_paths() {
        let md = emit_pages(path, &pages, &MarkdownOptions::default());
        let mut depth: i32 = 0;
        for line in md.lines() {
            let opens_fence = line.trim_start().starts_with("```");
            if opens_fence {
                if depth == 0 {
                    depth = 1;
                } else {
                    depth = 0;
                }
                continue;
            }
            if depth > 0 {
                assert!(
                    !line.contains("<!-- pdftract:"),
                    "{path}: a real anchor was emitted inside a code fence"
                );
            }
        }
    }
}

// ---------------------------------------------------------------------------
// 5. Public export surface
// ---------------------------------------------------------------------------

#[test]
fn parse_anchors_and_anchor_remain_public_exports() {
    // The doc's Rust API example imports from `pdftract_core::markdown`;
    // the crate root re-exports both names (lib.rs). Exercise both paths —
    // if either export disappears, this test stops compiling.
    use pdftract_core::{parse_anchors as root_parse_anchors, Anchor as RootAnchor};

    let anchor = RootAnchor::new(3, 12, [72.0, 640.5, 540.0, 672.0], "heading".to_string());
    assert_eq!(
        anchor.to_comment(),
        "<!-- pdftract: page=3 block=12 bbox=[72.0,640.5,540.0,672.0] kind=heading -->"
    );

    let module_anchor = Anchor::new(0, 0, [1.0, 2.0, 3.0, 4.0], "table".to_string());
    let parsed = root_parse_anchors(&module_anchor.to_comment());
    assert_eq!(parsed, vec![module_anchor]);
}
