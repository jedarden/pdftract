#![cfg(feature = "decrypt")]

use pdftract_core::encryption::rc4::{derive_file_key, validate_user_password};
use pdftract_core::{extract_pdf, ExtractionOptions};
use secrecy::SecretString;
use std::path::PathBuf;

fn fixture(name: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/encrypted")
        .join(name)
}

fn extracted_text(name: &str, password: &str) -> String {
    let mut options = ExtractionOptions::default();
    options.password = Some(SecretString::new(password.to_owned().into()));
    let result = extract_pdf(&fixture(name), &options)
        .unwrap_or_else(|error| panic!("{name} should extract: {error}"));
    result
        .pages
        .iter()
        .flat_map(|page| page.spans.iter().map(|span| span.text.as_str()))
        .collect::<Vec<_>>()
        .join("\n")
}

#[test]
fn encrypted_fixtures_extract_text_with_password() {
    for fixture_name in [
        "EC-04-rc4-encrypted.pdf",
        "EC-05-aes128-encrypted.pdf",
        "EC-06-aes256-encrypted.pdf",
    ] {
        let text = extracted_text(fixture_name, "test");
        assert!(
            text.contains("Hello, World!"),
            "{fixture_name} did not yield decrypted text: {text:?}"
        );
    }
}

#[test]
fn encrypted_fixture_reports_wrong_password_diagnostic() {
    let mut options = ExtractionOptions::default();
    options.password = Some(SecretString::new("wrong".to_owned().into()));
    let error = extract_pdf(&fixture("EC-04-rc4-encrypted.pdf"), &options)
        .expect_err("wrong password must fail");
    let message = error.to_string();
    assert!(message.contains("ENCRYPTION_WRONG_PASSWORD"), "{message}");
}

#[test]
fn encrypted_fixture_reports_missing_password_diagnostic() {
    let error = extract_pdf(
        &fixture("EC-04-rc4-encrypted.pdf"),
        &ExtractionOptions::default(),
    )
    .expect_err("missing password must fail");
    let message = error.to_string();
    assert!(message.contains("ENCRYPTION_WRONG_PASSWORD"), "{message}");
}

#[test]
fn unsupported_encryption_reports_structured_diagnostic() {
    let error = extract_pdf(&fixture("livecycle.pdf"), &ExtractionOptions::default())
        .expect_err("unsupported encryption must fail");
    let message = error.to_string();
    assert!(message.contains("ENCRYPTION_UNSUPPORTED"), "{message}");
}

#[test]
fn aes128_fixture_password_vectors_match() {
    let key = derive_file_key(
        b"test",
        &hex_bytes("badad1e86442699427116d3e5d5271bc80a27814fc5e80f815efeef839354c5f"),
        0xffff_fbfcu32,
        &hex_bytes("0bacc6b9933ead86b7dca33b3a436cea"),
        128,
        4,
    );
    let key = key.key().expect("key derivation should succeed");
    assert_eq!(key, hex_bytes("32a1137e4b1a7664a40a47bbe337ac9b"));
    assert!(validate_user_password(
        b"test",
        key,
        &hex_bytes("e7514dced4772b04eeb8f49d7a8a5f650122456a91bae5134273a6db134c87c4"),
        &hex_bytes("0bacc6b9933ead86b7dca33b3a436cea"),
        4,
    ));
}

fn hex_bytes(value: &str) -> Vec<u8> {
    (0..value.len())
        .step_by(2)
        .map(|index| u8::from_str_radix(&value[index..index + 2], 16).unwrap())
        .collect()
}
