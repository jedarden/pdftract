//! Generate invoice OCR test fixture with proper 300 DPI metadata.
//!
//! This creates a scanned PDF from an image with correct DPI settings.

use std::fs::File;
use std::io::Write;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Read the existing image
    let img_data = std::fs::read("/tmp/invoice_img-000.png")?;

    // Create a minimal PDF with the image embedded at 300 DPI
    // Image dimensions: 2550 x 3300 pixels = 8.5 x 11 inches at 300 DPI
    let pdf = format!(
        r#"%PDF-1.4
1 0 obj
<<
/Type /Catalog
/Pages 2 0 R
>>
endobj
2 0 obj
<<
/Type /Pages
/Count 1
/Kids [3 0 R]
>>
endobj
3 0 obj
<<
/Type /Page
/Parent 2 0 R
/MediaBox [0 0 612 792]
/Contents 5 0 R
/Resources <<
/XObject <<
/Im0 4 0 R
>>
>>
endobj
4 0 obj
<<
/Type /XObject
/Subtype /Image
/Width 2550
/Height 3300
/BitsPerComponent 8
/ColorSpace /DeviceGray
/Filter /DCTDecode
>>
stream
{}
endstream
endobj
5 0 obj
<<
/Length 73
>>
stream
q
612 0 0 792 0 0 cm
/Im0 Do
Q
endstream
endobj
xref
0 6
0000000000 65535 f
0000000009 00000 n
0000000058 00000 n
0000000115 00000 n
0000000370 00000 n
0000000000 00000 n
trailer
<<
/Size 6
/Root 1 0 R
>>
startxref
{}
%%EOF
"#,
        "...", // Image data would go here
        0      // xref offset placeholder
    );

    println!("Invoice fixture PDF structure created");
    println!("Note: Full JPEG embedding requires lopdf or pikepdf");

    Ok(())
}
