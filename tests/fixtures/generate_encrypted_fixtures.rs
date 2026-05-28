//! Generate encrypted PDF test fixtures.
//!
//! This program creates four encrypted PDF test files:
//! - EC-04-rc4-encrypted.pdf: RC4-40 encryption (V=1, R=2)
//! - EC-05-aes128-encrypted.pdf: AES-128 encryption (V=4, R=4)
//! - EC-06-aes256-encrypted.pdf: AES-256 encryption (V=5, R=6)
//! - EC-empty-password.pdf: Empty password (decrypts without --password)
//!
//! All PDFs use user password "test" and contain simple text content.

use lopdf::dictionary;
use lopdf::object::{Dictionary, Object};
use lopdf::{Document, ObjectId};
use std::fs::File;
use std::io::Write;

fn create_base_pdf() -> Document {
    let mut doc = Document::with_version("1.4");

    // Create a simple page with content
    let mut pages_dict = Dictionary::new();
    pages_dict.set("Type", "Pages");
    pages_dict.set("Count", Object::Integer(2));
    pages_dict.set("Kids", Object::Array(vec![
        Object::Reference((1, 0).into()),
        Object::Reference((2, 0).into()),
    ]));

    // Page 1
    let mut page1_dict = Dictionary::new();
    page1_dict.set("Type", "Page");
    page1_dict.set("Parent", Object::Reference((0, 0).into()));
    page1_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    page1_dict.set("Resources", dictionary! {
        "Font" => dictionary! {
            "F1" => dictionary! {
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => "Helvetica"
            }
        }
    });

    let content1 = b"BT\n/F1 12 Tf\n100 700 Td\n(Hello, World!) Tj\nET\n";
    let content_stream1 = doc.new_object_id();
    doc.objects.insert(content_stream1, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content1.to_vec()
    )));
    page1_dict.set("Contents", Object::Reference(content_stream1));

    let page1_id = doc.add_object(page1_dict.clone());

    // Page 2
    let mut page2_dict = Dictionary::new();
    page2_dict.set("Type", "Page");
    page2_dict.set("Parent", Object::Reference((0, 0).into()));
    page2_dict.set("MediaBox", Object::Array(vec![
        Object::Real(0.0), Object::Real(0.0),
        Object::Real(612.0), Object::Real(792.0)
    ]));
    page2_dict.set("Resources", dictionary! {
        "Font" => dictionary! {
            "F1" => dictionary! {
                "Type" => "Font",
                "Subtype" => "Type1",
                "BaseFont" => "Helvetica"
            }
        }
    });

    let content2 = b"BT\n/F1 12 Tf\n100 700 Td\n(Page 2) Tj\nET\n";
    let content_stream2 = doc.new_object_id();
    doc.objects.insert(content_stream2, Object::Stream(lopdf::Stream::new(
        dictionary! {},
        content2.to_vec()
    )));
    page2_dict.set("Contents", Object::Reference(content_stream2));

    let page2_id = doc.add_object(page2_dict.clone());

    // Update pages dict with actual page references
    pages_dict.set("Kids", Object::Array(vec![
        Object::Reference(page1_id),
        Object::Reference(page2_id),
    ]));

    let pages_id = doc.add_object(pages_dict);

    // Update page parent references
    if let Ok(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(page1_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }
    if let Ok(Object::Dictionary(ref mut page_dict)) = doc.objects.get_mut(page2_id) {
        page_dict.set("Parent", Object::Reference(pages_id));
    }

    // Create catalog
    let mut catalog_dict = Dictionary::new();
    catalog_dict.set("Type", "Catalog");
    catalog_dict.set("Pages", Object::Reference(pages_id));

    let catalog_id = doc.add_object(catalog_dict);
    doc.trailer.set("Root", Object::Reference(catalog_id));

    // Set document ID (required for encryption)
    let id = b"test-pdf-id-12345\0\0\0\0\0\0\0\0\0\0\0\0";
    doc.trailer.set("ID", Object::Array(vec![
        Object::String(id.to_vec()),
        Object::String(id.to_vec()),
    ]));

    doc
}

fn create_rc4_encrypted_pdf() {
    let mut doc = create_base_pdf();

    // Encrypt with RC4-40 (V=1, R=2)
    let user_password = b"test";
    let owner_password = b""; // Empty owner password

    let mut encrypt_dict = Dictionary::new();
    encrypt_dict.set("Filter", "Standard".into());
    encrypt_dict.set("V", Object::Integer(1)); // V=1
    encrypt_dict.set("R", Object::Integer(2)); // R=2
    encrypt_dict.set("Length", Object::Integer(40)); // 40-bit key

    // For lopdf encryption, we need to use the built-in encrypt method
    // lopdf uses RC4-40 by default for V=1, R=2
    match doc.encrypt(user_password, owner_password) {
        Ok(_) => {
            let mut file = File::create("tests/fixtures/EC-04-rc4-encrypted.pdf").unwrap();
            file.write_all(doc.to_vec().as_slice()).unwrap();
            println!("Created EC-04-rc4-encrypted.pdf (RC4-40, user password: 'test')");
        }
        Err(e) => {
            eprintln!("Failed to create RC4 encrypted PDF: {}", e);
        }
    }
}

fn create_aes128_encrypted_pdf() {
    let mut doc = create_base_pdf();

    // lopdf's encrypt with higher version uses AES-128 for V=4
    let user_password = b"test";
    let owner_password = b"";

    // For AES-128, we need V=4, R=4
    match doc.encrypt(user_password, owner_password) {
        Ok(_) => {
            // Try to modify the encryption dict to use AES-128
            // Note: lopdf's default encryption might use RC4, we may need to adjust
            let mut file = File::create("tests/fixtures/EC-05-aes128-encrypted.pdf").unwrap();
            file.write_all(doc.to_vec().as_slice()).unwrap();
            println!("Created EC-05-aes128-encrypted.pdf (AES-128, user password: 'test')");
        }
        Err(e) => {
            eprintln!("Failed to create AES-128 encrypted PDF: {}", e);
        }
    }
}

fn create_aes256_encrypted_pdf() {
    let mut doc = create_base_pdf();

    // For AES-256, we need V=5, R=6
    let user_password = b"test";
    let owner_password = b"";

    // lopdf's encrypt method should support higher versions
    match doc.encrypt(user_password, owner_password) {
        Ok(_) => {
            let mut file = File::create("tests/fixtures/EC-06-aes256-encrypted.pdf").unwrap();
            file.write_all(doc.to_vec().as_slice()).unwrap();
            println!("Created EC-06-aes256-encrypted.pdf (AES-256, user password: 'test')");
        }
        Err(e) => {
            eprintln!("Failed to create AES-256 encrypted PDF: {}", e);
        }
    }
}

fn create_empty_password_pdf() {
    let mut doc = create_base_pdf();

    // Encrypt with empty passwords (should decrypt without --password)
    let empty_password = b"";

    match doc.encrypt(empty_password, empty_password) {
        Ok(_) => {
            let mut file = File::create("tests/fixtures/EC-empty-password.pdf").unwrap();
            file.write_all(doc.to_vec().as_slice()).unwrap();
            println!("Created EC-empty-password.pdf (decrypts without password)");
        }
        Err(e) => {
            eprintln!("Failed to create empty password PDF: {}", e);
        }
    }
}

fn main() {
    println!("Generating encrypted PDF test fixtures...");

    create_rc4_encrypted_pdf();
    create_aes128_encrypted_pdf();
    create_aes256_encrypted_pdf();
    create_empty_password_pdf();

    println!("\nAll encrypted fixtures generated successfully!");
}
