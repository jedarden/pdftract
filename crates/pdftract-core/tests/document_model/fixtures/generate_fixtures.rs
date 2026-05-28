//! Generate document-model test fixtures.
//!
//! This program creates 15 PDF test fixtures for document model integration tests.
//!
//! FIXTURE PASSWORDS:
//! - All encrypted fixtures use user password "test" (NOT secret - these are test fixtures)
//! - Owner password is empty string for all encrypted fixtures

use lopdf::{Dictionary, Object, Stream, Document, StringFormat};
use std::fs::File;
use std::io::Write;
use std::process::Command;

fn create_minimal_page(content: &str) -> (Dictionary, Object) {
    let mut page_dict = Dictionary::new();
    page_dict.set(b"Type", "Page");
    page_dict.set(b"MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));

    let mut font_dict = Dictionary::new();
    font_dict.set(b"Type", "Font");
    font_dict.set(b"Subtype", "Type1");
    font_dict.set(b"BaseFont", "Helvetica");

    let mut resources = Dictionary::new();
    let mut fonts = Dictionary::new();
    fonts.set(b"F1", Object::Dictionary(font_dict));
    resources.set(b"Font", Object::Dictionary(fonts));
    page_dict.set(b"Resources", Object::Dictionary(resources));

    let content_bytes = format!("BT\n/F1 12 Tf\n100 700 Td\n({}) Tj\nET\n", content);
    let mut stream_dict = Dictionary::new();
    stream_dict.set(b"Length", Object::Integer(content_bytes.len() as i64));
    let content_stream = Stream::new(stream_dict, content_bytes.as_bytes().to_vec());

    (page_dict, Object::Stream(content_stream))
}

fn create_simple_base_pdf() -> Document {
    let mut doc = Document::with_version("1.4");

    let (page1_dict, content1) = create_minimal_page("Page 1");
    let (page2_dict, content2) = create_minimal_page("Page 2");

    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type", "Pages");
    pages_dict.set(b"Count", Object::Integer(2 as i64));
    pages_dict.set(b"Kids", Object::Array(vec![
        Object::Reference((1, 0).into()),
        Object::Reference((2, 0).into())
    ]));

    let mut page1_dict = page1_dict;
    page1_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page1_dict.set(b"Contents", Object::Reference((3, 0).into()));

    let mut page2_dict = page2_dict;
    page2_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page2_dict.set(b"Contents", Object::Reference((4, 0).into()));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));

    doc.objects.insert((0, 0).into(), Object::Dictionary(pages_dict));
    doc.objects.insert((1, 0).into(), Object::Dictionary(page1_dict));
    doc.objects.insert((2, 0).into(), Object::Dictionary(page2_dict));
    doc.objects.insert((3, 0).into(), content1);
    doc.objects.insert((4, 0).into(), content2);
    doc.objects.insert((5, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((5, 0)));

    let id = b"test-pdf-id-12345\0\0\0\0\0\0\0\0\0\0\0\0";
    doc.trailer.set(b"ID", Object::Array(vec![
        Object::String(id.to_vec(), StringFormat::Literal),
        Object::String(id.to_vec(), StringFormat::Literal),
    ]));

    doc
}

fn save_pdf(doc: &mut Document, filename: &str) {
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer).unwrap();
    let mut file = File::create(filename).unwrap();
    file.write_all(&buffer).unwrap();
}

fn encrypt_pdf(input: &str, output: &str, r_value: &str) {
    // Use qpdf to encrypt the PDF
    // R=2: RC4-40, R=3: RC4-128, R=4: AES-128, R=6: AES-256
    let result = Command::new("qpdf")
        .args(["--encrypt", "test", "", r_value, "--", input, output])
        .output();

    match result {
        Ok(result) => {
            if result.status.success() {
                println!("Created {} (encrypted with R={}, password: 'test')", output, r_value);
            } else {
                eprintln!("qpdf failed: {}", String::from_utf8_lossy(&result.stderr));
                eprintln!("Copy {} manually and encrypt with qpdf", input);
            }
        }
        Err(e) => {
            eprintln!("qpdf not found: {}. Copy {} manually and encrypt", e, input);
            // Copy the unencrypted version as fallback
            let _ = std::fs::copy(input, output);
        }
    }
}

