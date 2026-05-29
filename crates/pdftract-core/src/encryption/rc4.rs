//! RC4 decryption for PDF V=1 R=2 (40-bit) and V=2 R=3 (up to 128-bit).
//!
//! This module implements PDF RC4 decryption per PDF 1.7 spec (ISO 32000-1:2008),
//! section 7.6.4. It supports:
//! - V=1, R=2: RC4 40-bit
//! - V=2, R=3: RC4 40-128 bit
//!
//! # Key Derivation (Algorithm 2)
//!
//! The file encryption key is derived from:
//! 1. Pad password to 32 bytes via the standard padding string
//! 2. MD5 hash: pad || /O || /P (4 bytes LE) || first16(/ID\[0\])
//! 3. If R>=3: iterate MD5 50 times on the first n bytes (n = key_length/8)
//! 4. The first n bytes of the MD5 output is the encryption key
//!
//! # Per-Object Key Derivation (Algorithm 1)
//!
//! Each object uses a unique key derived from the file key:
//! 1. Take the encryption key + 3 bytes object number (LE) + 2 bytes generation (LE)
//! 2. MD5 hash; first (n+5) bytes (capped at 16) is the per-object key
//! 3. Initialize RC4 with this key; decrypt the object data
//!
//! # User Password Validation (Algorithm 4 for R=2, Algorithm 5 for R=3)
//!
//! - R=2: pad password; RC4-encrypt the 32-byte padding string with the file key;
//!   compare with /U
//! - R=3: pad password; MD5(pad || first16(/ID\[0\])); RC4 19 times with i^step key;
//!   compare first 16 bytes with first 16 of /U

#[cfg(feature = "decrypt")]
use md5::Md5;
#[cfg(feature = "decrypt")]
use digest::Digest;

/// The 32-byte standard password padding string from PDF spec Table 27.
///
/// This string is used to pad passwords to exactly 32 bytes when they are
/// shorter than 32 bytes. This is defined in PDF 1.7 spec Table 27.
const PASSWORD_PADDING: [u8; 32] = [
    0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA, 0x01,
    0x08, 0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE, 0x64, 0x53,
    0x69, 0x7A,
];

/// Maximum RC4 key length in bytes (128 bits = 16 bytes).
const MAX_KEY_LENGTH: usize = 16;

/// Minimum RC4 key length in bytes (40 bits = 5 bytes).
const MIN_KEY_LENGTH: usize = 5;

/// Result of file key derivation.
#[derive(Debug, Clone)]
pub enum FileKeyResult {
    /// Successfully derived file key
    Success(Vec<u8>),
    /// Wrong password (validation failed)
    WrongPassword,
    /// Invalid encryption data (malformed /O, /U, /ID)
    InvalidData(String),
}

impl FileKeyResult {
    /// Check if the result is successful.
    pub fn is_success(&self) -> bool {
        matches!(self, FileKeyResult::Success(_))
    }

    /// Get the file key if successful.
    pub fn key(&self) -> Option<&[u8]> {
        match self {
            FileKeyResult::Success(key) => Some(key),
            _ => None,
        }
    }
}

/// Pad a password to 32 bytes using the standard padding string.
///
/// If the password is less than 32 bytes, the padding string is appended
/// to fill to 32 bytes. If the password is 32 bytes or more, only the
/// first 32 bytes are used.
#[must_use]
pub fn pad_password(password: &[u8]) -> [u8; 32] {
    let mut padded = [0u8; 32];

    if password.is_empty() {
        // Empty password uses the padding string as-is
        padded.copy_from_slice(&PASSWORD_PADDING);
    } else {
        // Copy password bytes (up to 32)
        let copy_len = password.len().min(32);
        padded[..copy_len].copy_from_slice(&password[..copy_len]);

        // Fill remaining with padding string
        if copy_len < 32 {
            padded[copy_len..].copy_from_slice(&PASSWORD_PADDING[..32 - copy_len]);
        }
    }

    padded
}

