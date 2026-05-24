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

    // Create content stream with BDC/EMC marked content sequences for each MCID
    // Each MCID gets a marked content sequence
    let mut content_ops = String::new();
    for i in 0..num_total {
        content_ops.push_str(&format!(
            "BT\n/F1 12 Tf\n100 {} Td\n/MCID {} BDC\n(Test{}) Tj\nEMC\nET\n",
            700 - i * 15, // Move up for each MCID
            i,
            i
        ));
    }

    let content_length = content_ops.len();

    // Build the PDF content
    let pdf_body = format!(
        "%PDF-1.7\n
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
/Resources <<
/Font <<
/F1 <<
/Type /Font
/Subtype /Type1
/BaseFont /Helvetica
>>
>>
>>
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
/Length {}
>>
stream
{}
endstream
endobj
",
        if suspects { "true" } else { "false" },
        (0..num_total).map(|i| i.to_string()).collect::<Vec<_>>().join(" "),
        nums_content,
        content_length,
        content_ops
    );

    // Calculate xref offsets by searching for object markers
    // The offsets are from the beginning of the file (after %PDF-1.7\n)
    let mut offsets = vec![0u64; 8]; // 0-7 objects
    let mut current_offset = 10u64; // Start after "%PDF-1.7\n" (10 bytes)

    for i in 1..=7 {
        offsets[i] = current_offset;
        // Find the end of this object by searching for "endobj"
        let obj_marker = format!("{} 0 obj", i);
        let obj_start = pdf_body[current_offset as usize..].find(&obj_marker)
            .expect(&format!("Object {} not found", i));
        let obj_end = pdf_body[current_offset as usize + obj_start..].find("endobj")
            .expect(&format!("endobj for object {} not found", i));
        current_offset += (obj_start + obj_end + 6) as u64; // +6 for "endobj"
    }

    let xref_offset = current_offset;

    let xref_table = format!(
        "xref\n0 8\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<<\n/Size 8\n/Root 1 0 R\n>>\nstartxref\n{}\n%%EOF\n",
        offsets[1], offsets[2], offsets[3], offsets[4], offsets[5], offsets[6], offsets[7], xref_offset
    );

    let mut file = File::create(path)?;
    file.write_all(pdf_body.as_bytes())?;
    file.write_all(xref_table.as_bytes())?;

    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating tagged PDF fixtures for Phase 7.1.4 coverage check...");

    // Fixture 1: Suspects true, 60% coverage -> fallback to XY-cut
    write_pdf("tagged-suspects-true.pdf", true, 6, 10)?;
    println!("Created: tagged-suspects-true.pdf");
    println!("  - /MarkInfo /Suspects: true");
    println!("  - Coverage: 60% (6/10 MCIDs claimed)");
    println!("  - Expected: fallback to XY-cut, reading_order_algorithm = 'xy_cut'");

    // Fixture 2: Suspects false, 50% coverage -> trust StructTree
    write_pdf("tagged-suspects-false.pdf", false, 5, 10)?;
    println!("Created: tagged-suspects-false.pdf");
    println!("  - /MarkInfo /Suspects: false");
    println!("  - Coverage: 50% (5/10 MCIDs claimed)");
    println!("  - Expected: trust StructTree, reading_order_algorithm = 'struct_tree'");

    // Fixture 3: Suspects true, 95% coverage -> trust StructTree
    write_pdf("tagged-suspects-true-high-coverage.pdf", true, 19, 20)?;
    println!("Created: tagged-suspects-true-high-coverage.pdf");
    println!("  - /MarkInfo /Suspects: true");
    println!("  - Coverage: 95% (19/20 MCIDs claimed)");
    println!("  - Expected: trust StructTree, reading_order_algorithm = 'struct_tree'");

    println!("\nAll fixtures generated successfully!");
    Ok(())
}
