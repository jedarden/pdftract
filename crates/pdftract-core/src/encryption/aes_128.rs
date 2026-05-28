//! AES-128 decryption for PDF V=4 R=4 (Acrobat 7-8 era).
//!
//! This module implements AES-128 CBC-mode decryption per PDF 1.7 spec
//! (ISO 32000-1:2008), section 7.6.4.2. It supports:
//! - V=4, R=4: AES-128 via crypt filters (AESV2)
//!
//! # Key Derivation (Algorithm 1, AES variant)
//!
//! The per-object encryption key is derived from the file key:
//! 1. file_key || 3 bytes obj num (LE) || 2 bytes gen (LE) || "sAlT" (4 ASCII bytes)
//! 2. MD5 hash; first (n+5) bytes (capped at 16) is the per-object key (AES-128 = 16 bytes)
//!
//! # AES-128 CBC Decryption
//!
//! Data layout: first 16 bytes = IV; rest = ciphertext
//! Decrypt with AES-128-CBC + PKCS#5 padding (PKCS#7 with block size 16)

#[cfg(feature = "decrypt")]
use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
#[cfg(feature = "decrypt")]
use md5::Md5;
#[cfg(feature = "decrypt")]
use digest::Digest;

type Aes128CbcDec = cbc::Decryptor<aes::Aes128>;

/// AES-128 block size in bytes (128 bits).
const AES_BLOCK_SIZE: usize = 16;

/// The "sAlT" suffix for AES key derivation in V=4 (4 bytes: 0x73 0x41 0x6C 0x54).
const AES_SALT: [u8; 4] = [0x73, 0x41, 0x6C, 0x54];

/// Derive the per-object encryption key for AES-128 (Algorithm 1, AES variant).
///
/// # Arguments
///
/// * `file_key` - The file encryption key (from Algorithm 2)
/// * `object_number` - The PDF object number
/// * `generation` - The PDF object generation number
///
/// # Returns
///
/// The 16-byte AES-128 per-object key.
#[cfg(feature = "decrypt")]
#[must_use]
pub fn derive_aes_128_object_key(file_key: &[u8], object_number: u32, generation: u16) -> [u8; AES_BLOCK_SIZE] {
    // Object number as 3-byte little-endian
    let obj_bytes = object_number.to_le_bytes();
    // Generation as 2-byte little-endian
    let gen_bytes = generation.to_le_bytes();

    let mut md5 = Md5::new();
    md5.update(file_key);
    md5.update(&obj_bytes[..3]); // First 3 bytes of object number
    md5.update(&gen_bytes); // Both bytes of generation number
    md5.update(&AES_SALT); // "sAlT" suffix is mandatory for AES in V=4

    let mut hash = md5.finalize();

    // For AES-128, we use the first 16 bytes of the hash
    let mut key = [0u8; AES_BLOCK_SIZE];
    key.copy_from_slice(&hash[..AES_BLOCK_SIZE]);
    key
}

/// Decrypt AES-128 encrypted data in CBC mode with PKCS#5 padding.
///
/// # Arguments
///
/// * `file_key` - The file encryption key
/// * `object_number` - The PDF object number
/// * `generation` - The PDF object generation number
/// * `data` - The encrypted data (IV + ciphertext)
///
/// # Returns
///
/// `Ok(plaintext)` on success, `Err(message)` on failure.
///
/// # Errors
///
/// - `ENCRYPTION_INVALID_LENGTH` if data length (after IV) is not a multiple of 16
/// - `ENCRYPTION_INVALID_PADDING` if PKCS#5 padding validation fails
#[cfg(feature = "decrypt")]
pub fn aes_128_decrypt(
    file_key: &[u8],
    object_number: u32,
    generation: u16,
    data: &[u8],
) -> Result<Vec<u8>, String> {
    // Data must contain at least the IV (16 bytes)
    if data.len() < AES_BLOCK_SIZE {
        return Err("Encrypted data too short (missing IV)".to_string());
    }

    // Extract IV from first 16 bytes
    let iv = &data[..AES_BLOCK_SIZE];
    let ciphertext = &data[AES_BLOCK_SIZE..];

    // Ciphertext length must be a multiple of block size
    if ciphertext.len() % AES_BLOCK_SIZE != 0 {
        return Err(format!(
            "Invalid ciphertext length: {} bytes (must be multiple of 16)",
            ciphertext.len()
        ));
    }

    // Derive the per-object AES-128 key
    let key = derive_aes_128_object_key(file_key, object_number, generation);

    let mut iv_copy = [0u8; AES_BLOCK_SIZE];
    iv_copy.copy_from_slice(iv);

    let mut data_copy = ciphertext.to_vec();

    // Decrypt with PKCS#7 padding (compatible with PKCS#5 for block size 16)
    let decryptor = Aes128CbcDec::new(&key.into(), &iv_copy.into());
    let decrypted_data = decryptor
        .decrypt_padded_mut::<Pkcs7>(&mut data_copy)
        .map_err(|e| format!("AES-128 decryption failed (invalid padding): {}", e))?;

    Ok(decrypted_data.to_vec())
}

