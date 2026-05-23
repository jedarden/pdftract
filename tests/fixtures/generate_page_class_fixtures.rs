/// Generate page classification test fixtures.
///
/// This creates 4 minimal PDF fixtures for page classification testing:
/// 1. vector_pure - Pure text PDF (born-digital)
/// 2. scanned_single - Image-only PDF (scanned)
/// 3. brokenvector_pdfa - PDF/A with invisible text over image
/// 4. hybrid_header_body - Text header + scanned body (hybrid)
///
/// Run with: cargo run --bin generate_page_class_fixtures

use std::io::Write;

/// Minimal PDF structure builder
struct PdfBuilder {
    objects: Vec<Vec<u8>>,
    xref: Vec<u64>,
}

impl PdfBuilder {
    fn new() -> Self {
        Self {
            objects: Vec::new(),
            xref: Vec::new(),
        }
    }

    /// Add an object and return its index (1-based)
    fn add_object(&mut self, data: &[u8]) -> usize {
        self.objects.push(data.to_vec());
        self.objects.len()
    }

    /// Build the complete PDF document
    fn build(mut self) -> Vec<u8> {
        let mut pdf = Vec::new();

        // PDF header
        pdf.write_all(b"%PDF-1.4\n").unwrap();

        // Write placeholder for xref table
        let _xref_offset = pdf.len();
        pdf.write_all(b"0000000000 65535 f \n").unwrap();

        // Write objects and record offsets
        self.xref.push(pdf.len() as u64);
        for obj in &self.objects {
            pdf.write_all(obj).unwrap();
        }

        // Write xref table
        let xref_start = pdf.len();
        pdf.write_all(b"xref\n").unwrap();
        pdf.write_all(format!("0 {}\n", self.objects.len() + 1).as_bytes()).unwrap();
        pdf.write_all(b"0000000000 65535 f \n").unwrap();
        for offset in &self.xref[1..] {
            pdf.write_all(format!("{:010} 00000 n \n", offset).as_bytes()).unwrap();
        }

        // Write trailer
        pdf.write_all(b"trailer\n").unwrap();
        pdf.write_all(b"<<\n").unwrap();
        pdf.write_all(format!("/Size {}\n", self.objects.len() + 1).as_bytes()).unwrap();
        pdf.write_all(b"/Root 1 0 R\n").unwrap();
        pdf.write_all(b">>\n").unwrap();
        pdf.write_all(b"startxref\n").unwrap();
        pdf.write_all(format!("{}\n", xref_start).as_bytes()).unwrap();
        pdf.write_all(b"%%EOF\n").unwrap();

        pdf
    }
}

/// Create a minimal pure vector PDF (text only)
fn create_vector_pure_pdf() -> Vec<u8> {
    let mut builder = PdfBuilder::new();

    // Catalog
    let catalog = b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n\n";
    builder.add_object(catalog);

    // Pages
    let pages = b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n\n";
    builder.add_object(pages);

    // Page (612x792 points = Letter)
    let page = b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents 4 0 R\n/Resources <<\n/Font <<\n/F1 5 0 R\n>>\n>>\n>>\nendobj\n\n";
    builder.add_object(page);

    // Content stream (simple text)
    let content = b"4 0 obj\n<< /Length 135 >>\nstream\nBT\n/F1 12 Tf\n50 700 Td\n(This is a pure vector PDF page with text content.) Tj\n0 -20 Td\n(Born-digital documents have selectable text.) Tj\nET\nendstream\nendobj\n\n";
    builder.add_object(content);

    // Font (Helvetica)
    let font = b"5 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n\n";
    builder.add_object(font);

    builder.build()
}

/// Create a minimal scanned PDF (image only)
fn create_scanned_single_pdf() -> Vec<u8> {
    let mut builder = PdfBuilder::new();

    // Catalog
    let catalog = b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n\n";
    builder.add_object(catalog);

    // Pages
    let pages = b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n\n";
    builder.add_object(pages);

    // Page
    let page = b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents 4 0 R\n/Resources <<\n/XObject <<\n/Im1 5 0 R\n>>\n>>\n>>\nendobj\n\n";
    builder.add_object(page);

    // Content stream (draw image)
    let content = b"4 0 obj\n<< /Length 67 >>\nstream\nq\n612 792 scale\n0 0 1 d1\n/Im1 Do\nQ\nendstream\nendobj\n\n";
    builder.add_object(content);

    // Image (1x1 white pixel - minimal valid image)
    // Using a minimal DCT-decoded (JPEG) image placeholder
    let image = b"5 0 obj\n<<\n/Type /XObject\n/Subtype /Image\n/Width 1\n/Height 1\n/BitsPerComponent 8\n/ColorSpace /DeviceGray\n/Length 8\n>>\nstream\n\xff\xff\xff\xff\xff\xff\xff\xff\nendstream\nendobj\n\n";
    builder.add_object(image);

    builder.build()
}