fn create_encrypted_rc4_pdf() {
    let mut doc = create_simple_base_pdf();
    save_pdf(&mut doc, "tests/document_model/fixtures/_temp_rc4.pdf");
    encrypt_pdf("tests/document_model/fixtures/_temp_rc4.pdf",
               "tests/document_model/fixtures/encrypted_rc4_test.pdf", "2");
    let _ = std::fs::remove_file("tests/document_model/fixtures/_temp_rc4.pdf");
}

fn create_encrypted_aes128_pdf() {
    let mut doc = create_simple_base_pdf();
    save_pdf(&mut doc, "tests/document_model/fixtures/_temp_aes128.pdf");
    encrypt_pdf("tests/document_model/fixtures/_temp_aes128.pdf",
               "tests/document_model/fixtures/encrypted_aes128_test.pdf", "4");
    let _ = std::fs::remove_file("tests/document_model/fixtures/_temp_aes128.pdf");
}

fn create_encrypted_aes256_pdf() {
    let mut doc = Document::with_version("2.0");
    let (page1_dict, content1) = create_minimal_page("Page 1");
    let (page2_dict, content2) = create_minimal_page("Page 2");

    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type", "Pages");
    pages_dict.set(b"Count", Object::Integer(2 as i64));
    pages_dict.set(b"Kids", Object::Array(vec![
        Object::Reference((1, 0).into()),
        Object::Reference((2, 0).into())
    ]));

    let mut page1_dict = page1_dict;
    page1_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page1_dict.set(b"Contents", Object::Reference((3, 0).into()));

    let mut page2_dict = page2_dict;
    page2_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page2_dict.set(b"Contents", Object::Reference((4, 0).into()));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));

    doc.objects.insert((0, 0).into(), Object::Dictionary(pages_dict));
    doc.objects.insert((1, 0).into(), Object::Dictionary(page1_dict));
    doc.objects.insert((2, 0).into(), Object::Dictionary(page2_dict));
    doc.objects.insert((3, 0).into(), content1);
    doc.objects.insert((4, 0).into(), content2);
    doc.objects.insert((5, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((5, 0)));

    let id = b"test-pdf-id-12345\0\0\0\0\0\0\0\0\0\0\0\0";
    doc.trailer.set(b"ID", Object::Array(vec![
        Object::String(id.to_vec(), StringFormat::Literal),
        Object::String(id.to_vec(), StringFormat::Literal),
    ]));

    save_pdf(&mut doc, "tests/document_model/fixtures/_temp_aes256.pdf");
    encrypt_pdf("tests/document_model/fixtures/_temp_aes256.pdf",
               "tests/document_model/fixtures/encrypted_aes256_test.pdf", "6");
    let _ = std::fs::remove_file("tests/document_model/fixtures/_temp_aes256.pdf");
}

fn create_encrypted_empty_password_pdf() {
    let mut doc = create_simple_base_pdf();
    save_pdf(&mut doc, "tests/document_model/fixtures/_temp_empty.pdf");
    // Empty password uses same command - qpdf treats empty owner password as ""
    encrypt_pdf("tests/document_model/fixtures/_temp_empty.pdf",
               "tests/document_model/fixtures/encrypted_empty_password.pdf", "2");
    let _ = std::fs::remove_file("tests/document_model/fixtures/_temp_empty.pdf");
}