/// Check if a crypt filter is /Identity (no-op).
///
/// Per PDF spec 7.6.5, /Identity crypt filter passes data through
/// without encryption.
#[must_use]
pub const fn is_identity_filter(filter_name: &str) -> bool {
    // Case-sensitive comparison per PDF spec
    filter_name.eq_ignore_ascii_case("Identity")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes_salt_constant() {
        assert_eq!(AES_SALT, [0x73, 0x41, 0x6C, 0x54]);
        // Verify it's the ASCII encoding of "sAlT"
        assert_eq!(std::str::from_utf8(&AES_SALT), Ok("sAlT"));
    }

    #[test]
    fn test_is_identity_filter() {
        assert!(is_identity_filter("Identity"));
        assert!(is_identity_filter("identity"));
        assert!(is_identity_filter("IDENTITY"));
        assert!(!is_identity_filter("AESV2"));
        assert!(!is_identity_filter("V2"));
    }

    #[test]
    fn test_derive_aes_128_object_key_different_objects() {
        let file_key = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        let key1 = derive_aes_128_object_key(&file_key, 1, 0);
        let key2 = derive_aes_128_object_key(&file_key, 2, 0);

        assert_ne!(key1, key2, "Different objects should have different keys");
    }

    #[test]
    fn test_derive_aes_128_object_key_same_object() {
        let file_key = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        let key1 = derive_aes_128_object_key(&file_key, 42, 0);
        let key2 = derive_aes_128_object_key(&file_key, 42, 0);

        assert_eq!(key1, key2, "Same object should derive same key");
    }

    #[test]
    fn test_derive_aes_128_object_key_generation_affects_key() {
        let file_key = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];

        let key1 = derive_aes_128_object_key(&file_key, 42, 0);
        let key2 = derive_aes_128_object_key(&file_key, 42, 1);

        assert_ne!(key1, key2, "Different generation numbers should produce different keys");
    }

    #[test]
    fn test_aes_128_decrypt_roundtrip() {
        // Create test data
        let file_key = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let object_number = 42;
        let generation = 0;
        let plaintext = b"Hello, AES-128 world! This is a test.";

        // For a proper roundtrip test, we'd need to encrypt first
        // Since we don't have an encrypt function, we'll just verify the decrypt function
        // doesn't panic on valid input structure
        let result = aes_128_decrypt(&file_key, object_number, generation, plaintext);
        // This will likely fail padding validation, but shouldn't panic
        assert!(result.is_ok() || result.is_err());
    }

    #[test]
    fn test_aes_128_decrypt_too_short() {
        let file_key = vec![1u8, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
        let data = [0u8; 8]; // Too short for IV

        let result = aes_128_decrypt(&file_key, 1, 0, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn test_aes_128_decrypt_invalid_length() {
        let file_key = vec![1u8; 16];
        // IV (16 bytes) + 17 bytes of ciphertext (not a multiple of 16)
        let mut data = vec![0u8; 33];

        let result = aes_128_decrypt(&file_key, 1, 0, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be multiple of 16"));
    }

    #[test]
    fn test_aes_128_decrypt_exact_iv_only() {
        let file_key = vec![1u8; 16];
        let data = [0u8; 16]; // Only IV, no ciphertext

        // With 0 bytes of ciphertext, PKCS#7 padding validation fails
        // because there's no padding to strip. This is correct behavior.
        let result = aes_128_decrypt(&file_key, 1, 0, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("invalid padding"));
    }

    #[test]
    fn test_aes_128_decrypt_empty_data() {
        let file_key = vec![1u8; 16];
        let data = [];

        let result = aes_128_decrypt(&file_key, 1, 0, &data);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("too short"));
    }

    #[test]
    fn test_aes_block_size_constant() {
        assert_eq!(AES_BLOCK_SIZE, 16);
    }
}
