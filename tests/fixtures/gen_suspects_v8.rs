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

    // Build the PDF content objects
    let objects = vec![
        // Object 1: Catalog
        format!(
            "1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n/MarkInfo <<\n  /Marked true\n  /Suspects {}\n>>\n/StructTreeRoot 3 0 R\n>>\nendobj\n",
            if suspects { "true" } else { "false" }
        ),
        // Object 2: Pages
        "2 0 obj\n<<\n/Type /Pages\n/Kids [4 0 R]\n/Count 1\n>>\nendobj\n".to_string(),
        // Object 3: StructTreeRoot
        "3 0 obj\n<<\n/Type /StructTreeRoot\n/K [5 0 R]\n/ParentTree 6 0 R\n>>\nendobj\n".to_string(),
        // Object 4: Page
        format!(
            "4 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents 7 0 R\n/StructParents 0\n/Resources <<\n/Font <<\n/F1 <<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\n>>\n>>\n>>\nendobj\n"
        ),
        // Object 5: StructElem
        format!(
            "5 0 obj\n<<\n/Type /StructElem\n/S /P\n/K [{}]\n>>\nendobj\n",
            (0..num_total).map(|i| i.to_string()).collect::<Vec<_>>().join(" ")
        ),
        // Object 6: ParentTree
        format!(
            "6 0 obj\n<<\n{}>>\nendobj\n",
            nums_content
        ),
        // Object 7: Content stream
        format!(
            "7 0 obj\n<<\n/Length {}\n>>\nstream\n{}\nendstream\nendobj\n",
            content_length,
            content_ops
        ),
    ];

    // Calculate xref offsets
    let mut offsets = vec![0u64; 8]; // 0-7 objects
    offsets[0] = 0; // Object 0 is always free
    let mut current_offset = 10u64; // Start after "%PDF-1.7\n" (10 bytes)

    for (i, obj) in objects.iter().enumerate() {
        offsets[i + 1] = current_offset;
        current_offset += obj.len() as u64;
    }

    let xref_offset = current_offset;

    let xref_table = format!(
        "xref\n0 8\n0000000000 65535 f \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \n{:010} 00000 n \ntrailer\n<<\n/Size 8\n/Root 1 0 R\n>>\nstartxref\n{}\n%%EOF\n",
        offsets[1], offsets[2], offsets[3], offsets[4], offsets[5], offsets[6], offsets[7], xref_offset
    );

    let mut file = File::create(path)?;
    file.write_all(b"%PDF-1.7\n")?;
    for obj in &objects {
        file.write_all(obj.as_bytes())?;
    }
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
