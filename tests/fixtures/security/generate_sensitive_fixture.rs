//! Generate sensitive.pdf for TH-08 log audit test.
//!
//! Creates a password-protected PDF with unique, distinctive markers:
//! - Body text contains "UNIQUE-MARKER-IN-BODY-TEXT-7f9a"
//! - Password value is "UNIQUE-PASSWORD-FOR-TH08-7f9a"
//!
//! These markers are specifically designed to be unlikely to appear
//! in normal log output, making substring-based leak detection reliable.

use lopdf::dictionary;
use lopdf::object::{Dictionary, Object};
use lopdf::{Document, ObjectId};
use std::fs::File;
use std::io::Write;

const BODY_TEXT: &str = "UNIQUE-MARKER-IN-BODY-TEXT-7f9a";
const PASSWORD: &str = "UNIQUE-PASSWORD-FOR-TH08-7f9a";

fn create_sensitive_pdf() -> Document {
    let mut doc = Document::with_version("1.4");

    // Create a simple page with the unique marker content
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(1));
    pages_dict.set("Kids", Object::Array(vec![
        Object::Reference((1, 0).into()),
    ]));

    // Create the page
    let mut page_dict = Dictionary::new();
    page_dict.set("Type", "Page");
    page_dict.set("Parent", Object::Reference((0, 0).into()));
    page_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    page_dict.set("Resources", dictionary! {
        "Font" => dictionary! {
            "F1" => dictionary! {
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => "Helvetica"
            }
        }
    });

    // Content stream with the unique marker text
    let content = format!(
        "BT\n/F1 12 Tf\n100 700 Td\n({}) Tj\nET\n",
        BODY_TEXT
    );
    let content_bytes = content.as_bytes();
    let content_stream = doc.new_object_id();
    doc.objects.insert(content_stream, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content_bytes.to_vec()
    )));
    page_dict.set("Contents", Object::Reference(content_stream));

    let page_id = doc.add_object(page_dict);

    // Update pages dict with actual page reference
    pages_dict.set("Kids", Object::Array(vec![
        Object::Reference(page_id),
    ]));

    let pages_id = doc.add_object(pages_dict);

    // Update page parent reference
    if let Ok(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(page_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Set document ID (required for encryption)
    let id = b"th08-sensitive-pdf-7f9a\0\0\0\0\0\0\0\0\0\0\0\0";
    doc.trailer.set("ID", Object::Array(vec![
        Object::String(id.to_vec()),
        Object::String(id.to_vec()),
    ]));

    doc
}

fn main() {
    println!("Generating TH-08 sensitive fixture...");

    let mut doc = create_sensitive_pdf();

    // Encrypt with the unique password
    let user_password = PASSWORD.as_bytes();
    let owner_password = b"";

    match doc.encrypt(user_password, owner_password) {
        Ok(_) => {
            let output_path = "tests/fixtures/security/sensitive.pdf";
            let mut file = File::create(output_path).unwrap();
            file.write_all(doc.to_vec().as_slice()).unwrap();
            println!("Created {}", output_path);
            println!("  Password: {}", PASSWORD);
            println!("  Body text marker: {}", BODY_TEXT);
        }
        Err(e) => {
            eprintln!("Failed to create encrypted PDF: {}", e);
            std::process::exit(1);
        }
    }
}