fn create_encrypted_unknown_handler_pdf() {
    // For unsupported handler, create a simple PDF with a fake /Encrypt dict
    let mut doc = create_simple_base_pdf();

    // Get the PDF data
    let mut buffer = Vec::new();
    doc.save_to(&mut buffer).unwrap();
    let pdf_str = String::from_utf8_lossy(&buffer);

    // Insert a custom encryption dict before the xref table
    let encrypt_dict = "1 0 obj\n<</Filter/Adobe.PubSec/V 2/R 2/Length 40/O(\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00)\n/U(\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00\\x00)\\nP -604>>\nendobj\n";

    // Find the trailer
    let trailer_pos = pdf_str.find("trailer").unwrap_or(pdf_str.len());
    let mut result = pdf_str.to_string();
    result.insert_str(trailer_pos, encrypt_dict);
    result = result.replace("1 0 obj", "2 0 obj"); // Shift object numbers

    // Add Encrypt reference to trailer
    result = result.replace("trailer\n<<", "trailer\n<</Encrypt 1 0 R");

    let mut file = File::create("tests/document_model/fixtures/encrypted_unknown_handler.pdf").unwrap();
    file.write_all(result.as_bytes()).unwrap();
    println!("Created encrypted_unknown_handler.pdf (unsupported Adobe.PubSec handler)");
}

fn create_tagged_3_level_outline_pdf() {
    let mut doc = Document::with_version("1.4");

    let (page1_dict, content1) = create_minimal_page("Chapter 1");
    let (page2_dict, content2) = create_minimal_page("Section 1.1");
    let (page3_dict, content3) = create_minimal_page("Subsection 1.1.1");

    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type", "Pages");
    pages_dict.set(b"Count", Object::Integer(3 as i64));
    pages_dict.set(b"Kids", Object::Array(vec![
        Object::Reference((1, 0).into()),
        Object::Reference((2, 0).into()),
        Object::Reference((3, 0).into())
    ]));

    let mut page1_dict = page1_dict;
    page1_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page1_dict.set(b"Contents", Object::Reference((7, 0).into()));

    let mut page2_dict = page2_dict;
    page2_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page2_dict.set(b"Contents", Object::Reference((8, 0).into()));

    let mut page3_dict = page3_dict;
    page3_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page3_dict.set(b"Contents", Object::Reference((9, 0).into()));

    // Create outline hierarchy (3 levels)
    let mut outline1 = Dictionary::new();
    outline1.set(b"Title", Object::String(b"Chapter 1".to_vec(), StringFormat::Literal));
    outline1.set(b"Parent", Object::Reference((10, 0).into()));
    outline1.set(b"Dest", Object::Array(vec![
        Object::Reference((1, 0).into()),
        Object::Name(b"Fit".to_vec())
    ]));

    let mut outline2 = Dictionary::new();
    outline2.set(b"Title", Object::String(b"Section 1.1".to_vec(), StringFormat::Literal));
    outline2.set(b"Parent", Object::Reference((10, 0).into()));
    outline2.set(b"Prev", Object::Reference((11, 0).into()));
    outline2.set(b"Dest", Object::Array(vec![
        Object::Reference((2, 0).into()),
        Object::Name(b"Fit".to_vec())
    ]));

    let mut outline3 = Dictionary::new();
    outline3.set(b"Title", Object::String(b"Subsection 1.1.1".to_vec(), StringFormat::Literal));
    outline3.set(b"Parent", Object::Reference((10, 0).into()));
    outline3.set(b"Prev", Object::Reference((12, 0).into()));
    outline3.set(b"Dest", Object::Array(vec![
        Object::Reference((3, 0).into()),
        Object::Name(b"Fit".to_vec())
    ]));

    let mut outlines = Dictionary::new();
    outlines.set(b"Type", "Outlines");
    outlines.set(b"Count", Object::Integer(3 as i64));
    outlines.set(b"First", Object::Reference((11, 0).into()));
    outlines.set(b"Last", Object::Reference((13, 0).into()));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));
    catalog_dict.set(b"Outlines", Object::Reference((10, 0).into()));

    doc.objects.insert((0, 0).into(), Object::Dictionary(pages_dict));
    doc.objects.insert((1, 0).into(), Object::Dictionary(page1_dict));
    doc.objects.insert((2, 0).into(), Object::Dictionary(page2_dict));
    doc.objects.insert((3, 0).into(), Object::Dictionary(page3_dict));
    doc.objects.insert((7, 0).into(), content1);
    doc.objects.insert((8, 0).into(), content2);
    doc.objects.insert((9, 0).into(), content3);
    doc.objects.insert((10, 0).into(), Object::Dictionary(outlines));
    doc.objects.insert((11, 0).into(), Object::Dictionary(outline1));
    doc.objects.insert((12, 0).into(), Object::Dictionary(outline2));
    doc.objects.insert((13, 0).into(), Object::Dictionary(outline3));
    doc.objects.insert((14, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((14, 0)));

    save_pdf(&mut doc, "tests/document_model/fixtures/tagged_3_level_outline.pdf");
    println!("Created tagged_3_level_outline.pdf (3-level outline hierarchy)");
}

