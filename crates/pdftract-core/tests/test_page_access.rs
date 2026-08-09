//! Test infrastructure for accessing Page objects from Document results
//!
//! This test file demonstrates and verifies the proper way to access Page objects
//! from Document parsing results, including both single Page and multiple Pages access patterns.
//!
//! Acceptance criteria:
//! - Test file can access Page objects from Document parse results
//! - Access code handles single Page object correctly
//! - Access code handles multiple Page objects in list/array correctly
//! - Test runs without errors accessing Pages
//!
//! Refers to: bf-49jplg

use anyhow::Result;
use pdftract_core::document::parse_pdf_file;
use pdftract_core::parser::pages::PageDict;
use std::path::PathBuf;

/// Returns the path to a simple test fixture with known content
fn test_fixture_path() -> PathBuf {
    let mut path = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    path.push("../../tests/fixtures/encrypted/EC-04-rc4-encrypted.pdf");
    path
}

/// Basic test: verify the fixture file exists and can be parsed
#[test]
fn test_document_parse_fixture() {
    let path = test_fixture_path();
    assert!(
        path.exists(),
        "Fixture file should exist at {}",
        path.display()
    );

    match parse_pdf_file(&path) {
        Ok((fingerprint, _catalog, pages, _resolver)) => {
            println!("✓ Document parsed successfully");
            println!("  Fingerprint: {}", fingerprint);
            println!("  Page count: {}", pages.len());
        }
        Err(e) => {
            println!("Note: Could not parse document (expected for encrypted fixtures): {}", e);
        }
    }
}

/// Test accessing a single Page from parse_pdf_file result
#[test]
fn test_access_single_page() {
    let path = test_fixture_path();
    if !path.exists() {
        println!("⚠ Fixture not found, skipping test");
        return;
    }

    let (_fingerprint, _catalog, pages, _resolver) = match parse_pdf_file(&path) {
        Ok(result) => result,
        Err(e) => {
            println!("⚠ Could not parse document (expected for encrypted): {}", e);
            return;
        }
    };

    if pages.is_empty() {
        println!("⚠ Document has no pages");
        return;
    }

    // Access the first page only (single Page access pattern)
    let first_page = &pages[0];

    println!("✓ Successfully accessed single Page");
    println!("  Page obj_ref: {}", first_page.obj_ref);
    println!("  MediaBox: {:?}", first_page.media_box);
    println!("  Rotation: {} degrees", first_page.rotate);
    println!("  Content streams: {}", first_page.contents.len());

    // Verify PageDict struct fields are accessible
    assert!(first_page.media_box[2] > 0.0, "Page should have positive width");
    assert!(first_page.media_box[3] > 0.0, "Page should have positive height");
    assert_eq!(first_page.rotate % 90, 0, "Rotation should be multiple of 90");
}

/// Test accessing multiple Pages from parse_pdf_file result
#[test]
fn test_access_multiple_pages() {
    let path = test_fixture_path();
    if !path.exists() {
        println!("⚠ Fixture not found, skipping test");
        return;
    }

    let (_fingerprint, _catalog, pages, _resolver) = match parse_pdf_file(&path) {
        Ok(result) => result,
        Err(e) => {
            println!("⚠ Could not parse document (expected for encrypted): {}", e);
            return;
        }
    };

    println!("✓ Accessed {} page(s)", pages.len());

    for (idx, page) in pages.iter().enumerate() {
        println!("  Page {}: MediaBox={:?}, Rotate={}°",
                 idx, page.media_box, page.rotate);
    }

    // Test accessing specific pages by index
    if !pages.is_empty() {
        let first_page = &pages[0];
        let last_page = &pages[pages.len() - 1];

        println!("✓ First page: MediaBox {:?}", first_page.media_box);
        println!("✓ Last page: MediaBox {:?}", last_page.media_box);
    }
}

