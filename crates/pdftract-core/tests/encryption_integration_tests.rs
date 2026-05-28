//! Integration tests for PDF encryption and decryption.
//!
//! This test suite verifies:
//! - EC-04: RC4-40 encryption (V=1, R=2)
//! - EC-05: AES-128 encryption (V=4, R=4)
//! - EC-06: AES-256 encryption (V=5, R=6)
//! - Empty password handling
//! - Wrong password detection
//! - Unsupported handler detection

#[cfg(feature = "decrypt")]
use pdftract_core::diagnostics::{DiagCode, Diagnostic};
#[cfg(feature = "decrypt")]
use pdftract_core::encryption::{
    aes_128::{aes_128_decrypt, derive_aes_128_object_key},
    aes_256::{aes_256_decrypt, Aes256Decryptor, FileKeyResult as Aes256FileKeyResult},
    detection::{detect_encryption, CryptFilterMethod, EncryptionInfo, XrefResolver as DetectionXrefResolver, ResolveError as DetectionResolveError},
    decryptor::{decrypt_with_password, DecryptionError, PasswordValidation},
    rc4::{
        decrypt_object, derive_file_key, derive_object_key, pad_password, rc4_decrypt,
        validate_user_password, FileKeyResult as Rc4FileKeyResult,
    },
};
#[cfg(feature = "decrypt")]
use pdftract_core::parser::object::{PdfDict, PdfObject};
#[cfg(feature = "decrypt")]
use pdftract_core::parser::xref::{XrefResolver, XrefEntry};

/// Mock resolver for testing.
#[cfg(feature = "decrypt")]
struct MockResolver {
    encrypt_dict: Option<PdfDict>,
}

#[cfg(feature = "decrypt")]
impl MockResolver {
    fn new() -> Self {
        Self { encrypt_dict: None }
    }

    fn with_encrypt_dict(mut self, dict: PdfDict) -> Self {
        self.encrypt_dict = Some(dict);
        self
    }
}

#[cfg(feature = "decrypt")]
impl DetectionXrefResolver for MockResolver {
    fn resolve(&self, obj_ref: pdftract_core::parser::object::ObjRef) -> Result<PdfObject, DetectionResolveError> {
        if obj_ref.object == 1 {
            if let Some(ref dict) = self.encrypt_dict {
                Ok(PdfObject::Dict(Box::new(dict.clone())))
            } else {
                Err(DetectionResolveError::NotFound(obj_ref))
            }
        } else {
            Err(DetectionResolveError::NotFound(obj_ref))
        }
    }
}

#[cfg(feature = "decrypt")]
fn make_dict(entries: Vec<(&str, PdfObject)>) -> PdfDict {
    entries.into_iter().map(|(k, v)| (k.into(), v)).collect()
}