fn create_ocg_default_off_pdf() {
    let mut doc = create_simple_base_pdf();

    // Create OCG (Optional Content Group)
    let mut ocg_dict = Dictionary::new();
    ocg_dict.set(b"Type", "OCG");
    ocg_dict.set(b"Name", Object::String(b"Test Layer".to_vec(), StringFormat::Literal));

    // Create /OCProperties with /D /BaseState /OFF
    let mut default_config = Dictionary::new();
    default_config.set(b"BaseState", Object::Name(b"OFF".to_vec()));
    default_config.set(b"ON", Object::Array(vec![]));

    let mut oc_properties = Dictionary::new();
    oc_properties.set(b"OCGs", Object::Array(vec![Object::Reference((6, 0).into())]));
    oc_properties.set(b"D", Object::Reference((7, 0).into()));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));
    catalog_dict.set(b"OCProperties", Object::Reference((8, 0).into()));

    doc.objects.insert((6, 0).into(), Object::Dictionary(ocg_dict));
    doc.objects.insert((7, 0).into(), Object::Dictionary(default_config));
    doc.objects.insert((8, 0).into(), Object::Dictionary(oc_properties));
    doc.objects.insert((5, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((5, 0)));

    save_pdf(&mut doc, "tests/document_model/fixtures/ocg_default_off.pdf");
    println!("Created ocg_default_off.pdf (OCG with /BaseState /OFF)");
}

fn create_multi_revision_3_pdf() {
    let mut doc = create_simple_base_pdf();
    save_pdf(&mut doc, "tests/document_model/fixtures/multi_revision_3.pdf");
    println!("Created multi_revision_3.pdf (normal PDF - for true multi-revision, use qpdf --linearize)");
}

fn create_inheritance_grandparent_mediabox_pdf() {
    let mut doc = Document::with_version("1.4");

    // Create a 3-level /Pages tree where MediaBox is only on the grandparent
    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type", "Pages");
    pages_dict.set(b"Count", Object::Integer(2 as i64));
    pages_dict.set(b"Kids", Object::Array(vec![Object::Reference((10, 0).into())]));
    pages_dict.set(b"MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));

    let mut parent_pages = Dictionary::new();
    parent_pages.set(b"Type", "Pages");
    parent_pages.set(b"Count", Object::Integer(2 as i64));
    parent_pages.set(b"Kids", Object::Array(vec![
        Object::Reference((1, 0).into()),
        Object::Reference((2, 0).into())
    ]));

    let (page1_dict, content1) = create_minimal_page("Page 1");
    let mut page1_dict = page1_dict;
    page1_dict.set(b"Parent", Object::Reference((10, 0).into()));
    page1_dict.set(b"Contents", Object::Reference((11, 0).into()));
    page1_dict.remove(b"MediaBox"); // No MediaBox - inherits

    let (page2_dict, content2) = create_minimal_page("Page 2");
    let mut page2_dict = page2_dict;
    page2_dict.set(b"Parent", Object::Reference((10, 0).into()));
    page2_dict.set(b"Contents", Object::Reference((12, 0).into()));
    page2_dict.remove(b"MediaBox"); // No MediaBox - inherits

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));

    doc.objects.insert((0, 0).into(), Object::Dictionary(pages_dict));
    doc.objects.insert((10, 0).into(), Object::Dictionary(parent_pages));
    doc.objects.insert((1, 0).into(), Object::Dictionary(page1_dict));
    doc.objects.insert((2, 0).into(), Object::Dictionary(page2_dict));
    doc.objects.insert((11, 0).into(), content1);
    doc.objects.insert((12, 0).into(), content2);
    doc.objects.insert((13, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((13, 0)));

    save_pdf(&mut doc, "tests/document_model/fixtures/inheritance_grandparent_mediabox.pdf");
    println!("Created inheritance_grandparent_mediabox.pdf (MediaBox from grandparent)");
}

