//! Generate a minimal valid tagged PDF for testing Phase 7.1.4 coverage check.
//!
//! This creates a PDF with:
//! - /MarkInfo /Suspects configurable
//! - StructTree with ParentTree
//! - MCID-based content association
//!
//! The PDF is minimal but valid, with correct xref table offsets.

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Generate fixture 1: Suspects true, low coverage -> XY-cut fallback
    generate_pdf("tests/fixtures/tagged-suspects-true.pdf", true, 6, 10)?;

    // Generate fixture 2: Suspects false, low coverage -> trust StructTree
    generate_pdf("tests/fixtures/tagged-suspects-false.pdf", false, 5, 10)?;

    // Generate fixture 3: Suspects true, high coverage -> trust StructTree
    generate_pdf("tests/fixtures/tagged-suspects-true-high-coverage.pdf", true, 19, 20)?;

    Ok(())
}

fn generate_pdf(path: &str, suspects: bool, num_claimed: usize, num_total: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut pdf_parts = Vec::new();

    // PDF header
    pdf_parts.push(b"%PDF-1.7\n".to_vec());

    // Object 1: Catalog
    let obj1 = format!(
        "1 0 obj\n\
        <<\n\
        /Type /Catalog\n\
        /Pages 2 0 R\n\
        /MarkInfo <<\n\
          /Marked true\n\
          /Suspects {}\n\
        >>\n\
        /StructTreeRoot 3 0 R\n\
        >>\n\
        endobj\n",
        if suspects { "true" } else { "false" }
    );
    pdf_parts.push(obj1.into_bytes());

    // Object 2: Pages
    let obj2 = "2 0 obj\n\
        <<\n\
        /Type /Pages\n\
        /Kids [4 0 R]\n\
        /Count 1\n\
        >>\n\
        endobj\n";
    pdf_parts.push(obj2.as_bytes().to_vec());
    pdf_parts.push(obj2.into_bytes());

    // Object 3: StructTreeRoot
    let obj3 = "3 0 obj\n\
        <<\n\
        /Type /StructTreeRoot\n\
        /K [5 0 R]\n\
        /ParentTree 6 0 R\n\
        >>\n\
        endobj\n".to_vec();
    pdf_parts.push(obj3);

    // Object 4: Page
    let obj4 = "4 0 obj\n\
        <<\n\
        /Type /Page\n\
        /Parent 2 0 R\n\
        /MediaBox [0 0 612 792]\n\
        /Contents 7 0 R\n\
        /StructParents 0\n\
        /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>\n\
        >>\n\
        endobj\n".to_vec();
    pdf_parts.push(obj4);

    // Object 5: StructElem (paragraph) with MCID array
    let mcid_array: Vec<String> = (0..num_total).map(|i| i.to_string()).collect();
    let obj5 = format!(
        "5 0 obj\n\
        <<\n\
        /Type /StructElem\n\
        /S /P\n\
        /K [{}]\n\
        >>\n\
        endobj\n",
        mcid_array.join(" ")
    );
    pdf_parts.push(obj5.into_bytes());

    // Object 6: ParentTree (number tree with /Nums array)
    let mut parent_tree_entries = Vec::new();
    for i in 0..num_total {
        if i < num_claimed {
            parent_tree_entries.push("5 0 R".to_string());
        } else {
            parent_tree_entries.push("null".to_string());
        }
    }
    let obj6 = format!(
        "6 0 obj\n\
        <<\n\
        /Nums [\n\
        0 [{}]\n\
        ]\n\
        >>\n\
        endobj\n",
        parent_tree_entries.join(" ")
    );
    pdf_parts.push(obj6.into_bytes());

    // Object 7: Content stream
    let obj7 = "7 0 obj\n\
        <<\n\
        /Length 44\n\
        >>\n\
        stream\n\
        BT\n\
        /F1 12 Tf\n\
        100 700 Td\n\
        (Test) Tj\n\
        ET\n\
        endstream\n\
        endobj\n".to_vec();
    pdf_parts.push(obj7);

    // Build the PDF up to xref and calculate offsets
    let mut pdf_before_xref = Vec::new();
    for part in &pdf_parts {
        pdf_before_xref.extend_from_slice(part);
    }

    // Calculate object offsets
    let mut offsets = Vec::new();
    let mut current = 0;
    for part in &pdf_parts {
        offsets.push(current);
        current += part.len();
    }

    // xref starts after all objects
    let xref_offset = current;

    // Build xref table
    let mut xref = Vec::new();
    xref.push(b"xref\n".to_vec());
    xref.push(b"0 8\n".to_vec());
    xref.push(format!("{:010} 65535 f \n", 0).into_bytes());

    for offset in offsets {
        xref.push(format!("{:010} 00000 n \n", offset).into_bytes());
    }

    // Trailer
    let trailer = format!(
        "trailer\n\
        <<\n\
        /Size 8\n\
        /Root 1 0 R\n\
        >>\n\
        startxref\n\
        {}\n\
        %%EOF\n",
        xref_offset
    );

    // Combine everything
    let mut final_pdf = Vec::new();
    final_pdf.extend_from_slice(&pdf_before_xref);
    for part in xref {
        final_pdf.extend_from_slice(&part);
    }
    final_pdf.extend_from_slice(trailer.as_bytes());

    // Write to file
    let mut file = File::create(path)?;
    file.write_all(&final_pdf)?;

    eprintln!("Created: {}", path);
    eprintln!("  /Suspects: {}", suspects);
    eprintln!("  Coverage: {}/{} MCIDs claimed", num_claimed, num_total);

    Ok(())
}