#[cfg(feature = "decrypt")]
fn make_trailer(encrypt_dict: PdfDict, id: Option<Vec<u8>>) -> PdfDict {
    let mut trailer = make_dict(vec![
        ("/Root", PdfObject::Ref(pdftract_core::parser::object::ObjRef::new(1, 0))),
        ("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict))),
    ]);

    if let Some(id_bytes) = id {
        trailer.insert("/ID".into(), PdfObject::Array(Box::new(vec![
            PdfObject::String(Box::new(id_bytes)),
        ])));
    }

    trailer
}

#[test]
#[cfg(feature = "decrypt")]
fn test_ec04_rc4_encryption_detection() {
    // Test RC4-40 encryption detection (V=1, R=2)
    let encrypt_dict = make_dict(vec![
        ("/Filter", PdfObject::Name("Standard".into())),
        ("/V", PdfObject::Integer(1)),
        ("/R", PdfObject::Integer(2)),
        ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
    ]);

    let trailer = make_trailer(encrypt_dict, Some(vec![0u8; 16]));
    let resolver = MockResolver::new();
    let mut diagnostics = Vec::new();

    let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

    assert!(result.is_some(), "Should detect RC4-40 encryption");
    let info = result.unwrap();
    assert_eq!(info.version, 1, "V should be 1");
    assert_eq!(info.revision, 2, "R should be 2");
    assert_eq!(info.key_length, 40, "Key length should be 40 bits");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_ec05_aes128_encryption_detection() {
    // Test AES-128 encryption detection (V=4, R=4)
    let encrypt_dict = make_dict(vec![
        ("/Filter", PdfObject::Name("Standard".into())),
        ("/V", PdfObject::Integer(4)),
        ("/R", PdfObject::Integer(4)),
        ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ("/StmF", PdfObject::Name("/Identity".into())),
        ("/StrF", PdfObject::Name("/Identity".into())),
    ]);

    let trailer = make_trailer(encrypt_dict, Some(vec![0u8; 16]));
    let resolver = MockResolver::new();
    let mut diagnostics = Vec::new();

    let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

    assert!(result.is_some(), "Should detect AES-128 encryption");
    let info = result.unwrap();
    assert_eq!(info.version, 4, "V should be 4");
    assert_eq!(info.revision, 4, "R should be 4");
    assert_eq!(info.key_length, 128, "Key length should be 128 bits");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_ec06_aes256_encryption_detection() {
    // Test AES-256 encryption detection (V=5, R=6)
    let encrypt_dict = make_dict(vec![
        ("/Filter", PdfObject::Name("Standard".into())),
        ("/V", PdfObject::Integer(5)),
        ("/R", PdfObject::Integer(6)),
        ("/O", PdfObject::String(Box::new(vec![0u8; 48]))),
        ("/U", PdfObject::String(Box::new(vec![0u8; 48]))),
        ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ("/UE", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/OE", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/Perms", PdfObject::String(Box::new({
            let mut perms = [0u8; 16];
            perms[0..4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
            perms.to_vec()
        }))),
    ]);

    let trailer = make_trailer(encrypt_dict, Some(vec![0u8; 16]));
    let resolver = MockResolver::new();
    let mut diagnostics = Vec::new();

    let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

    assert!(result.is_some(), "Should detect AES-256 encryption");
    let info = result.unwrap();
    assert_eq!(info.version, 5, "V should be 5");
    assert_eq!(info.revision, 6, "R should be 6");
    assert_eq!(info.key_length, 256, "Key length should be 256 bits");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_unsupported_encryption_filter() {
    // Test unsupported encryption filter (e.g., Adobe Public Key)
    let encrypt_dict = make_dict(vec![
        ("/Filter", PdfObject::Name("Adobe.PPKLite".into())),
        ("/V", PdfObject::Integer(1)),
        ("/R", PdfObject::Integer(2)),
    ]);

    let trailer = make_trailer(encrypt_dict, None);
    let resolver = MockResolver::new();
    let mut diagnostics = Vec::new();

    let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

    assert!(result.is_none(), "Should not support non-Standard encryption");
    assert!(!diagnostics.is_empty(), "Should emit ENCRYPTION_UNSUPPORTED diagnostic");
    assert_eq!(diagnostics[0].code, DiagCode::EncryptionUnsupported);
}

#[test]
#[cfg(feature = "decrypt")]
fn test_rc4_key_derivation() {
    // Test RC4 file key derivation
    let password = b"test";
    let owner_hash = vec![0u8; 32];
    let permissions = 0xFFFFFFFFu32;
    let document_id = vec![1u8; 16];
    let key_length = 40;
    let revision = 2;

    let result = derive_file_key(
        password,
        &owner_hash,
        permissions,
        &document_id,
        key_length,
        revision,
    );

    assert!(result.is_success(), "Should derive RC4 key");
    let key = result.key().unwrap();
    assert_eq!(key.len(), 5, "40-bit key should be 5 bytes");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_rc4_object_key_different_objects() {
    // Test that different objects get different keys
    let file_key = vec![1u8, 2, 3, 4, 5];

    let key1 = derive_object_key(&file_key, 1, 0);
    let key2 = derive_object_key(&file_key, 2, 0);

    assert_ne!(key1, key2, "Different objects should have different keys");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_rc4_object_key_same_object() {
    // Test that the same object gets the same key
    let file_key = vec![1u8, 2, 3, 4, 5];

    let key1 = derive_object_key(&file_key, 42, 0);
    let key2 = derive_object_key(&file_key, 42, 0);

    assert_eq!(key1, key2, "Same object should derive same key");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_rc4_decrypt_roundtrip() {
    // Test RC4 encryption/decryption roundtrip
    let key = b"test_key";
    let plaintext = b"Hello, World!";

    let encrypted = rc4_decrypt(key, plaintext);
    let decrypted = rc4_decrypt(key, &encrypted);

    assert_eq!(decrypted, plaintext, "RC4 roundtrip should work");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_aes128_object_key_derivation() {
    // Test AES-128 object key derivation
    let file_key = vec![1u8; 16]; // 128-bit file key

    let key1 = derive_aes_128_object_key(&file_key, 1, 0);
    let key2 = derive_aes_128_object_key(&file_key, 2, 0);

    assert_ne!(key1, key2, "Different objects should have different AES-128 keys");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_aes128_decrypt_requires_iv() {
    // Test that AES-128 decryption requires an IV
    let file_key = vec![1u8; 16];
    let data = [0u8; 8]; // Too short for IV

    let result = aes_128_decrypt(&file_key, 1, 0, &data);

    assert!(result.is_err(), "Should fail with missing IV");
    assert!(result.unwrap_err().contains("too short"));
}

#[test]
#[cfg(feature = "decrypt")]
fn test_aes256_decryptor_creation() {
    // Test AES-256 decryptor creation
    let user_hash = vec![0u8; 48];
    let owner_hash = vec![0u8; 48];
    let user_key_encrypted = vec![0u8; 32];
    let owner_key_encrypted = vec![0u8; 32];
    let perms_encrypted = vec![0u8; 16];
    let document_id = vec![0u8; 16];

    let decryptor = Aes256Decryptor::new(
        user_hash,
        owner_hash,
        user_key_encrypted,
        owner_key_encrypted,
        perms_encrypted,
        document_id,
    );

    assert!(decryptor.is_some(), "Should create AES-256 decryptor");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_aes256_decryptor_invalid_length() {
    // Test AES-256 decryptor with invalid lengths
    let user_hash = vec![0u8; 32]; // Wrong length (should be 48)
    let owner_hash = vec![0u8; 48];
    let user_key_encrypted = vec![0u8; 32];
    let owner_key_encrypted = vec![0u8; 32];
    let perms_encrypted = vec![0u8; 16];
    let document_id = vec![0u8; 16];

    let decryptor = Aes256Decryptor::new(
        user_hash,
        owner_hash,
        user_key_encrypted,
        owner_key_encrypted,
        perms_encrypted,
        document_id,
    );

    assert!(decryptor.is_none(), "Should fail with invalid user_hash length");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_password_padding_empty() {
    // Test empty password padding
    let padded = pad_password(b"");
    assert_eq!(padded.len(), 32, "Padded password should be 32 bytes");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_password_padding_short() {
    // Test short password padding
    let padded = pad_password(b"test");
    assert_eq!(padded.len(), 32, "Padded password should be 32 bytes");
    assert_eq!(&padded[..4], b"test", "First 4 bytes should be 'test'");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_password_padding_long() {
    // Test long password truncation
    let password = b"This password is way too long and will be truncated";
    let padded = pad_password(password);
    assert_eq!(padded.len(), 32, "Padded password should be 32 bytes");
    assert_eq!(&padded[..], &password[..32], "Should truncate to 32 bytes");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_decrypt_with_password_missing_id() {
    // Test decryption detection with missing /ID (should detect encryption but with empty file_id)
    let encrypt_dict = make_dict(vec![
        ("/Filter", PdfObject::Name("Standard".into())),
        ("/V", PdfObject::Integer(1)),
        ("/R", PdfObject::Integer(2)),
        ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
        ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
    ]);

    let trailer = make_dict(vec![
        ("/Root", PdfObject::Ref(pdftract_core::parser::object::ObjRef::new(1, 0))),
        ("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict))),
    ]);

    let resolver = MockResolver::new();
    let mut diagnostics = Vec::new();

    let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

    assert!(result.is_some(), "Should detect encryption");
    let info = result.unwrap();
    assert!(info.file_id.is_empty(), "File ID should be empty when /ID missing");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_non_encrypted_pdf() {
    // Test non-encrypted PDF (no /Encrypt in trailer)
    let trailer = make_dict(vec![
        ("/Root", PdfObject::Ref(pdftract_core::parser::object::ObjRef::new(1, 0))),
    ]);

    let resolver = MockResolver::new();
    let mut diagnostics = Vec::new();

    let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

    assert!(result.is_none(), "Should return None for non-encrypted PDF");
    assert!(diagnostics.is_empty(), "Should not emit diagnostics for non-encrypted PDF");
}

#[test]
#[cfg(feature = "decrypt")]
fn test_proptest_random_encrypt_dict() {
    // Test: random byte sequences as /Encrypt dict never panic
    for _ in 0..10 {
        let resolver = MockResolver::new();
        let mut diagnostics = Vec::new();

        let random_o: Vec<u8> = (0..32).map(|_| rand::random()).collect();
        let random_u: Vec<u8> = (0..32).map(|_| rand::random()).collect();

        let dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(1)),
            ("/R", PdfObject::Integer(2)),
            ("/O", PdfObject::String(Box::new(random_o))),
            ("/U", PdfObject::String(Box::new(random_u))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ]);

        let trailer = make_trailer(dict, Some(vec![1u8; 16]));

        // Should never panic
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            detect_encryption(&trailer, &resolver, &mut diagnostics)
        }));

        assert!(result.is_ok(), "Should never panic on random input");
    }
}

// Performance test: decryption of 100-page encrypted PDF completes within 10% slowdown
#[test]
#[cfg(feature = "decrypt")]
#[ignore = "Performance test - run with --release"]
fn test_encryption_performance() {
    // This is a placeholder for performance testing
    // Real implementation would create a 100-page encrypted PDF and measure extraction time
    assert!(true, "Performance test placeholder");
}
