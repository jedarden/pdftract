//! PDF encryption support (RC4, AES-128, AES-256).
//!
//! This module implements PDF decryption per PDF 2.0 spec (ISO 32000-2:2017).
//! It supports:
//! - V=1, R=2: RC4 40-bit
//! - V=2, R=3: RC4 40-128 bit
//! - V=4, R=4: RC4 or AES-128 via crypt filters
//! - V=5, R=5/6: AES-256 with SHA-256/384/512 key derivation
//!
//! The `decrypt` feature must be enabled to use this module.

#[cfg(feature = "decrypt")]
pub mod aes_256;

#[cfg(feature = "decrypt")]
pub use aes_256::{aes_256_decrypt, Aes256Decryptor, FileKeyResult};

use crate::diagnostics::{DiagCode, Diagnostic};

/// Encryption algorithm version.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionVersion {
    /// V=1: RC4 40-bit
    V1,
    /// V=2: RC4 40-128 bit
    V2,
    /// V=4: RC4 or AES-128 via crypt filters
    V4,
    /// V=5: AES-256 (PDF 2.0)
    V5,
}

/// Encryption algorithm revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EncryptionRevision {
    /// R=2: RC4 40-bit
    R2,
    /// R=3: RC4 40-128 bit
    R3,
    /// R=4: Crypt filters
    R4,
    /// R=5: AES-256 (original PDF 2.0)
    R5,
    /// R=6: AES-256 (enhanced for Spectre mitigation)
    R6,
}

/// Encryption metadata extracted from the PDF's /Encrypt dictionary.
#[derive(Debug, Clone)]
pub struct EncryptionInfo {
    /// Algorithm version (V)
    pub version: EncryptionVersion,
    /// Algorithm revision (R)
    pub revision: EncryptionRevision,
    /// Key length in bits (40, 128, or 256)
    pub key_length: u32,
    /// Owner password hash (O)
    pub owner_hash: Vec<u8>,
    /// User password hash (U)
    pub user_hash: Vec<u8>,
    /// Permissions flags (P)
    pub permissions: u32,
    /// File encryption key (encrypted)
    pub file_key_encrypted: Option<Vec<u8>>,
    /// Crypt filter dictionary (CF) for V=4 and V=5
    pub crypt_filters: Option<Vec<u8>>,
}

/// Result of password validation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PasswordValidation {
    /// Empty password (owner password not set)
    EmptyPassword,
    /// User password matched
    UserPassword,
    /// Owner password matched
    OwnerPassword,
}

/// Error during decryption.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecryptError {
    /// Unsupported encryption algorithm
    UnsupportedAlgorithm,
    /// Wrong password
    WrongPassword,
    /// Missing required field in encryption dictionary
    MissingField(String),
    /// Invalid data format
    InvalidFormat,
    /// Decryption failed (corrupted data)
    DecryptionFailed,
}

impl DecryptError {
    /// Convert to diagnostic code.
    pub fn to_diag_code(&self) -> DiagCode {
        match self {
            DecryptError::UnsupportedAlgorithm => DiagCode::EncryptionUnsupported,
            DecryptError::WrongPassword => DiagCode::EncryptionWrongPassword,
            DecryptError::MissingField(_) => DiagCode::StructMissingKey,
            DecryptError::InvalidFormat => DiagCode::EncryptionWrongPassword,
            DecryptError::DecryptionFailed => DiagCode::EncryptionWrongPassword,
        }
    }

    /// Convert to diagnostic.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            DecryptError::UnsupportedAlgorithm => Diagnostic::with_static_no_offset(
                DiagCode::EncryptionUnsupported,
                "Unsupported encryption algorithm",
            ),
            DecryptError::WrongPassword => Diagnostic::with_static_no_offset(
                DiagCode::EncryptionWrongPassword,
                "Wrong password",
            ),
            DecryptError::MissingField(field) => Diagnostic::with_dynamic_no_offset(
                DiagCode::StructMissingKey,
                format!("Missing encryption field: {}", field),
            ),
            DecryptError::InvalidFormat => Diagnostic::with_static_no_offset(
                DiagCode::EncryptionWrongPassword,
                "Invalid encrypted data format",
            ),
            DecryptError::DecryptionFailed => Diagnostic::with_static_no_offset(
                DiagCode::EncryptionWrongPassword,
                "Decryption failed",
            ),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_decrypt_error_to_diag_code() {
        assert_eq!(
            DecryptError::UnsupportedAlgorithm.to_diag_code(),
            DiagCode::EncryptionUnsupported
        );
        assert_eq!(
            DecryptError::WrongPassword.to_diag_code(),
            DiagCode::EncryptionWrongPassword
        );
    }

    #[test]
    fn test_decrypt_error_to_diagnostic() {
        let diag = DecryptError::WrongPassword.to_diagnostic();
        assert_eq!(diag.code, DiagCode::EncryptionWrongPassword);
    }
}