/// Derive the file encryption key (Algorithm 2 from PDF spec 7.6.4.3).
///
/// # Arguments
///
/// * `password` - The user or owner password (empty byte slice for no password)
/// * `owner_hash` - The /O value from the encryption dictionary
/// * `permissions` - The /P value (4 bytes, little-endian)
/// * `document_id` - The first element of the /ID array (used in key derivation)
/// * `key_length` - The encryption key length in bits (40, 128, etc.)
/// * `revision` - The encryption revision (2 or 3)
///
/// # Returns
///
/// `FileKeyResult` with the derived key (length = key_length / 8 bytes).
#[cfg(feature = "decrypt")]
pub fn derive_file_key(
    password: &[u8],
    owner_hash: &[u8],
    permissions: u32,
    document_id: &[u8],
    key_length: u32,
    revision: u32,
) -> FileKeyResult {
    // Validate inputs
    let key_bytes = (key_length / 8) as usize;
    if key_bytes < MIN_KEY_LENGTH || key_bytes > MAX_KEY_LENGTH {
        return FileKeyResult::InvalidData(format!(
            "Invalid key length: {} bits (must be 40-128)",
            key_length
        ));
    }

    if document_id.len() < 16 {
        return FileKeyResult::InvalidData(
            "Document ID too short (must be at least 16 bytes)".to_string(),
        );
    }

    // Step 1: Pad password to 32 bytes
    let padded_password = pad_password(password);

    // Step 2: MD5 hash: pad || /O || /P (4 bytes LE) || first16(/ID[0])
    let mut md5 = Md5::new();
    md5.update(&padded_password);
    md5.update(owner_hash);

    // Permissions as 4-byte little-endian
    let perm_bytes = permissions.to_le_bytes();
    md5.update(&perm_bytes);

    // First 16 bytes of document ID
    md5.update(&document_id[..16]);

    let mut hash = md5.finalize();

    // Step 3: If R>=3, iterate MD5 50 times on the first n bytes
    if revision >= 3 {
        for _ in 0..50 {
            let mut md5 = Md5::new();
            md5.update(&hash[..key_bytes]);
            hash = md5.finalize();
        }
    }

    // Step 4: The first n bytes of the MD5 output is the encryption key
    FileKeyResult::Success(hash[..key_bytes].to_vec())
}

/// Derive the per-object encryption key (Algorithm 1 from PDF spec 7.6.4.3).
///
/// # Arguments
///
/// * `file_key` - The file encryption key
/// * `object_number` - The PDF object number (0-based)
/// * `generation` - The PDF object generation number
///
/// # Returns
///
/// The per-object encryption key (length = min(file_key.len() + 5, 16) bytes).
#[cfg(feature = "decrypt")]
#[must_use]
pub fn derive_object_key(file_key: &[u8], object_number: u32, generation: u16) -> Vec<u8> {
    let key_len = std::cmp::min(file_key.len() + 5, 16);

    // Object number as 3-byte little-endian
    let obj_bytes = object_number.to_le_bytes();
    // Generation as 2-byte little-endian
    let gen_bytes = generation.to_le_bytes();

    let mut md5 = Md5::new();
    md5.update(file_key);
    md5.update(&obj_bytes[..3]); // First 3 bytes of object number
    md5.update(&gen_bytes); // Both bytes of generation number

    let hash = md5.finalize();
    hash[..key_len].to_vec()
}

/// Decrypt data using RC4 with the given key.
///
/// # Arguments
///
/// * `key` - The RC4 key
/// * `data` - The data to decrypt
///
/// # Returns
///
/// The decrypted data.
#[cfg(feature = "decrypt")]
pub fn rc4_decrypt(key: &[u8], data: &[u8]) -> Vec<u8> {
    // RC4 supports variable key sizes from 1-256 bytes
    // Implement RC4 directly since the rc4 crate has API compatibility issues
    rc4_decrypt_direct(key, data)
}

