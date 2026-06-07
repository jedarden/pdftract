//! Debug test for fingerprint content stream resolution.

use pdftract_core::document::parse_pdf_file;
use pdftract_core::fingerprint::{compute_fingerprint, ContentStreamData, FingerprintInput, PageFingerprintData};
use pdftract_core::parser::xref::XrefResolver;

#[test]
fn debug_content_stream_resolution() {
    let cargo_manifest_dir = std::env::var("CARGO_MANIFEST_DIR").unwrap();
    let base = std::path::Path::new(&cargo_manifest_dir);
    let fixture_path = base
        .parent()
        .and_then(|p| p.parent())
        .unwrap_or(base)
        .join("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");

    println!("DEBUG: fixture_path = {:?}", fixture_path);
    println!("DEBUG: file exists = {:?}", fixture_path.exists());

    // Parse the PDF
    let (fingerprint, catalog, pages, resolver) = parse_pdf_file(&fixture_path)
        .expect("Failed to parse PDF");

    println!("Fingerprint from parse_pdf_file: {}", fingerprint);
    println!("Number of pages: {}", pages.len());
    println!("Catalog pages_ref: {:?}", catalog.pages_ref);

    // Try to resolve the pages_ref directly
    println!("=== Resolving catalog.pages_ref ===");
    match resolver.resolve(catalog.pages_ref) {
        Ok(obj) => {
            println!("  -> Discriminant: {:?}", std::mem::discriminant(&obj));
            if let Some(dict) = obj.as_dict() {
                println!("  -> IS DICT!");
                for (key, value) in dict.iter().take(10) {
                    println!("     {} -> {:?}", key, std::mem::discriminant(value));
                }
            } else if obj.is_null() {
                println!("  -> IS NULL (stub resolver)");
            }
        }
        Err(e) => {
            println!("  -> ERROR: {:?}", e);
        }
    }

    // Check page content streams
    for (i, page) in pages.iter().enumerate() {
        println!("=== Page {} ===", i);
        println!("Content streams: {}", page.contents.len());
        for (j, &content_ref) in page.contents.iter().enumerate() {
            println!("  Stream {} = {:?}", j, content_ref);

            // Try to resolve it WITHOUT source (should return Null)
            println!("  Resolve WITHOUT source:");
            match resolver.resolve(content_ref) {
                Ok(obj) => {
                    println!("    -> Discriminant: {:?}", std::mem::discriminant(&obj));
                    if let Some(stream) = obj.as_stream() {
                        println!("    -> IS STREAM! Length: {:?}", stream.dict.get("/Length"));
                        println!("    -> Dict: {:?}", stream.dict.iter().map(|(k, v)| (k, std::mem::discriminant(v))).collect::<Vec<_>>());
                    } else if obj.is_null() {
                        println!("    -> IS NULL (stub resolver)");
                    }
                }
                Err(e) => {
                    println!("    -> ERROR: {:?}", e);
                }
            }
        }
        println!("MediaBox: {:?}", page.media_box);
        println!("Rotate: {}", page.rotate);
    }
}

#[test]
fn debug_direct_content_stream_hash() {
    use std::sync::Arc;

    let resolver = XrefResolver::new();

    // Test with direct content streams (no source needed)
    let input_v1 = FingerprintInput {
        page_count: 1,
        pages: vec![PageFingerprintData {
            content_streams: vec![ContentStreamData::Direct(b"BT /F1 12 Tf 50 700 Td (Hello World) Tj ET".to_vec())],
            resources: None,
            media_box: [0.0, 0.0, 612.0, 792.0],
            crop_box: None,
            rotate: 0,
        }],
        struct_tree_root_ref: None,
        is_tagged: false,
        catalog_flags: Default::default(),
    };

    let input_v2 = FingerprintInput {
        page_count: 1,
        pages: vec![PageFingerprintData {
            content_streams: vec![ContentStreamData::Direct(b"BT /F1 12 Tf 50 700 Td (Hello Worl) Tj ET".to_vec())],
            resources: None,
            media_box: [0.0, 0.0, 612.0, 792.0],
            crop_box: None,
            rotate: 0,
        }],
        struct_tree_root_ref: None,
        is_tagged: false,
        catalog_flags: Default::default(),
    };

    let fp_v1 = compute_fingerprint(&input_v1, &resolver, None);
    let fp_v2 = compute_fingerprint(&input_v2, &resolver, None);

    println!("Direct content v1 fingerprint: {}", fp_v1);
    println!("Direct content v2 fingerprint: {}", fp_v2);

    assert_ne!(fp_v1, fp_v2, "Different direct content streams must produce different fingerprints");
}
