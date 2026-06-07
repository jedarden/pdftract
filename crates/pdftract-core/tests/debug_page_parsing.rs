//! Debug test to check page parsing for fingerprint fixtures.

use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::catalog::{parse_catalog, Catalog};
use pdftract_core::parser::pages::flatten_page_tree;
use pdftract_core::parser::stream::{FileSource, PdfSource};
use pdftract_core::parser::xref::{load_xref_with_prev_chain, XrefResolver};
use std::path::Path;

#[test]
fn test_debug_glyph_fixture_parsing() {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let base = Path::new(&cargo_manifest_dir);

    let v1_path = base
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(base)
        .join("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");

    let v2_path = base
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(base)
        .join("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    println!("Parsing v1: {:?}", v1_path);

    // Manual parsing to debug
    let source = FileSource::open(&v1_path).expect("Failed to open v1");
    let file_len = source.len().expect("Failed to get file length");
    println!("v1 file length: {}", file_len);

    // Read trailer to find startxref
    let tail_size = 1024.min(file_len) as usize;
    let tail_data = source.read_at(file_len - tail_size as u64, tail_size)
        .expect("Failed to read tail");
    let tail_str = std::str::from_utf8(&tail_data).unwrap_or("<invalid utf8>");
    println!("v1 tail:\n{}", tail_str);

    let startxref_offset = tail_str
        .find("startxref")
        .and_then(|pos| {
            let after = &tail_str[pos + 9..];
            after.lines().next()
                .and_then(|line| u64::from_str_radix(line.trim(), 10).ok())
        });
    println!("v1 startxref: {:?}", startxref_offset);

    if let Some(offset) = startxref_offset {
        let xref_section = load_xref_with_prev_chain(&source, offset);
        println!("v1 xref entries: {}", xref_section.entries.len());
        println!("v1 trailer: {:?}", xref_section.trailer);

        let root_ref = xref_section.trailer
            .as_ref()
            .and_then(|trailer| trailer.get("Root"))
            .and_then(|obj| obj.as_ref());
        println!("v1 /Root ref: {:?}", root_ref);

        if let Some(root_ref) = root_ref {
            let resolver = XrefResolver::from_section(xref_section.clone());
            println!("v1 resolving catalog...");

            let catalog_result = parse_catalog(&resolver, root_ref, Some(&source as &dyn PdfSource));
            match &catalog_result {
                Ok(catalog) => {
                    println!("v1 catalog pages_ref: {:?}", catalog.pages_ref);
                    let pages_result = flatten_page_tree(&resolver, catalog.pages_ref);
                    match &pages_result {
                        Ok(pages) => println!("v1 pages: {}", pages.len()),
                        Err(diagnostics) => println!("v1 flatten error: {:?}", diagnostics),
                    }
                }
                Err(diagnostics) => println!("v1 catalog error: {:?}", diagnostics),
            }
        }
    }

    println!("\nParsing v2: {:?}", v2_path);

    // Manual parsing to debug
    let source2 = FileSource::open(&v2_path).expect("Failed to open v2");
    let file_len2 = source2.len().expect("Failed to get file length");
    println!("v2 file length: {}", file_len2);

    // Read trailer to find startxref
    let tail_data2 = source2.read_at(file_len2 - tail_size as u64, tail_size)
        .expect("Failed to read tail");
    let tail_str2 = std::str::from_utf8(&tail_data2).unwrap_or("<invalid utf8>");
    println!("v2 tail:\n{}", tail_str2);

    let startxref_offset2 = tail_str2
        .find("startxref")
        .and_then(|pos| {
            let after = &tail_str2[pos + 9..];
            after.lines().next()
                .and_then(|line| u64::from_str_radix(line.trim(), 10).ok())
        });
    println!("v2 startxref: {:?}", startxref_offset2);
}

#[test]
fn test_debug_glyph_fixture_parse_pdf_file() {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let base = Path::new(&cargo_manifest_dir);

    let v1_path = base
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(base)
        .join("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");

    println!("Parsing v1 with parse_pdf_file: {:?}", v1_path);
    let (fp1, catalog1, pages1, _resolver1) = parse_pdf_file(&v1_path)
        .expect("Failed to parse v1");
    println!("v1 fingerprint: {}", fp1);
    println!("v1 catalog pages_ref: {:?}", catalog1.pages_ref);
    println!("v1 pages: {}", pages1.len());
}