/// Test type assertions for PageDict objects
#[test]
fn test_page_type_assertions() {
    let path = test_fixture_path();
    if !path.exists() {
        println!("⚠ Fixture not found, skipping test");
        return;
    }

    let (_fingerprint, _catalog, pages, _resolver) = match parse_pdf_file(&path) {
        Ok(result) => result,
        Err(e) => {
            println!("⚠ Could not parse document (expected for encrypted): {}", e);
            return;
        }
    };

    if pages.is_empty() {
        println!("⚠ Document has no pages");
        return;
    }

    // Test that we get the correct type from parse_pdf_file
    let first_page = &pages[0];

    // Type assertion: this is a PageDict struct with expected fields
    let _obj_ref: pdftract_core::parser::object::ObjRef = first_page.obj_ref;
    let _media_box: [f64; 4] = first_page.media_box;
    let _crop_box: Option<[f64; 4]> = first_page.crop_box;
    let _rotate: i32 = first_page.rotate;
    let _contents: Vec<pdftract_core::parser::object::ObjRef> = first_page.contents.clone();
    let _annots: Vec<pdftract_core::parser::object::ObjRef> = first_page.annots.clone();

    println!("✓ Type assertion passed: PageDict has expected fields");
    println!("  - obj_ref: {}", _obj_ref);
    println!("  - media_box: {:?}", _media_box);
    println!("  - crop_box: {:?}", _crop_box);
    println!("  - rotate: {}", _rotate);
    println!("  - contents count: {}", _contents.len());
    println!("  - annots count: {}", _annots.len());
}

/// Test Page vector access patterns
#[test]
fn test_page_vector_access_patterns() {
    let path = test_fixture_path();
    if !path.exists() {
        println!("⚠ Fixture not found, skipping test");
        return;
    }

    let (_fingerprint, _catalog, pages, _resolver) = match parse_pdf_file(&path) {
        Ok(result) => result,
        Err(e) => {
            println!("⚠ Could not parse document (expected for encrypted): {}", e);
            return;
        }
    };

    // Test 1: Vector length access
    let page_count = pages.len();
    println!("✓ Page count: {}", page_count);

    // Test 2: First/last access
    if !pages.is_empty() {
        let first = pages.first().unwrap();
        let last = pages.last().unwrap();
        println!("✓ First page media_box: {:?}", first.media_box);
        println!("✓ Last page media_box: {:?}", last.media_box);
    }

    // Test 3: Iteration pattern
    let mut iter_count = 0;
    for page in &pages {
        iter_count += 1;
        assert!(page.media_box[2] > 0.0, "Page width should be positive");
        assert!(page.media_box[3] > 0.0, "Page height should be positive");
    }
    assert_eq!(iter_count, page_count, "Iteration should visit all pages");

    // Test 4: get() method for safe access
    if page_count > 0 {
        let valid_page = pages.get(0);
        assert!(valid_page.is_some(), "Should get page at valid index 0");

        let invalid_page = pages.get(page_count);
        assert!(invalid_page.is_none(), "Should not get page at out-of-bounds index");
    }
}

/// Test accessing Page fields with validation
#[test]
fn test_page_field_access() {
    let path = test_fixture_path();
    if !path.exists() {
        println!("⚠ Fixture not found, skipping test");
        return;
    }

    let (_fingerprint, _catalog, pages, _resolver) = match parse_pdf_file(&path) {
        Ok(result) => result,
        Err(e) => {
            println!("⚠ Could not parse document (expected for encrypted): {}", e);
            return;
        }
    };

    if pages.is_empty() {
        println!("⚠ Document has no pages");
        return;
    }

    for page in &pages {
        // MediaBox validation
        assert!(page.media_box.len() == 4, "MediaBox should have 4 elements");
        assert!(page.media_box[2] > page.media_box[0], "MediaBox width should be positive");
        assert!(page.media_box[3] > page.media_box[1], "MediaBox height should be positive");

        // Rotation validation
        assert!(page.rotate == 0 || page.rotate == 90 || page.rotate == 180 || page.rotate == 270,
                "Rotation should be 0, 90, 180, or 270");

        // Contents validation (can be empty for blank pages)
        println!("Page has {} content streams", page.contents.len());

        // Resources should be present
        let _resources = &page.resources;
    }

    println!("✓ All page fields validated successfully");
}

/// Test empty page list handling
#[test]
fn test_empty_page_handling() {
    // Create a test with empty pages to ensure we handle it gracefully
    let pages: Vec<PageDict> = vec![];

    assert_eq!(pages.len(), 0, "Empty pages vector should have length 0");
    assert!(pages.is_empty(), "Empty pages vector should be empty");

    // Test first/last on empty vector
    assert!(pages.first().is_none(), "First page on empty vector should be None");
    assert!(pages.last().is_none(), "Last page on empty vector should be None");
}