/// Direct RC4 implementation for PDF decryption.
///
/// RC4 is a simple stream cipher that generates a keystream by:
/// 1. Initialize a 256-byte S-box with the key
/// 2. Generate keystream bytes by swapping entries in the S-box
#[cfg(feature = "decrypt")]
fn rc4_decrypt_direct(key: &[u8], data: &[u8]) -> Vec<u8> {
    // Key scheduling algorithm (KSA)
    let mut s = [0u8; 256];
    for (i, s_i) in s.iter_mut().enumerate() {
        *s_i = i as u8;
    }

    let key_len = key.len();
    let mut j: u8 = 0;
    for i in 0..256 {
        j = j.wrapping_add(s[i]).wrapping_add(key[i % key_len]);
        s.swap(i, j as usize);
    }

    // Pseudo-random generation algorithm (PRGA)
    let mut result = data.to_vec();
    let mut i: u8 = 0;
    let mut j: u8 = 0;

    for (k, byte) in result.iter_mut().enumerate() {
        i = i.wrapping_add(1);
        j = j.wrapping_add(s[i as usize]);
        s.swap(i as usize, j as usize);

        let t = s[(s[i as usize].wrapping_add(s[j as usize])) as usize];
        *byte ^= t;
    }

    result
}

/// Decrypt a PDF object using the file encryption key (Algorithm 1).
///
/// This is the main entry point for decrypting PDF objects. It derives
/// the per-object key and decrypts the data.
///
/// # Arguments
///
/// * `file_key` - The file encryption key
/// * `object_number` - The PDF object number
/// * `generation` - The PDF object generation number
/// * `data` - The encrypted data
///
/// # Returns
///
/// The decrypted data.
#[cfg(feature = "decrypt")]
pub fn decrypt_object(
    file_key: &[u8],
    object_number: u32,
    generation: u16,
    data: &[u8],
) -> Vec<u8> {
    let object_key = derive_object_key(file_key, object_number, generation);
    rc4_decrypt(&object_key, data)
}

/// Validate user password for R=2 (Algorithm 4 from PDF spec 7.6.4.4).
///
/// # Arguments
///
/// * `password` - The user password to validate
/// * `file_key` - The file encryption key
/// * `user_hash` - The /U value from the encryption dictionary
///
/// # Returns
///
/// `true` if the password is correct, `false` otherwise.
#[cfg(feature = "decrypt")]
#[must_use]
pub fn validate_user_password_r2(password: &[u8], file_key: &[u8], user_hash: &[u8]) -> bool {
    // Step 1: Pad password to 32 bytes
    let padded_password = pad_password(password);

    // Step 2: RC4-encrypt the padding string with the file key
    let encrypted_padding = rc4_decrypt(file_key, &PASSWORD_PADDING);

    // Step 3: Compare with /U
    if user_hash.len() < 32 {
        return false;
    }

    &encrypted_padding[..32] == &user_hash[..32]
}

/// Validate user password for R=3 (Algorithm 5 from PDF spec 7.6.4.4).
///
/// # Arguments
///
/// * `password` - The user password to validate
/// * `file_key` - The file encryption key
/// * `user_hash` - The /U value from the encryption dictionary
/// * `document_id` - The first element of the /ID array
///
/// # Returns
///
/// `true` if the password is correct, `false` otherwise.
#[cfg(feature = "decrypt")]
#[must_use]
pub fn validate_user_password_r3(
    password: &[u8],
    file_key: &[u8],
    user_hash: &[u8],
    document_id: &[u8],
) -> bool {
    // Step 1: Pad password to 32 bytes
    let padded_password = pad_password(password);

    // Step 2: MD5 hash of padded password || first 16 bytes of document ID
    let mut md5 = Md5::new();
    md5.update(&padded_password);
    if document_id.len() >= 16 {
        md5.update(&document_id[..16]);
    }
    let hash = md5.finalize();

    // Step 3: RC4-encrypt the hash with the file key, 19 times
    let mut data = hash.to_vec();
    for i in 1..=19 {
        // XOR key with iteration counter for each round
        let mut key_copy = vec![0u8; file_key.len()];
        for (j, &byte) in file_key.iter().enumerate() {
            key_copy[j] = byte ^ (i as u8);
        }
        data = rc4_decrypt(&key_copy, &data);
    }

    // Step 4: Compare first 16 bytes with /U
    if user_hash.len() < 16 {
        return false;
    }

    &data[..16] == &user_hash[..16]
}

