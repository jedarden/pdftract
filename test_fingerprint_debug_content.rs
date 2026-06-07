// Test: Debug fingerprint content stream decoding
use pdftract_core::document::compute_pdf_fingerprint;

fn main() {
    let v1_path = std::path::PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v1.pdf");
    let v2_path = std::path::PathBuf::from("tests/fingerprint/fixtures/content_edit_one_glyph/v2.pdf");

    let fp1 = compute_pdf_fingerprint(&v1_path).expect("Failed to compute v1");
    let fp2 = compute_pdf_fingerprint(&v2_path).expect("Failed to compute v2");

    println!("v1 fingerprint: {}", fp1);
    println!("v2 fingerprint: {}", fp2);
    println!("Equal: {}", fp1 == fp2);

    // Let's also check file sizes
    let v1_meta = std::fs::metadata(&v1_path).unwrap();
    let v2_meta = std::fs::metadata(&v2_path).unwrap();
    println!("v1 size: {} bytes", v1_meta.len());
    println!("v2 size: {} bytes", v2_meta.len());

    // And file hashes
    use std::io::Read;
    let mut v1_bytes = Vec::new();
    let mut v2_bytes = Vec::new();
    std::fs::File::open(&v1_path).unwrap().read_to_end(&mut v1_bytes).unwrap();
    std::fs::File::open(&v2_path).unwrap().read_to_end(&mut v2_bytes).unwrap();

    use sha2::{Digest, Sha256};
    let v1_hash = Sha256::digest(&v1_bytes);
    let v2_hash = Sha256::digest(&v2_bytes);
    println!("v1 SHA256: {}", hex::encode(v1_hash));
    println!("v2 SHA256: {}", hex::encode(v2_hash));
}
