//! Example: Compute PDF structural fingerprint.
//!
//! Demonstrates fingerprint computation for PDF document identification.
//! The fingerprint is a reproducible 256-bit hash that identifies the
//! semantic content independent of metadata churn.
//!
//! Usage:
//!   cargo run --example hash -- tests/fixtures/sample.pdf

use anyhow::Result;
use pdftract_core::fingerprint::{
    compute_fingerprint, ContentStreamData, FingerprintInput, PageFingerprintData,
};
use pdftract_core::parser::catalog::parse_catalog;
use pdftract_core::parser::pages::flatten_page_tree;
use pdftract_core::parser::stream::{FileSource, PdfSource};
use pdftract_core::parser::xref::{load_xref_with_prev_chain, XrefResolver};
use std::env;
use std::path::Path;

fn main() -> Result<()> {
    // Get PDF path from command line, or use a default
    let args: Vec<String> = env::args().collect();
    let pdf_path = args
        .get(1)
        .map(|s| s.as_str())
        .unwrap_or("tests/fixtures/sample.pdf");

    // Open the PDF
    let source = FileSource::open(Path::new(pdf_path))?;

    // Find the startxref offset
    let source_len = source.len()?;
    let tail_len = 1024.min(source_len as usize) as u64;
    let tail_start = source_len - tail_len;
    let tail_data = source.read_at(tail_start, tail_len as usize)?;

    let startxref_pos = tail_data
        .windows(9)
        .rposition(|w| w == b"startxref")
        .ok_or_else(|| anyhow::anyhow!("startxref not found"))?;

    let offset_str = std::str::from_utf8(&tail_data[startxref_pos + 9..])
        .map_err(|_| anyhow::anyhow!("Invalid UTF-8 in startxref"))?
        .split_whitespace()
        .next()
        .ok_or_else(|| anyhow::anyhow!("No offset after startxref"))?;

    let startxref_offset: u64 = offset_str
        .parse()
        .map_err(|_| anyhow::anyhow!("Invalid startxref offset"))?;

    // Load xref and parse catalog
    let xref_section = load_xref_with_prev_chain(&source, startxref_offset);
    let resolver = XrefResolver::from_section(xref_section.clone());

    let root_ref = xref_section
        .trailer
        .as_ref()
        .and_then(|t| t.get("Root"))
        .and_then(|o| o.as_ref())
        .ok_or_else(|| anyhow::anyhow!("No /Root in trailer"))?;

    let catalog =
        parse_catalog(&resolver, root_ref, Some(&source as &dyn PdfSource)).map_err(|d| {
            anyhow::anyhow!(
                "Catalog parse failed: {}",
                d.first().map(|d| d.message.as_ref()).unwrap_or("unknown")
            )
        })?;

    // Flatten page tree
    let pages = flatten_page_tree(&resolver, catalog.pages_ref).map_err(|d| {
        anyhow::anyhow!(
            "Page tree parse failed: {}",
            d.first().map(|d| d.message.as_ref()).unwrap_or("unknown")
        )
    })?;

    // Build fingerprint input
    let page_count = pages.len() as u32;
    let fingerprint_pages = pages
        .iter()
        .map(|page| PageFingerprintData {
            content_streams: page
                .contents
                .iter()
                .map(|&r| ContentStreamData::Indirect(r))
                .collect(),
            resources: None,
            media_box: page.media_box,
            crop_box: page.crop_box,
            rotate: page.rotate,
        })
        .collect();

    let fingerprint_input = FingerprintInput {
        page_count,
        pages: fingerprint_pages,
        struct_tree_root_ref: catalog.struct_tree_root_ref,
        is_tagged: catalog.mark_info.is_tagged,
        catalog_flags: Default::default(),
    };

    // Compute fingerprint
    let fingerprint = compute_fingerprint(
        &fingerprint_input,
        &resolver,
        Some(&source as &dyn PdfSource),
    );

    println!("{}", fingerprint);

    Ok(())
}
