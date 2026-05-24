//! Generate tagged PDF fixtures for testing Phase 7.1.4 coverage check
//!
//! This creates three fixtures:
//! 1. tagged-suspects-true.pdf - Suspects true, 60% coverage -> fallback to XY-cut
//! 2. tagged-suspects-false.pdf - Suspects false, 50% coverage -> trust StructTree
//! 3. tagged-suspects-true-high-coverage.pdf - Suspects true, 95% coverage -> trust StructTree

use std::fs::File;
use std::io::Write;

fn write_pdf(path: &str, suspects: bool, num_claimed: usize, num_total: usize) -> Result<(), Box<dyn std::error::Error>> {
    // Create ParentTree /Nums array with claimed and null entries
    // Format: /Nums [0 [ref ref null ref ...]]
    let mut nums_content = String::from("  /Nums [\n    0 [");
    for i in 0..num_total {
        if i < num_claimed {
            nums_content.push_str(" 5 0 R");
        } else {
            nums_content.push_str(" null");
        }
        if i < num_total - 1 {
            nums_content.push(' ');
        }
    }
    nums_content.push_str(" ]\n  ]\n");

    // Create /K array for StructElem with MCIDs
    let k_array = (0..num_total).map(|i| i.to_string()).collect::<Vec<_>>().join(" ");

    // Calculate coverage percentage for debugging
    let coverage = (num_claimed as f64 / num_total as f64) * 100.0;

    let pdf_data = format!(
        "%PDF-1.7
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
/MarkInfo <<
  /Marked true
  /Suspects {}
>>
/StructTreeRoot 3 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Kids [4 0 R]
/Count 1
>>
endobj
3 0 obj
<<
/Type /StructTreeRoot
/K [5 0 R]
/ParentTree 6 0 R
>>
endobj
4 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 7 0 R
/StructParents 0
>>
endobj
5 0 obj
<<
/Type /StructElem
/S /P
/K [{}]
>>
endobj
6 0 obj
<<
{}
>>
endobj
7 0 obj
<<
/Length 44
>>
stream
BT
/F1 12 Tf
100 700 Td
(Test) Tj
ET
endstream
endobj
xref
0 8
0000000000 65535 f
0000000009 00000 n
0000000121 00000 n
0000000205 00000 n
0000000317 00000 n
0000000449 00000 n
0000000529 00000 n
0000000685 00000 n
trailer
<<
/Size 8
/Root 1 0 R
>>
startxref
751
%%EOF",
        if suspects { "true" } else { "false" },
        k_array,
        nums_content
    );

    let mut file = File::create(path)?;
    file.write_all(pdf_data.as_bytes())?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating tagged PDF fixtures for Phase 7.1.4 coverage check...");

    // Fixture 1: Suspects true, 60% coverage -> fallback to XY-cut
    write_pdf("tests/fixtures/tagged-suspects-true.pdf", true, 6, 10)?;
    println!("Created: tests/fixtures/tagged-suspects-true.pdf");
    println!("  - /MarkInfo /Suspects: true");
    println!("  - Coverage: 60% (6/10 MCIDs claimed)");
    println!("  - Expected: fallback to XY-cut, reading_order_algorithm = 'xy_cut'");

    // Fixture 2: Suspects false, 50% coverage -> trust StructTree
    write_pdf("tests/fixtures/tagged-suspects-false.pdf", false, 5, 10)?;
    println!("Created: tests/fixtures/tagged-suspects-false.pdf");
    println!("  - /MarkInfo /Suspects: false");
    println!("  - Coverage: 50% (5/10 MCIDs claimed)");
    println!("  - Expected: trust StructTree, reading_order_algorithm = 'struct_tree'");

    // Fixture 3: Suspects true, 95% coverage -> trust StructTree
    write_pdf("tests/fixtures/tagged-suspects-true-high-coverage.pdf", true, 19, 20)?;
    println!("Created: tests/fixtures/tagged-suspects-true-high-coverage.pdf");
    println!("  - /MarkInfo /Suspects: true");
    println!("  - Coverage: 95% (19/20 MCIDs claimed)");
    println!("  - Expected: trust StructTree, reading_order_algorithm = 'struct_tree'");

    println!("\nAll fixtures generated successfully!");
    Ok(())
}
