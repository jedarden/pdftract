//! AES-256 decryption for PDF V=5 R=6 (PDF 2.0).
//!
//! This module implements AES-256 decryption per PDF 2.0 spec (ISO 32000-2:2017),
//! section 7.6.4.3. It uses the complex Algorithm 8 for key derivation involving
//! SHA-256, SHA-384, and SHA-512 in a multi-round protocol.
//!
//! # Key Derivation (Algorithm 8)
//!
//! The file encryption key is derived through a 64-round iterative process:
//! 1. Compute initial hash H = SHA-256(password || salt_U || U || salt_O || O)
//! 2. For 64 rounds, select hash function based on H's last byte mod 3
//! 3. After 64 rounds, decrypt /UE (or /OE) with AES-256-CBC to get file key
//!
//! # Per-Object Encryption
//!
//! V=5 does NOT use per-object key derivation. The file key is used directly
//! for every object, with a 16-byte IV prepended to each encrypted stream.

use aes::cipher::{block_padding::Pkcs7, BlockDecryptMut, KeyIvInit};
use sha2::{Digest, Sha256, Sha384, Sha512};
use std::fmt;

type Aes256CbcDec = cbc::Decryptor<aes::Aes256>;

/// AES-256 block size in bytes (128 bits).
const AES_BLOCK_SIZE: usize = 16;

/// Salt size for V=5 encryption (8 bytes).
const SALT_SIZE: usize = 8;

/// User/Owner key size for V=5 (32 bytes for AES-256).
const KEY_SIZE: usize = 32;

/// Validation salt offset in /U or /O.
const VALIDATION_SALT_OFFSET: usize = 0;

/// Key salt offset in /U or /O.
const KEY_SALT_OFFSET: usize = 8;

/// Hash offset in /U or /O (after the two salts).
const HASH_OFFSET: usize = 16;

/// Number of key derivation rounds for R=6 (R=5 uses fewer).
const KEY_DERIVATION_ROUNDS: usize = 64;

/// Result of file key derivation.
#[derive(Debug, Clone)]
pub enum FileKeyResult {
    /// Successfully derived file key (32 bytes for AES-256)
    Success([u8; KEY_SIZE]),
    /// Wrong password (validation hash mismatch)
    WrongPassword,
    /// Invalid encryption data (malformed /U, /O, /UE, /OE)
    InvalidData(String),
}

impl FileKeyResult {
    /// Check if the result is successful.
    pub fn is_success(&self) -> bool {
        matches!(self, FileKeyResult::Success(_))
    }

    /// Get the file key if successful.
    pub fn key(&self) -> Option<[u8; KEY_SIZE]> {
        match self {
            FileKeyResult::Success(key) => Some(*key),
            _ => None,
        }
    }
}

/// AES-256 decryptor for PDF V=5 R=6.
///
/// This handles both user-password and owner-password authentication paths,
/// as well as the complex Algorithm 8 key derivation.
pub struct Aes256Decryptor {
    /// User password hash /U (48 bytes for V=5: 8-byte validation salt + 8-byte key salt + 32-byte hash)
    user_hash: Vec<u8>,
    /// Owner password hash /O (48 bytes)
    owner_hash: Vec<u8>,
    /// Encrypted user encryption key /UE (32 bytes)
    user_key_encrypted: Vec<u8>,
    /// Encrypted owner encryption key /OE (32 bytes)
    owner_key_encrypted: Vec<u8>,
    /// Encrypted permissions /Perms (16 bytes)
    perms_encrypted: Vec<u8>,
    /// Document ID (first element of /ID array, used in key derivation)
    document_id: Vec<u8>,
}