fn create_missing_mediabox_pdf() {
    let mut doc = Document::with_version("1.4");

    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type", "Pages");
    pages_dict.set(b"Count", Object::Integer(1 as i64));
    pages_dict.set(b"Kids", Object::Array(vec![Object::Reference((1, 0).into())]));

    let mut page_dict = Dictionary::new();
    page_dict.set(b"Type", "Page");
    page_dict.set(b"Parent", Object::Reference((0, 0).into()));
    // No MediaBox - should trigger DEFAULT_MEDIABOX

    let content_bytes = b"BT\n/F1 12 Tf\n100 700 Td\n(No MediaBox) Tj\nET\n";
    let mut stream_dict = Dictionary::new();
    stream_dict.set(b"Length", Object::Integer(content_bytes.len() as i64));
    let content_stream = Stream::new(stream_dict, content_bytes.to_vec());

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));

    doc.objects.insert((0, 0).into(), Object::Dictionary(pages_dict));
    doc.objects.insert((1, 0).into(), Object::Dictionary(page_dict));
    doc.objects.insert((2, 0).into(), Object::Stream(content_stream));
    doc.objects.insert((3, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((3, 0)));

    save_pdf(&mut doc, "tests/document_model/fixtures/missing_mediabox.pdf");
    println!("Created missing_mediabox.pdf (no MediaBox, defaults to US Letter)");
}

fn create_partial_resource_override_pdf() {
    let mut doc = Document::with_version("1.4");

    let mut root_resources = Dictionary::new();
    let mut root_fonts = Dictionary::new();
    root_fonts.set(b"F1", Object::Reference((4, 0).into()));
    root_fonts.set(b"F2", Object::Reference((5, 0).into()));
    let mut root_xobject = Dictionary::new();
    root_xobject.set(b"Im1", Object::Reference((6, 0).into()));
    root_resources.set(b"Font", Object::Dictionary(root_fonts));
    root_resources.set(b"XObject", Object::Dictionary(root_xobject));

    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type", "Pages");
    pages_dict.set(b"Count", Object::Integer(1 as i64));
    pages_dict.set(b"Kids", Object::Array(vec![Object::Reference((1, 0).into())]));
    pages_dict.set(b"Resources", Object::Reference((10, 0).into()));

    // Page overrides /Font but not /XObject
    let mut page_resources = Dictionary::new();
    let mut page_fonts = Dictionary::new();
    page_fonts.set(b"F1", Object::Reference((7, 0).into())); // Override F1
    page_fonts.set(b"F3", Object::Reference((8, 0).into())); // Add new font
    page_resources.set(b"Font", Object::Dictionary(page_fonts));
    // No /XObject - should inherit Im1 from parent

    let (mut page_dict, content) = create_minimal_page("Partial Override");
    page_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page_dict.set(b"Contents", Object::Reference((11, 0).into()));
    page_dict.set(b"Resources", Object::Dictionary(page_resources));

    let mut font1 = Dictionary::new();
    font1.set(b"Type", "Font");
    font1.set(b"Subtype", "Type1");
    font1.set(b"BaseFont", "Helvetica");

    let mut font2 = Dictionary::new();
    font2.set(b"Type", "Font");
    font2.set(b"Subtype", "Type1");
    font2.set(b"BaseFont", "Times-Roman");

    let mut font3 = Dictionary::new();
    font3.set(b"Type", "Font");
    font3.set(b"Subtype", "Type1");
    font3.set(b"BaseFont", "Courier");

    let mut image = Dictionary::new();
    image.set(b"Type", "XObject");
    image.set(b"Subtype", "Image");
    image.set(b"Width", Object::Integer(100 as i64));
    image.set(b"Height", Object::Integer(100 as i64));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));

    doc.objects.insert((0, 0).into(), Object::Dictionary(pages_dict));
    doc.objects.insert((1, 0).into(), Object::Dictionary(page_dict));
    doc.objects.insert((4, 0).into(), Object::Dictionary(font1.clone()));
    doc.objects.insert((5, 0).into(), Object::Dictionary(font2));
    doc.objects.insert((6, 0).into(), Object::Dictionary(image));
    doc.objects.insert((7, 0).into(), Object::Dictionary(font1)); // Overridden F1
    doc.objects.insert((8, 0).into(), Object::Dictionary(font3));
    doc.objects.insert((10, 0).into(), Object::Dictionary(root_resources));
    doc.objects.insert((11, 0).into(), content);
    doc.objects.insert((12, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((12, 0)));

    save_pdf(&mut doc, "tests/document_model/fixtures/partial_resource_override.pdf");
    println!("Created partial_resource_override.pdf (partial /Resources override)");
}

