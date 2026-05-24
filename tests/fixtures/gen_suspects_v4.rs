//! Generate a minimal valid tagged PDF for testing Phase 7.1.4 coverage check.

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    generate_pdf("tests/fixtures/tagged-suspects-true.pdf", true, 6, 10)?;
    generate_pdf("tests/fixtures/tagged-suspects-false.pdf", false, 5, 10)?;
    generate_pdf("tests/fixtures/tagged-suspects-true-high-coverage.pdf", true, 19, 20)?;
    Ok(())
}

fn generate_pdf(path: &str, suspects: bool, num_claimed: usize, num_total: usize) -> Result<(), Box<dyn std::error::Error>> {
    let mut pdf = String::from("%PDF-1.7\n");

    // Object 1: Catalog
    pdf.push_str(&format!(
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
    ));

    // Object 2: Pages
    pdf.push_str(
        "2 0 obj\n\
        <<\n\
        /Type /Pages\n\
        /Kids [4 0 R]\n\
        /Count 1\n\
        >>\n\
        endobj\n"
    );

    // Object 3: StructTreeRoot
    pdf.push_str(
        "3 0 obj\n\
        <<\n\
        /Type /StructTreeRoot\n\
        /K [5 0 R]\n\
        /ParentTree 6 0 R\n\
        >>\n\
        endobj\n"
    );

    // Object 4: Page
    pdf.push_str(
        "4 0 obj\n\
        <<\n\
        /Type /Page\n\
        /Parent 2 0 R\n\
        /MediaBox [0 0 612 792]\n\
        /Contents 7 0 R\n\
        /StructParents 0\n\
        /Resources << /Font << /F1 << /Type /Font /Subtype /Type1 /BaseFont /Helvetica >> >> >>\n\
        >>\n\
        endobj\n"
    );

    // Object 5: StructElem (paragraph) with MCID array
    let mcid_array: Vec<String> = (0..num_total).map(|i| i.to_string()).collect();
    pdf.push_str(&format!(
        "5 0 obj\n\
        <<\n\
        /Type /StructElem\n\
        /S /P\n\
        /K [{}]\n\
        >>\n\
        endobj\n",
        mcid_array.join(" ")
    ));

    // Object 6: ParentTree (number tree with /Nums array)
    let mut parent_tree_entries = Vec::new();
    for i in 0..num_total {
        if i < num_claimed {
            parent_tree_entries.push("5 0 R".to_string());
        } else {
            parent_tree_entries.push("null".to_string());
        }
    }
    pdf.push_str(&format!(
        "6 0 obj\n\
        <<\n\
        /Nums [\n\
        0 [{}]\n\
        ]\n\
        >>\n\
        endobj\n",
        parent_tree_entries.join(" ")
    ));

    // Object 7: Content stream
    pdf.push_str(
        "7 0 obj\n\
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
        endobj\n"
    );

    // Find the offset of each object by searching for "N 0 obj"
    let mut offsets = vec![0usize; 8]; // Index 0 is dummy, 1-7 are actual objects
    let mut current_pos = 0;
    let pdf_bytes = pdf.as_bytes();

    for n in 1..=7 {
        let pattern = format!("{} 0 obj\n", n);
        if let Some(pos) = pdf.find(&pattern) {
            offsets[n] = pos;
        }
    }

    // xref starts after all objects
    let xref_offset = pdf.len();

    // Build xref table
    pdf.push_str("xref\n");
    pdf.push_str("0 8\n");
    pdf.push_str("0000000000 65535 f \n");

    for n in 1..=7 {
        pdf.push_str(&format!("{:010} 00000 n \n", offsets[n]));
    }

    // Trailer
    pdf.push_str(&format!(
        "trailer\n\
        <<\n\
        /Size 8\n\
        /Root 1 0 R\n\
        >>\n\
        startxref\n\
        {}\n\
        %%EOF\n",
        xref_offset
    ));

    // Write to file
    let mut file = File::create(path)?;
    file.write_all(pdf.as_bytes())?;

    eprintln!("Created: {}", path);
    eprintln!("  /Suspects: {}", suspects);
    eprintln!("  Coverage: {}/{} MCIDs claimed", num_claimed, num_total);

    Ok(())
}
