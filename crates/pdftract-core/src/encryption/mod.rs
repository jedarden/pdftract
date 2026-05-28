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

pub mod detection;

#[cfg(feature = "decrypt")]
pub mod aes_128;

#[cfg(feature = "decrypt")]
pub mod aes_256;

#[cfg(feature = "decrypt")]
pub mod rc4;

#[cfg(feature = "decrypt")]
pub use aes_128::{aes_128_decrypt, derive_aes_128_object_key, is_identity_filter};

#[cfg(feature = "decrypt")]
pub use aes_256::{aes_256_decrypt, Aes256Decryptor, FileKeyResult as Aes256FileKeyResult};

#[cfg(feature = "decrypt")]
pub use rc4::{
    decrypt_object, derive_file_key, derive_object_key, pad_password, rc4_decrypt,
    validate_user_password, validate_user_password_r2, validate_user_password_r3,
    FileKeyResult as Rc4FileKeyResult,
};

pub use detection::{
    detect_encryption, AuthEvent, CryptFilterDef, CryptFilterMethod, CryptFiltersV4,
    EncryptionInfo,
};

use crate::diagnostics::{DiagCode, Diagnostic};

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