/// Create a minimal BrokenVector PDF (invisible text over image)
fn create_brokenvector_pdfa_pdf() -> Vec<u8> {
    let mut builder = PdfBuilder::new();

    // Catalog
    let catalog = b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n\n";
    builder.add_object(catalog);

    // Pages
    let pages = b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n\n";
    builder.add_object(pages);

    // Page
    let page = b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents 4 0 R\n/Resources <<\n/XObject <<\n/Im1 5 0 R\n>>\n/Font <<\n/F1 6 0 R\n>>\n>>\n>>\nendobj\n\n";
    builder.add_object(page);

    // Content stream (invisible text Tr=3 over image)
    let content = b"4 0 obj\n<< /Length 230 >>\nstream\nq\n612 792 scale\n0 0 1 d1\n/Im1 Do\nQ\nBT\n/F1 12 Tf\n50 700 Td\n3 Tr\n(This text is invisible but present for OCR overlay.) Tj\n0 -20 Td\n(BrokenVector pattern: invisible text layer over scan.) Tj\nET\nendstream\nendobj\n\n";
    builder.add_object(content);

    // Full-page image
    let image = b"5 0 obj\n<<\n/Type /XObject\n/Subtype /Image\n/Width 1\n/Height 1\n/BitsPerComponent 8\n/ColorSpace /DeviceGray\n/Length 8\n>>\nstream\n\xff\xff\xff\xff\xff\xff\xff\xff\nendstream\nendobj\n\n";
    builder.add_object(image);

    // Font
    let font = b"6 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n\n";
    builder.add_object(font);

    builder.build()
}

/// Create a minimal Hybrid PDF (text header + image body)
fn create_hybrid_header_body_pdf() -> Vec<u8> {
    let mut builder = PdfBuilder::new();

    // Catalog
    let catalog = b"1 0 obj\n<<\n/Type /Catalog\n/Pages 2 0 R\n>>\nendobj\n\n";
    builder.add_object(catalog);

    // Pages
    let pages = b"2 0 obj\n<<\n/Type /Pages\n/Kids [3 0 R]\n/Count 1\n>>\nendobj\n\n";
    builder.add_object(pages);

    // Page
    let page = b"3 0 obj\n<<\n/Type /Page\n/Parent 2 0 R\n/MediaBox [0 0 612 792]\n/Contents [4 0 R 5 0 R]\n/Resources <<\n/XObject <<\n/Im1 6 0 R\n>>\n/Font <<\n/F1 7 0 R\n>>\n>>\n>>\nendobj\n\n";
    builder.add_object(page);

    // Content stream 1 (text header - top 15% of page)
    let header = b"4 0 obj\n<< /Length 140 >>\nstream\nBT\n/F1 12 Tf\n50 750 Td\n(This is a text header in a hybrid document.) Tj\n0 -20 Td\n(The body below is a scanned image.) Tj\nET\nendstream\nendobj\n\n";
    builder.add_object(header);

    // Content stream 2 (image body - bottom 85% of page)
    let body = b"5 0 obj\n<< /Length 80 >>\nstream\nq\n0 118 612 674 re\nW n\n0 118 translate\n612 674 scale\n/Im1 Do\nQ\nendstream\nendobj\n\n";
    builder.add_object(body);

    // Body image
    let image = b"6 0 obj\n<<\n/Type /XObject\n/Subtype /Image\n/Width 1\n/Height 1\n/BitsPerComponent 8\n/ColorSpace /DeviceGray\n/Length 8\n>>\nstream\n\xff\xff\xff\xff\xff\xff\xff\xff\nendstream\nendobj\n\n";
    builder.add_object(image);

    // Font
    let font = b"7 0 obj\n<<\n/Type /Font\n/Subtype /Type1\n/BaseFont /Helvetica\n>>\nendobj\n\n";
    builder.add_object(font);

    builder.build()
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("Generating page classification fixtures...\n");

    // Create vector_pure fixture
    println!("Creating vector_pure fixture...");
    let vector_pdf = create_vector_pure_pdf();
    let vector_path = "tests/fixtures/page_class/vector_pure/source.pdf";
    let vector_len = vector_pdf.len();
    std::fs::write(vector_path, vector_pdf)?;
    println!("  Wrote {} bytes to {}", vector_len, vector_path);

    // Create scanned_single fixture
    println!("Creating scanned_single fixture...");
    let scanned_pdf = create_scanned_single_pdf();
    let scanned_path = "tests/fixtures/page_class/scanned_single/source.pdf";
    let scanned_len = scanned_pdf.len();
    std::fs::write(scanned_path, scanned_pdf)?;
    println!("  Wrote {} bytes to {}", scanned_len, scanned_path);

    // Create brokenvector_pdfa fixture
    println!("Creating brokenvector_pdfa fixture...");
    let broken_pdf = create_brokenvector_pdfa_pdf();
    let broken_path = "tests/fixtures/page_class/brokenvector_pdfa/source.pdf";
    let broken_len = broken_pdf.len();
    std::fs::write(broken_path, broken_pdf)?;
    println!("  Wrote {} bytes to {}", broken_len, broken_path);

    // Create hybrid_header_body fixture
    println!("Creating hybrid_header_body fixture...");
    let hybrid_pdf = create_hybrid_header_body_pdf();
    let hybrid_path = "tests/fixtures/page_class/hybrid_header_body/source.pdf";
    let hybrid_len = hybrid_pdf.len();
    std::fs::write(hybrid_path, hybrid_pdf)?;
    println!("  Wrote {} bytes to {}", hybrid_len, hybrid_path);

    println!("\nAll PDF fixtures generated successfully!");
    Ok(())
}
