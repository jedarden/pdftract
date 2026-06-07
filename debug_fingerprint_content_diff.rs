//! Debug tool to compare fingerprints of two PDFs
use pdftract_core::document::compute_pdf_fingerprint;
use pdftract_core::fingerprint::{compute_fingerprint, FingerprintInput, PageFingerprintData, ContentStreamData};
use pdftract_core::parser::catalog::parse_catalog;
use pdftract_core::parser::pages::flatten_page_tree;
use pdftract_core::parser::stream::{FileSource, PdfSource};
use pdftract_core::parser::xref::{load_xref_with_prev_chain, XrefResolver};
use std::path::Path;

fn find_startxref(source: &FileSource) -> anyhow::Result<u64> {
    let len = source.len()?;
    let scan_size = 1024.min(len) as usize;
    let scan_start = (len - scan_size as u64) as u64;

    let tail_data = source.read_at(scan_start, scan_size)?;

    let startxref_pos = tail_data
        .windows(9)
        .rposition(|w| w == b"startxref")
        .ok_or_else(|| anyhow::anyhow!("startxref not found"))?;

    let offset_data = &tail_data[startxref_pos + 9..];

    let offset_start = offset_data
        .iter()
        .position(|&b| !matches!(b, b' ' | b'\r' | b'\n' | b'\t'))
        .unwrap_or(offset_data.len());

    let offset_data_trimmed = &offset_data[offset_start..];

    let newline_pos = offset_data_trimmed
        .iter()
        .position(|&b| b == b'\n' || b == b'\r')
        .unwrap_or(offset_data_trimmed.len());

    let offset_str = std::str::from_utf8(&offset_data_trimmed[..newline_pos])?;

    let offset: u64 = offset_str.trim().parse()?;
    Ok(offset)
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        eprintln!("Usage: {} <v1.pdf> <v2.pdf>", args[0]);
        std::process::exit(1);
    }

    let v1_path = Path::new(&args[1]);
    let v2_path = Path::new(&args[2]);

    println!("Comparing fingerprints:");
    println!("  v1: {}", v1_path.display());
    println!("  v2: {}", v2_path.display());
    println!();

    // Parse v1
    let source1 = FileSource::open(v1_path).unwrap();
    let startxref1 = find_startxref(&source1).unwrap();
    let xref1 = load_xref_with_prev_chain(&source1, startxref1);
    let resolver1 = XrefResolver::from_section(xref1.clone());
    let root_ref1 = xref1.trailer.as_ref().and_then(|t| t.get("Root")).and_then(|o| o.as_ref()).unwrap();
    let catalog1 = parse_catalog(&resolver1, root_ref1, Some(&source1 as &dyn PdfSource)).unwrap();
    let pages1 = flatten_page_tree(&resolver1, catalog1.pages_ref).unwrap();

    // Parse v2
    let source2 = FileSource::open(v2_path).unwrap();
    let startxref2 = find_startxref(&source2).unwrap();
    let xref2 = load_xref_with_prev_chain(&source2, startxref2);
    let resolver2 = XrefResolver::from_section(xref2.clone());
    let root_ref2 = xref2.trailer.as_ref().and_then(|t| t.get("Root")).and_then(|o| o.as_ref()).unwrap();
    let catalog2 = parse_catalog(&resolver2, root_ref2, Some(&source2 as &dyn PdfSource)).unwrap();
    let pages2 = flatten_page_tree(&resolver2, catalog2.pages_ref).unwrap();

    println!("v1: {} pages", pages1.len());
    println!("v2: {} pages", pages2.len());

    // Compare content stream references
    println!("\nv1 page 0 contents: {:?}", pages1[0].contents);
    println!("v2 page 0 contents: {:?}", pages2[0].contents);

    // Resolve and decode content streams
    println!("\n=== v1 content streams ===");
    for (i, &obj_ref) in pages1[0].contents.iter().enumerate() {
        println!("Stream {} (ref {:?}):", i, obj_ref);
        match resolver1.resolve(obj_ref) {
            Ok(pdftract_core::parser::object::PdfObject::Stream(stream)) => {
                println!("  Dict keys: {:?}", stream.dict.keys().collect::<Vec<_>>());
                let opts = pdftract_core::parser::stream::ExtractionOptions::default();
                let mut counter = 0u64;
                let decoded = pdftract_core::parser::stream::decode_stream(&*stream, &source1, &opts, &mut counter);
                println!("  Decoded {} bytes: {:?}", decoded.len(), String::from_utf8_lossy(&decoded));
            }
            Ok(other) => {
                println!("  Not a stream: {:?}", std::mem::discriminant(&other));
            }
            Err(e) => {
                println!("  Failed to resolve: {:?}", e);
            }
        }
    }

    println!("\n=== v2 content streams ===");
    for (i, &obj_ref) in pages2[0].contents.iter().enumerate() {
        println!("Stream {} (ref {:?}):", i, obj_ref);
        match resolver2.resolve(obj_ref) {
            Ok(pdftract_core::parser::object::PdfObject::Stream(stream)) => {
                println!("  Dict keys: {:?}", stream.dict.keys().collect::<Vec<_>>());
                let opts = pdftract_core::parser::stream::ExtractionOptions::default();
                let mut counter = 0u64;
                let decoded = pdftract_core::parser::stream::decode_stream(&*stream, &source2, &opts, &mut counter);
                println!("  Decoded {} bytes: {:?}", decoded.len(), String::from_utf8_lossy(&decoded));
            }
            Ok(other) => {
                println!("  Not a stream: {:?}", std::mem::discriminant(&other));
            }
            Err(e) => {
                println!("  Failed to resolve: {:?}", e);
            }
        }
    }

    // Compute fingerprints
    let fp1 = compute_pdf_fingerprint(v1_path).unwrap();
    let fp2 = compute_pdf_fingerprint(v2_path).unwrap();

    println!("\n=== Fingerprints ===");
    println!("v1: {}", fp1);
    println!("v2: {}", fp2);
    println!("Match: {}", fp1 == fp2);
}