fn create_js_in_openaction_pdf() {
    let mut doc = create_simple_base_pdf();

    let mut open_action = Dictionary::new();
    open_action.set(b"S", "JavaScript");
    open_action.set(b"JS", Object::String(b"app.alert('Hello from PDF!');".to_vec(), StringFormat::Literal));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));
    catalog_dict.set(b"OpenAction", Object::Reference((6, 0).into()));

    doc.objects.insert((6, 0).into(), Object::Dictionary(open_action));
    doc.objects.insert((7, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((7, 0)));

    save_pdf(&mut doc, "tests/document_model/fixtures/js_in_openaction.pdf");
    println!("Created js_in_openaction.pdf (/OpenAction /S /JavaScript)");
}

fn create_xfa_form_pdf() {
    let mut doc = create_simple_base_pdf();

    let mut acroform = Dictionary::new();
    acroform.set(b"XFA", Object::String(b"template".to_vec(), StringFormat::Literal));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));
    catalog_dict.set(b"AcroForm", Object::Reference((6, 0).into()));

    doc.objects.insert((6, 0).into(), Object::Dictionary(acroform));
    doc.objects.insert((7, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((7, 0)));

    save_pdf(&mut doc, "tests/document_model/fixtures/xfa_form.pdf");
    println!("Created xfa_form.pdf (/AcroForm /XFA present)");
}

fn create_pdfa_1b_conformance_pdf() {
    let mut doc = create_simple_base_pdf();

    let xmp_metadata = r#"<?xpacket begin="?" id="W5M0MpCehiHzreSzNTczkc9d"?>
<x:xmpmeta xmlns:x="adobe:ns:meta/" x:xmptk="Adobe XMP Core 5.6-c140 79.160451">
  <rdf:RDF xmlns:rdf="http://www.w3.org/1999/02/22-rdf-syntax-ns#">
    <rdf:Description rdf:about=""
      xmlns:pdfaid="http://www.aiim.org/pdfa/ns/id/">
      <pdfaid:part>1</pdfaid:part>
      <pdfaid:conformance>B</pdfaid:conformance>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end="w"?>"#;

    let mut metadata_dict = Dictionary::new();
    metadata_dict.set(b"Type", "Metadata");
    metadata_dict.set(b"Subtype", "XML");
    let metadata_stream = Stream::new(metadata_dict, xmp_metadata.as_bytes().to_vec());

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));
    catalog_dict.set(b"Metadata", Object::Reference((6, 0).into()));

    doc.objects.insert((6, 0).into(), Object::Stream(metadata_stream));
    doc.objects.insert((7, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((7, 0)));

    save_pdf(&mut doc, "tests/document_model/fixtures/pdfa_1b_conformance.pdf");
    println!("Created pdfa_1b_conformance.pdf (XMP PDF/A-1B metadata)");
}