impl Aes256Decryptor {
    /// Create a new AES-256 decryptor from encryption metadata.
    ///
    /// # Arguments
    ///
    /// * `user_hash` - The /U value from the encryption dictionary (48 bytes)
    /// * `owner_hash` - The /O value from the encryption dictionary (48 bytes)
    /// * `user_key_encrypted` - The /UE value (32 bytes)
    /// * `owner_key_encrypted` - The /OE value (32 bytes)
    /// * `perms_encrypted` - The /Perms value (16 bytes)
    /// * `document_id` - The first element of the /ID array (used in key derivation)
    ///
    /// # Returns
    ///
    /// `Some(decryptor)` if all fields are valid, `None` otherwise.
    pub fn new(
        user_hash: Vec<u8>,
        owner_hash: Vec<u8>,
        user_key_encrypted: Vec<u8>,
        owner_key_encrypted: Vec<u8>,
        perms_encrypted: Vec<u8>,
        document_id: Vec<u8>,
    ) -> Option<Self> {
        // Validate lengths
        if user_hash.len() != 48 || owner_hash.len() != 48 {
            return None;
        }
        if user_key_encrypted.len() != 32 || owner_key_encrypted.len() != 32 {
            return None;
        }
        if perms_encrypted.len() != 16 {
            return None;
        }

        Some(Self {
            user_hash,
            owner_hash,
            user_key_encrypted,
            owner_key_encrypted,
            perms_encrypted,
            document_id,
        })
    }

    /// Derive the file encryption key using the user password.
    ///
    /// Implements Algorithm 11 (user password validation) from PDF 2.0 spec.
    ///
    /// # Arguments
    ///
    /// * `password` - The user password to try (empty string for no-password case)
    ///
    /// # Returns
    ///
    /// `FileKeyResult` indicating success or failure reason.
    pub fn derive_file_key_user(&self, password: &str) -> FileKeyResult {
        // Extract validation salt and key salt from /U
        let validation_salt =
            &self.user_hash[VALIDATION_SALT_OFFSET..VALIDATION_SALT_OFFSET + SALT_SIZE];
        let key_salt = &self.user_hash[KEY_SALT_OFFSET..KEY_SALT_OFFSET + SALT_SIZE];
        let stored_hash = &self.user_hash[HASH_OFFSET..];

        // Algorithm 11 step (a): compute hash for validation
        let validation_hash =
            self.compute_password_hash(password, validation_salt, &self.user_hash);

        // Compare with stored hash
        if validation_hash != stored_hash {
            return FileKeyResult::WrongPassword;
        }

        // Algorithm 11 step (b): compute hash for key derivation
        let key_hash = self.compute_password_hash(password, key_salt, &self.user_hash);

        // Decrypt /UE with this key to get the file encryption key
        let file_key = self.decrypt_ue_or_oe(&self.user_key_encrypted, &key_hash);

        FileKeyResult::Success(file_key)
    }

    /// Derive the file encryption key using the owner password.
    ///
    /// Implements Algorithm 12 (owner password validation) from PDF 2.0 spec.
    ///
    /// # Arguments
    ///
    /// * `password` - The owner password to try
    ///
    /// # Returns
    ///
    /// `FileKeyResult` indicating success or failure reason.
    pub fn derive_file_key_owner(&self, password: &str) -> FileKeyResult {
        // Extract validation salt and key salt from /O
        let validation_salt =
            &self.owner_hash[VALIDATION_SALT_OFFSET..VALIDATION_SALT_OFFSET + SALT_SIZE];
        let key_salt = &self.owner_hash[KEY_SALT_OFFSET..KEY_SALT_OFFSET + SALT_SIZE];
        let stored_hash = &self.owner_hash[HASH_OFFSET..];

        // Algorithm 12 step (a): compute hash for validation (includes /U)
        let validation_hash = self.compute_owner_password_hash(
            password,
            validation_salt,
            &self.owner_hash,
            &self.user_hash,
        );

        // Compare with stored hash
        if validation_hash != stored_hash {
            return FileKeyResult::WrongPassword;
        }

        // Algorithm 12 step (b): compute hash for key derivation
        let key_hash =
            self.compute_owner_password_hash(password, key_salt, &self.owner_hash, &self.user_hash);

        // Decrypt /OE with this key to get the file encryption key
        let file_key = self.decrypt_ue_or_oe(&self.owner_key_encrypted, &key_hash);

        FileKeyResult::Success(file_key)
    }