/// Validate user password (dispatches to R=2 or R=3 algorithm).
///
/// # Arguments
///
/// * `password` - The user password to validate
/// * `file_key` - The file encryption key
/// * `user_hash` - The /U value from the encryption dictionary
/// * `document_id` - The first element of the /ID array
/// * `revision` - The encryption revision (2 or 3)
///
/// # Returns
///
/// `true` if the password is correct, `false` otherwise.
#[cfg(feature = "decrypt")]
#[must_use]
pub fn validate_user_password(
    password: &[u8],
    file_key: &[u8],
    user_hash: &[u8],
    document_id: &[u8],
    revision: u32,
) -> bool {
    if revision == 2 {
        validate_user_password_r2(password, file_key, user_hash)
    } else if revision == 3 {
        validate_user_password_r3(password, file_key, user_hash, document_id)
    } else {
        false
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_password_padding_empty() {
        let padded = pad_password(b"");
        assert_eq!(padded, PASSWORD_PADDING);
    }

    #[test]
    fn test_password_padding_short() {
        let padded = pad_password(b"test");
        // First 4 bytes should be "test"
        assert_eq!(&padded[..4], b"test");
        // Remaining should be from padding string
        assert_eq!(&padded[4..], &PASSWORD_PADDING[..28]);
    }

    #[test]
    fn test_password_padding_exact() {
        let password = b"12345678901234567890123456789012"; // Exactly 32 bytes
        let padded = pad_password(password);
        assert_eq!(padded, *password);
    }

    #[test]
    fn test_password_padding_long() {
        let password = b"This password is way too long and will be truncated";
        let padded = pad_password(password);
        // Should only use first 32 bytes
        assert_eq!(&padded[..], &password[..32]);
    }

    #[test]
    fn test_derive_file_key_basic() {
        let password = b"test";
        let owner_hash = vec![0u8; 32];
        let permissions = 0xFFFFFFFFu32;
        let document_id = vec![0u8; 16];
        let key_length = 40; // 40-bit
        let revision = 2;

        let result = derive_file_key(
            password,
            &owner_hash,
            permissions,
            &document_id,
            key_length,
            revision,
        );

        assert!(result.is_success());
        let key = result.key().unwrap();
        assert_eq!(key.len(), 5); // 40 bits = 5 bytes
    }

    #[test]
    fn test_derive_file_key_128_bit() {
        let password = b"test";
        let owner_hash = vec![0u8; 32];
        let permissions = 0xFFFFFFFFu32;
        let document_id = vec![0u8; 16];
        let key_length = 128; // 128-bit
        let revision = 3;

        let result = derive_file_key(
            password,
            &owner_hash,
            permissions,
            &document_id,
            key_length,
            revision,
        );

        assert!(result.is_success());
        let key = result.key().unwrap();
        assert_eq!(key.len(), 16); // 128 bits = 16 bytes
    }

    #[test]
    fn test_derive_file_key_invalid_key_length() {
        let password = b"test";
        let owner_hash = vec![0u8; 32];
        let permissions = 0xFFFFFFFFu32;
        let document_id = vec![0u8; 16];
        let key_length = 256; // Too long for RC4
        let revision = 3;

        let result = derive_file_key(
            password,
            &owner_hash,
            permissions,
            &document_id,
            key_length,
            revision,
        );

        assert!(!result.is_success());
    }

    #[test]
    fn test_derive_file_key_short_document_id() {
        let password = b"test";
        let owner_hash = vec![0u8; 32];
        let permissions = 0xFFFFFFFFu32;
        let document_id = vec![0u8; 8]; // Too short
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

        assert!(!result.is_success());
    }

    #[test]
    fn test_derive_object_key() {
        let file_key = vec![1u8, 2, 3, 4, 5]; // 5-byte key
        let object_number = 100;
        let generation = 0;

        let object_key = derive_object_key(&file_key, object_number, generation);

        // Key should be min(5 + 5, 16) = 10 bytes
        assert_eq!(object_key.len(), 10);
    }

    #[test]
    fn test_rc4_decrypt_roundtrip() {
        let key = b"test_key";
        let plaintext = b"Hello, world!";

        // Encrypt (RC4 is symmetric, so decrypting is the same as encrypting)
        let encrypted = rc4_decrypt(key, plaintext);

        // Decrypt back
        let decrypted = rc4_decrypt(key, &encrypted);

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_decrypt_object_roundtrip() {
        let file_key = vec![1u8, 2, 3, 4, 5];
        let object_number = 42;
        let generation = 0;
        let plaintext = b"Secret object data";

        // Encrypt
        let encrypted = decrypt_object(&file_key, object_number, generation, plaintext);

        // Decrypt (should get original back since RC4 is symmetric)
        let decrypted = decrypt_object(&file_key, object_number, generation, &encrypted);

        assert_eq!(decrypted, plaintext);
    }

    #[test]
    fn test_validate_user_password_r2() {
        // This is a basic structure test - full validation requires real PDF test vectors
        let file_key = vec![1u8, 2, 3, 4, 5];
        let password = b"test";

        // Create a fake user_hash by encrypting the padding string
        let user_hash = rc4_decrypt(&file_key, &PASSWORD_PADDING);

        assert!(validate_user_password_r2(password, &file_key, &user_hash));
    }

    #[test]
    fn test_validate_user_password_r2_wrong_password() {
        let file_key = vec![1u8, 2, 3, 4, 5];
        let password = b"test";

        // Create a user_hash for a different password
        let wrong_password = pad_password(b"wrong");
        let mut md5 = Md5::new();
        md5.update(&wrong_password);
        md5.update(&[0u8; 32]); // fake owner_hash
        md5.update(&0xFFFFFFFFu32.to_le_bytes());
        md5.update(&[0u8; 16]); // fake document_id
        let wrong_key = md5.finalize();
        let user_hash = rc4_decrypt(&wrong_key[..5], &PASSWORD_PADDING);

        assert!(!validate_user_password_r2(password, &file_key, &user_hash));
    }

    #[test]
    fn test_file_key_result_is_success() {
        let key = vec![1u8, 2, 3, 4, 5];
        let result = FileKeyResult::Success(key.clone());
        assert!(result.is_success());
        assert_eq!(result.key(), Some(&key[..]));
    }

    #[test]
    fn test_file_key_result_wrong_password() {
        let result = FileKeyResult::WrongPassword;
        assert!(!result.is_success());
        assert_eq!(result.key(), None);
    }

    #[test]
    fn test_rc4_different_objects_different_keys() {
        let file_key = vec![1u8, 2, 3, 4, 5];

        let key1 = derive_object_key(&file_key, 1, 0);
        let key2 = derive_object_key(&file_key, 2, 0);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_rc4_same_object_same_key() {
        let file_key = vec![1u8, 2, 3, 4, 5];

        let key1 = derive_object_key(&file_key, 42, 0);
        let key2 = derive_object_key(&file_key, 42, 0);

        assert_eq!(key1, key2);
    }

    #[test]
    fn test_rc4_generation_affects_key() {
        let file_key = vec![1u8, 2, 3, 4, 5];

        let key1 = derive_object_key(&file_key, 42, 0);
        let key2 = derive_object_key(&file_key, 42, 1);

        assert_ne!(key1, key2);
    }

    #[test]
    fn test_password_padding_all_bytes() {
        // Test that all padding bytes are correctly defined
        assert_eq!(PASSWORD_PADDING.len(), 32);
        assert_eq!(
            PASSWORD_PADDING,
            [
                0x28, 0xBF, 0x4E, 0x5E, 0x4E, 0x75, 0x8A, 0x41, 0x64, 0x00, 0x4E, 0x56, 0xFF, 0xFA,
                0x01, 0x08, 0x2E, 0x2E, 0x00, 0xB6, 0xD0, 0x68, 0x3E, 0x80, 0x2F, 0x0C, 0xA9, 0xFE,
                0x64, 0x53, 0x69, 0x7A
            ]
        );
    }

    #[test]
    fn test_rc4_decrypt_empty_data() {
        let key = b"test_key";
        let data = b"";

        let result = rc4_decrypt(key, data);

        assert_eq!(result, Vec::<u8>::new());
    }

    #[test]
    fn test_rc4_decrypt_long_key() {
        // Test with a longer key (16 bytes = 128 bits)
        let key = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let plaintext = b"Hello, world!";

        let encrypted = rc4_decrypt(&key, plaintext);
        let decrypted = rc4_decrypt(&key, &encrypted);

        assert_eq!(decrypted, plaintext);
    }
}