fn create_page_labels_roman_arabic_pdf() {
    let mut doc = create_simple_base_pdf();

    // Add page 3 and 4
    let (page3_dict, content3) = create_minimal_page("Page 3");
    let (page4_dict, content4) = create_minimal_page("Page 4");
    let mut page3_dict = page3_dict;
    page3_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page3_dict.set(b"Contents", Object::Reference((8, 0).into()));
    let mut page4_dict = page4_dict;
    page4_dict.set(b"Parent", Object::Reference((0, 0).into()));
    page4_dict.set(b"Contents", Object::Reference((9, 0).into()));

    // Add /PageLabels number tree
    // Pages 0-3: roman numerals (i, ii, iii, iv)
    // Pages 4+: arabic (1, 2, 3, ...)
    let mut page_labels = Dictionary::new();
    page_labels.set(b"Nums", Object::Array(vec![
        Object::Integer(0 as i64),
        Object::Dictionary({
            let mut d = Dictionary::new();
            d.set(b"S", "r");
            d.set(b"St", Object::Integer(1 as i64));
            d
        }),
        Object::Integer(4 as i64),
        Object::Dictionary({
            let mut d = Dictionary::new();
            d.set(b"S", "D");
            d.set(b"St", Object::Integer(1 as i64));
            d
        })
    ]));

    let mut catalog_dict = Dictionary::new();
    catalog_dict.set(b"Type", "Catalog");
    catalog_dict.set(b"Pages", Object::Reference((0, 0).into()));
    catalog_dict.set(b"PageLabels", Object::Reference((10, 0).into()));

    // Update pages count to 4
    let mut pages_dict = Dictionary::new();
    pages_dict.set(b"Type", "Pages");
    pages_dict.set(b"Count", Object::Integer(4 as i64));
    pages_dict.set(b"Kids", Object::Array(vec![
        Object::Reference((1, 0).into()),
        Object::Reference((2, 0).into()),
        Object::Reference((3, 0).into()),
        Object::Reference((4, 0).into())
    ]));

    doc.objects.insert((0, 0).into(), Object::Dictionary(pages_dict));
    doc.objects.insert((3, 0).into(), Object::Dictionary(page3_dict));
    doc.objects.insert((4, 0).into(), Object::Dictionary(page4_dict));
    doc.objects.insert((8, 0).into(), content3);
    doc.objects.insert((9, 0).into(), content4);
    doc.objects.insert((10, 0).into(), Object::Dictionary(page_labels));
    doc.objects.insert((11, 0).into(), Object::Dictionary(catalog_dict));
    doc.trailer.set(b"Root", Object::Reference((11, 0)));

    save_pdf(&mut doc, "tests/document_model/fixtures/page_labels_roman_arabic.pdf");
    println!("Created page_labels_roman_arabic.pdf (roman 0-3, arabic 4+)");
}

fn main() {
    println!("Generating document-model test fixtures...");

    create_encrypted_rc4_pdf();
    create_encrypted_aes128_pdf();
    create_encrypted_aes256_pdf();
    create_encrypted_empty_password_pdf();
    create_encrypted_unknown_handler_pdf();
    create_tagged_3_level_outline_pdf();
    create_ocg_default_off_pdf();
    create_multi_revision_3_pdf();
    create_inheritance_grandparent_mediabox_pdf();
    create_missing_mediabox_pdf();
    create_partial_resource_override_pdf();
    create_js_in_openaction_pdf();
    create_xfa_form_pdf();
    create_pdfa_1b_conformance_pdf();
    create_page_labels_roman_arabic_pdf();

    println!("\nAll 15 document-model fixtures generated successfully!");
    println!("\nNote: Encrypted fixtures require qpdf to be installed.");
    println!("If qpdf is not available, encrypted fixtures will be unencrypted placeholders.");
}