    /// Decrypt /UE or /OE to recover the file encryption key.
    ///
    /// Uses AES-256-CBC with all-zero IV and no padding.
    /// The input is exactly 32 bytes (one AES block).
    fn decrypt_ue_or_oe(&self, encrypted: &[u8], key: &[u8]) -> [u8; KEY_SIZE] {
        assert_eq!(encrypted.len(), KEY_SIZE, "/UE and /OE must be 32 bytes");
        assert_eq!(key.len(), KEY_SIZE, "Key must be 32 bytes");

        // All-zero IV for /UE and /OE decryption
        let iv = [0u8; AES_BLOCK_SIZE];

        let mut key_copy = [0u8; KEY_SIZE];
        key_copy.copy_from_slice(key);

        let mut encrypted_copy = [0u8; KEY_SIZE];
        encrypted_copy.copy_from_slice(encrypted);

        // Per PDF 2.0 Algorithm 8/2.A, the file key is recovered with raw
        // AES-256-CBC and NO padding: the 32-byte /UE or /OE block decrypts
        // directly to the 32-byte file encryption key. A raw key is not
        // PKCS7-padded, so we must not attempt to strip padding here (doing so
        // would fail for essentially every real key). Decrypting block-by-block
        // is infallible for a block-aligned input, so this cannot panic.
        let mut decryptor = Aes256CbcDec::new(&key_copy.into(), &iv.into());
        for block in encrypted_copy.chunks_exact_mut(AES_BLOCK_SIZE) {
            let block = aes::cipher::generic_array::GenericArray::from_mut_slice(block);
            decryptor.decrypt_block_mut(block);
        }

        // Return the full 32-byte decrypted file key.
        let mut result = [0u8; KEY_SIZE];
        result.copy_from_slice(&encrypted_copy[..KEY_SIZE]);
        result
    }

    /// Compute the password hash for key derivation (Algorithm 8).
    ///
    /// This is the core of the PDF 2.0 key derivation - it runs 64 rounds of
    /// hashing, selecting between SHA-256, SHA-384, and SHA-512 based on
    /// the last byte of the previous hash.
    fn compute_password_hash(&self, password: &str, salt: &[u8], u_value: &[u8]) -> Vec<u8> {
        // Step 1: Initial hash H = SHA-256(password || salt || u_value)
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(salt);
        hasher.update(u_value);
        let mut h: Vec<u8> = hasher.finalize().to_vec();

        // Step 2: For 64 rounds, select hash based on last byte of H
        // E = password || salt || u_value
        let mut e = Vec::new();
        e.extend_from_slice(password.as_bytes());
        e.extend_from_slice(salt);
        e.extend_from_slice(u_value);

        for _ in 0..KEY_DERIVATION_ROUNDS {
            // Step 2a: Select hash function based on last byte of E mod 3
            // (Note: spec says "last byte of E", but E grows each round.
            // We use the last byte of the current E, which is h from previous round)
            let hash_byte = e.last().copied().unwrap_or(0);
            let hash_function = hash_byte % 3;

            // Step 2b: Compute hash with selected function
            let round_hash = match hash_function {
                0 => {
                    let mut hasher = Sha256::new();
                    hasher.update(&e);
                    hasher.finalize().to_vec()
                }
                1 => {
                    let mut hasher = Sha384::new();
                    hasher.update(&e);
                    hasher.finalize().to_vec()
                }
                2 => {
                    let mut hasher = Sha512::new();
                    hasher.update(&e);
                    hasher.finalize().to_vec()
                }
                _ => unreachable!(),
            };

            // Step 2c: E = E || round_hash
            e.extend_from_slice(&round_hash);

            // Update h for next round
            h = round_hash;
        }

        // Step 3: Return first 32 bytes of the final hash
        h[..KEY_SIZE].to_vec()
    }

