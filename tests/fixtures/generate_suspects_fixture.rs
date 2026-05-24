//! Generate a tagged PDF with /MarkInfo /Suspects true for testing Phase 7.1.4
//!
//! This creates a minimal tagged PDF with:
//! - /MarkInfo /Suspects true
//! - /StructTreeRoot with structure elements
//! - ParentTree with 60% coverage (triggers fallback)
//!
//! Usage: cargo run --bin generate_suspects_fixture

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let output_path = "tests/fixtures/tagged-suspects-true.pdf";

    // Create a minimal PDF with /MarkInfo /Suspects true
    // This is a manually crafted PDF that demonstrates the fallback behavior

    let pdf_data = b"%PDF-1.7
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
/MarkInfo <<
  /Marked true
  /Suspects true
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
/K [0 1 2 3 4 5]
>>
endobj
6 0 obj
<<
/Nums [
  0 [5 0 R 5 0 R 5 0 R 5 0 R 5 0 R 5 0 R null null null null]
]
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
0000000099 00000 n
0000000163 00000 n
0000000245 00000 n
0000000341 00000 n
0000000413 00000 n
0000000539 00000 n
trailer
<<
/Size 8
/Root 1 0 R
>>
startxref
651
%%EOF";

    let mut file = File::create(output_path)?;
    file.write_all(pdf_data)?;

    println!("Created fixture: {}", output_path);
    println!("This PDF has /MarkInfo /Suspects true and 60% StructTree coverage.");
    println!("Expected behavior: fallback to XY-cut, reading_order_algorithm = 'xy_cut'");

    Ok(())
}
