// Simple test to verify forward_scan_xref functionality
// This is a standalone test file to verify the forward scan implementation

use pdftract_core::parser::stream::MemorySource;
use pdftract_core::parser::xref::{forward_scan_xref, XrefEntry};

fn main() {
    println!("Testing forward_scan_xref implementation...\n");

    // Test 1: Simple PDF with a few indirect objects
    println!("Test 1: Simple PDF with indirect objects");
    let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                      2 0 obj\n<< /Type /Pages >>\nendobj\n\
                      3 0 obj\n<< /Type /Page >>\nendobj\n";

    let source = MemorySource::new(pdf_data.to_vec());
    let result = forward_scan_xref(&source, false);

    println!("  Found {} objects", result.len());
    assert_eq!(result.len(), 3, "Expected 3 objects");
    println!("  ✓ PASSED\n");

    // Test 2: Truncated file (critical test from plan)
    println!("Test 2: Truncated file - objects before truncation point");
    let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                      2 0 obj\n<< /Type /Pages >>\nendobj\n\
                      3 0 obj\n<< /Type /Page >>\nendobj\n\
                      xref\n\
                      0 4\n\
                      0000000000 65535 f \n\
                      0000000009 00000 n \n\
                      0000000045 00000 n \n\
                      0000000081 00000 n \n\
                      trailer\n\
                      << /Size 4 >>\n\
                      startxref\n\
                      117\n\
                      %%EOF\n\
                      4 0 obj\n\
                      << /Type /Outlines >>\n\
                      endobj\n";

    let source = MemorySource::new(pdf_data.to_vec());
    let result = forward_scan_xref(&source, false);

    println!(
        "  Found {} objects (including the one after truncated xref)",
        result.len()
    );
    assert!(result.len() >= 4, "Expected at least 4 objects");
    println!("  ✓ PASSED\n");

    // Test 3: Linearized file - should be disabled
    println!("Test 3: Linearized file - forward scan should be disabled");
    let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n";

    let source = MemorySource::new(pdf_data.to_vec());
    let result = forward_scan_xref(&source, true); // is_linearized = true

    println!("  Found {} objects (should be 0)", result.len());
    assert_eq!(result.len(), 0, "Expected 0 objects for linearized file");
    println!(
        "  Has LINEARIZED_NO_FORWARD_SCAN diagnostic: {}",
        result.diagnostics.iter().any(|d| matches!(
            d.code,
            pdftract_core::diagnostics::DiagCode::XrefLinearizedNoForwardScan
        ))
    );
    println!("  ✓ PASSED\n");

    // Test 4: Multi-revision - last occurrence wins
    println!("Test 4: Multi-revision handling - last occurrence wins");
    let pdf_data = b"1 0 obj\n<< /Type /Catalog /V 1 >>\nendobj\n\
                      2 0 obj\n<< /Type /Pages >>\nendobj\n\
                      1 0 obj\n<< /Type /Catalog /V 2 >>\nendobj\n";

    let source = MemorySource::new(pdf_data.to_vec());
    let result = forward_scan_xref(&source, false);

    println!("  Found {} unique objects", result.len());
    assert_eq!(result.len(), 2, "Expected 2 unique objects");

    // Object 1 should point to the SECOND occurrence (higher offset)
    if let Some(XrefEntry::InUse { offset, .. }) = result.entries.get(&1) {
        println!("  Object 1 offset: {} (should be > 50)", offset);
        assert!(*offset > 50, "Object 1 should point to second occurrence");
    }
    println!("  ✓ PASSED\n");

    // Test 5: XREF_REPAIRED diagnostic emission
    println!("Test 5: XREF_REPAIRED diagnostic emission");
    let pdf_data = b"1 0 obj\n<< /Type /Catalog >>\nendobj\n\
                      2 0 obj\n<< /Type /Pages >>\nendobj\n";

    let source = MemorySource::new(pdf_data.to_vec());
    let result = forward_scan_xref(&source, false);

    let has_repaired_diagnostic = result
        .diagnostics
        .iter()
        .any(|d| matches!(d.code, pdftract_core::diagnostics::DiagCode::XrefRepaired));
    println!(
        "  Has XREF_REPAIRED diagnostic: {}",
        has_repaired_diagnostic
    );
    assert!(has_repaired_diagnostic, "Expected XREF_REPAIRED diagnostic");
    println!("  ✓ PASSED\n");

    // Test 6: Empty file - no panic
    println!("Test 6: Empty file - should not panic");
    let pdf_data = b"";
    let source = MemorySource::new(pdf_data.to_vec());
    let result = forward_scan_xref(&source, false);
    println!("  Found {} objects", result.len());
    assert_eq!(result.len(), 0);
    println!("  ✓ PASSED\n");

    // Test 7: File with no objects - no panic
    println!("Test 7: File with no indirect objects");
    let pdf_data = b"%PDF-1.4\n\
                      % Some random content\n\
                      %%EOF\n";
    let source = MemorySource::new(pdf_data.to_vec());
    let result = forward_scan_xref(&source, false);
    println!("  Found {} objects", result.len());
    assert_eq!(result.len(), 0);
    println!("  ✓ PASSED\n");

    println!("All forward_scan_xref tests PASSED! ✓");
}