    /// Compute the owner password hash (Algorithm 12 variant).
    ///
    /// This is similar to compute_password_hash but includes both /U and /O values.
    fn compute_owner_password_hash(
        &self,
        password: &str,
        salt: &[u8],
        o_value: &[u8],
        u_value: &[u8],
    ) -> Vec<u8> {
        // Step 1: Initial hash H = SHA-256(password || salt || o_value || u_value)
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.update(salt);
        hasher.update(o_value);
        hasher.update(u_value);
        let mut h: Vec<u8> = hasher.finalize().to_vec();

        // Step 2: For 64 rounds, select hash based on last byte
        let mut e = Vec::new();
        e.extend_from_slice(password.as_bytes());
        e.extend_from_slice(salt);
        e.extend_from_slice(o_value);
        e.extend_from_slice(u_value);

        for _ in 0..KEY_DERIVATION_ROUNDS {
            let hash_byte = e.last().copied().unwrap_or(0);
            let hash_function = hash_byte % 3;

            let round_hash = match hash_function {
                0 => {
                    let mut hasher = Sha256::new();
                    hasher.update(&e);
                    hasher.finalize().to_vec()
                }
                1 => {
                    let mut hasher = Sha384::new();
                    hasher.update(&e);
                    hasher.finalize().to_vec()
                }
                2 => {
                    let mut hasher = Sha512::new();
                    hasher.update(&e);
                    hasher.finalize().to_vec()
                }
                _ => unreachable!(),
            };

            e.extend_from_slice(&round_hash);
            h = round_hash;
        }

        h[..KEY_SIZE].to_vec()
    }

    /// Decrypt a data stream using the file encryption key.
    ///
    /// For V=5, each stream has a 16-byte IV prepended to the ciphertext.
    /// This function strips the IV and decrypts the data using AES-256-CBC.
    ///
    /// # Arguments
    ///
    /// * `file_key` - The 32-byte file encryption key
    /// * `encrypted_data` - The encrypted data with IV prefix
    ///
    /// # Returns
    ///
    /// The decrypted plaintext, or an error message if decryption fails.
    pub fn decrypt_stream(
        &self,
        file_key: &[u8; 32],
        encrypted_data: &[u8],
    ) -> Result<Vec<u8>, String> {
        if encrypted_data.len() < AES_BLOCK_SIZE {
            return Err("Encrypted data too short (missing IV)".to_string());
        }

        // Extract IV from first 16 bytes
        let iv = &encrypted_data[..AES_BLOCK_SIZE];
        let ciphertext = &encrypted_data[AES_BLOCK_SIZE..];

        let mut key_copy = [0u8; KEY_SIZE];
        key_copy.copy_from_slice(file_key);

        let mut iv_copy = [0u8; AES_BLOCK_SIZE];
        iv_copy.copy_from_slice(iv);

        let mut data_copy = ciphertext.to_vec();

        // Decrypt with PKCS#7 padding
        let decryptor = Aes256CbcDec::new(&key_copy.into(), &iv_copy.into());
        let decrypted_data = decryptor
            .decrypt_padded_mut::<Pkcs7>(&mut data_copy)
            .map_err(|e| format!("AES-256 decryption failed: {}", e))?;

        // Return decrypted data (without padding)
        Ok(decrypted_data.to_vec())
    }

    /// Decrypt the /Perms field to recover permission bits.
    ///
    /// V=5 stores permissions in a 16-byte AES-256-ECB encrypted field.
    pub fn decrypt_perms(&self, file_key: &[u8; 32]) -> Result<[u8; 16], String> {
        use aes::cipher::{BlockDecrypt, KeyInit};

        type Aes256 = aes::Aes256;

        let mut key_copy = [0u8; KEY_SIZE];
        key_copy.copy_from_slice(file_key);

        let mut perms_copy = [0u8; 16];
        perms_copy.copy_from_slice(&self.perms_encrypted);

        // Decrypt with ECB (no IV) - one block for /Perms
        let cipher = Aes256::new(&key_copy.into());
        cipher.decrypt_block((&mut perms_copy).into());

        Ok(perms_copy)
    }
}

impl fmt::Debug for Aes256Decryptor {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Aes256Decryptor")
            .field("user_hash", &"<redacted>")
            .field("owner_hash", &"<redacted>")
            .field("user_key_encrypted", &"<redacted>")
            .field("owner_key_encrypted", &"<redacted>")
            .field("perms_encrypted", &"<redacted>")
            .field("document_id", &self.document_id)
            .finish()
    }
}

/// Convenience function to decrypt AES-256 encrypted data.
///
/// # Arguments
///
/// * `file_key` - The 32-byte file encryption key
/// * `encrypted_data` - The encrypted data with IV prefix
///
/// # Returns
///
/// The decrypted plaintext, or an error if decryption fails.
pub fn aes_256_decrypt(file_key: &[u8; 32], encrypted_data: &[u8]) -> Result<Vec<u8>, String> {
    // Create a dummy decryptor (we only need the decrypt_stream method)
    let dummy_decryptor = Aes256Decryptor::new(
        vec![0u8; 48],
        vec![0u8; 48],
        vec![0u8; 32],
        vec![0u8; 32],
        vec![0u8; 16],
        vec![],
    )
    .unwrap();

    dummy_decryptor.decrypt_stream(file_key, encrypted_data)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_aes256_decryptor_new_valid() {
        let user_hash = vec![0u8; 48];
        let owner_hash = vec![0u8; 48];
        let user_key_encrypted = vec![0u8; 32];
        let owner_key_encrypted = vec![0u8; 32];
        let perms_encrypted = vec![0u8; 16];
        let document_id = vec![];

        let decryptor = Aes256Decryptor::new(
            user_hash,
            owner_hash,
            user_key_encrypted,
            owner_key_encrypted,
            perms_encrypted,
            document_id,
        );

        assert!(decryptor.is_some());
    }

    #[test]
    fn test_aes256_decryptor_new_invalid_user_hash_length() {
        let user_hash = vec![0u8; 32]; // Wrong length
        let owner_hash = vec![0u8; 48];
        let user_key_encrypted = vec![0u8; 32];
        let owner_key_encrypted = vec![0u8; 32];
        let perms_encrypted = vec![0u8; 16];
        let document_id = vec![];

        let decryptor = Aes256Decryptor::new(
            user_hash,
            owner_hash,
            user_key_encrypted,
            owner_key_encrypted,
            perms_encrypted,
            document_id,
        );

        assert!(decryptor.is_none());
    }

    #[test]
    fn test_file_key_result_is_success() {
        let key = [0u8; 32];
        let result = FileKeyResult::Success(key);
        assert!(result.is_success());
        assert_eq!(result.key(), Some(key));
    }

    #[test]
    fn test_file_key_result_wrong_password() {
        let result = FileKeyResult::WrongPassword;
        assert!(!result.is_success());
        assert_eq!(result.key(), None);
    }

    #[test]
    fn test_compute_password_hash_basic() {
        let decryptor = Aes256Decryptor::new(
            vec![0u8; 48],
            vec![0u8; 48],
            vec![0u8; 32],
            vec![0u8; 32],
            vec![0u8; 16],
            vec![],
        )
        .unwrap();

        let salt = [0u8; 8];
        let u_value = [0u8; 48];
        let password = "test";

        let hash = decryptor.compute_password_hash(password, &salt, &u_value);

        // Should produce a 32-byte hash
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_decrypt_stream_too_short() {
        let decryptor = Aes256Decryptor::new(
            vec![0u8; 48],
            vec![0u8; 48],
            vec![0u8; 32],
            vec![0u8; 32],
            vec![0u8; 16],
            vec![],
        )
        .unwrap();

        let file_key = [0u8; 32];
        let encrypted_data = [0u8; 8]; // Too short

        let result = decryptor.decrypt_stream(&file_key, &encrypted_data);
        assert!(result.is_err());
    }

    #[test]
    fn test_aes_256_decrypt_basic() {
        // This is a basic sanity check - we'll need real test vectors for full validation
        let file_key = [0u8; 32];
        let encrypted_data = vec![0u8; 32]; // 16-byte IV + 16-byte data

        let result = aes_256_decrypt(&file_key, &encrypted_data);
        // Should not panic, though result may be garbage
        assert!(result.is_ok() || result.is_err());
    }

    /// Regression test for the /UE and /OE file-key recovery panic.
    ///
    /// Per PDF 2.0 Algorithm 8/2.A, the 32-byte /UE (or /OE) block holds the
    /// file encryption key encrypted with AES-256-CBC, all-zero IV, and NO
    /// padding. A raw file key is not PKCS7-padded, so the previous
    /// implementation (`decrypt_padded_mut::<Pkcs7>(...).expect(...)`) panicked
    /// for essentially every real key — here `file_key` ends in `0xAB`, which
    /// is not a valid PKCS7 pad length, so the old code hit `UnpadError` and
    /// `.expect()` panicked. This test builds the /UE block exactly as a
    /// conforming producer would (raw NoPadding encrypt), recovers the key via
    /// `decrypt_ue_or_oe`, and asserts it round-trips to all 32 bytes without
    /// panicking. It then confirms the recovered key decrypts a stream to the
    /// expected plaintext.
    #[test]
    fn test_decrypt_ue_or_oe_no_padding_roundtrip_no_panic() {
        use aes::cipher::{block_padding::NoPadding, BlockEncryptMut};

        type Aes256CbcEnc = cbc::Encryptor<aes::Aes256>;

        // The intermediate hash used to encrypt /UE (Algorithm 8 step (b)).
        let key_hash = [0x11u8; KEY_SIZE];
        // A raw 32-byte file key whose final byte (0xAB) is NOT a valid PKCS7
        // pad length — this is what made the old Pkcs7 path panic.
        let file_key = [0xABu8; KEY_SIZE];

        // Build the /UE block: AES-256-CBC, all-zero IV, no padding.
        let zero_iv = [0u8; AES_BLOCK_SIZE];
        let mut ue = file_key;
        Aes256CbcEnc::new(&key_hash.into(), &zero_iv.into())
            .encrypt_padded_mut::<NoPadding>(&mut ue, KEY_SIZE)
            .expect("NoPadding encrypt of a 32-byte block is infallible");

        let decryptor = Aes256Decryptor::new(
            vec![0u8; 48],
            vec![0u8; 48],
            ue.to_vec(),
            vec![0u8; 32],
            vec![0u8; 16],
            vec![],
        )
        .unwrap();

        // Must NOT panic and must return the full 32-byte raw key.
        let recovered = decryptor.decrypt_ue_or_oe(&ue, &key_hash);
        assert_eq!(
            recovered, file_key,
            "recovered file key must equal the original raw 32-byte key"
        );

        // The recovered key must decrypt a stream to the expected plaintext.
        let plaintext = b"pdftract V5/R6 file-key recovery works";
        let iv = [0x22u8; AES_BLOCK_SIZE];
        let mut buf = vec![0u8; plaintext.len() + AES_BLOCK_SIZE];
        buf[..plaintext.len()].copy_from_slice(plaintext);
        let ciphertext = Aes256CbcEnc::new(&file_key.into(), &iv.into())
            .encrypt_padded_mut::<Pkcs7>(&mut buf, plaintext.len())
            .expect("stream encryption should succeed");

        let mut encrypted_data = iv.to_vec();
        encrypted_data.extend_from_slice(ciphertext);

        let decrypted = decryptor
            .decrypt_stream(&recovered, &encrypted_data)
            .expect("stream decryption with recovered key should succeed");
        assert_eq!(decrypted, plaintext);
    }
}
