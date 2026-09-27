//! Unified PDF decryption module.
//!
//! This module provides a high-level API for PDF decryption that:
//! - Detects encryption from the trailer's /Encrypt dictionary
//! - Attempts password validation (empty string first, then user-provided)
//! - Provides per-object and per-stream decryption functions

#[cfg(feature = "decrypt")]
use crate::diagnostics::{DiagCode, Diagnostic};
#[cfg(feature = "decrypt")]
use crate::encryption::{
    aes_128::aes_128_decrypt,
    aes_256::{aes_256_decrypt, Aes256Decryptor, FileKeyResult as Aes256FileKeyResult},
    detection::{detect_encryption, CryptFilterMethod, EncryptionInfo},
    rc4::{
        decrypt_object, derive_file_key, pad_password, rc4_decrypt, validate_user_password,
        FileKeyResult as Rc4FileKeyResult,
    },
};
#[cfg(feature = "decrypt")]
use crate::parser::xref::XrefResolver;
#[cfg(feature = "decrypt")]
use digest::Digest;

/// Error during PDF decryption.
#[cfg(feature = "decrypt")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DecryptionError {
    /// Unsupported encryption algorithm (e.g., Adobe Public Key)
    UnsupportedAlgorithm,
    /// Wrong password (validation failed)
    WrongPassword,
    /// Missing required field in encryption dictionary
    MissingField(String),
    /// Invalid data format
    InvalidFormat,
    /// Decryption failed (corrupted data)
    DecryptionFailed,
}

#[cfg(feature = "decrypt")]
impl DecryptionError {
    /// Convert to diagnostic code.
    pub fn to_diag_code(&self) -> DiagCode {
        match self {
            DecryptionError::UnsupportedAlgorithm => DiagCode::EncryptionUnsupported,
            DecryptionError::WrongPassword => DiagCode::EncryptionWrongPassword,
            DecryptionError::MissingField(_) => DiagCode::StructMissingKey,
            DecryptionError::InvalidFormat | DecryptionError::DecryptionFailed => {
                DiagCode::EncryptionWrongPassword
            }
        }
    }

    /// Convert to diagnostic.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            DecryptionError::UnsupportedAlgorithm => Diagnostic::with_static_no_offset(
                DiagCode::EncryptionUnsupported,
                "Unsupported encryption algorithm",
            ),
            DecryptionError::WrongPassword => Diagnostic::with_static_no_offset(
                DiagCode::EncryptionWrongPassword,
                "Wrong password",
            ),
            DecryptionError::MissingField(field) => Diagnostic::with_dynamic_no_offset(
                DiagCode::StructMissingKey,
                format!("Missing encryption field: {}", field),
            ),
            DecryptionError::InvalidFormat => Diagnostic::with_static_no_offset(
                DiagCode::EncryptionWrongPassword,
                "Invalid encrypted data format",
            ),
            DecryptionError::DecryptionFailed => Diagnostic::with_static_no_offset(
                DiagCode::EncryptionWrongPassword,
                "Decryption failed",
            ),
        }
    }
}

/// Result of password validation.
#[cfg(feature = "decrypt")]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PasswordValidation {
    /// Empty password (owner password not set)
    EmptyPassword,
    /// User password matched
    UserPassword,
    /// Owner password matched
    OwnerPassword,
}

/// Decryption context for an encrypted PDF.
///
/// This struct contains the file encryption key and metadata needed
/// to decrypt streams and strings in the PDF.
#[cfg(feature = "decrypt")]
#[derive(Clone)]
pub struct DecryptionContext {
    /// Encryption metadata from the /Encrypt dictionary
    pub info: EncryptionInfo,
    /// File encryption key (derived from password)
    file_key: Vec<u8>,
    /// Which password validation succeeded
    password_source: PasswordValidation,
    /// Crypt filter to use for streams (from /StmF)
    stream_filter: String,
    /// Crypt filter to use for strings (from /StrF)
    string_filter: String,
    /// Crypt filter to use for embedded files (from /EFF)
    embedded_file_filter: String,
}

#[cfg(feature = "decrypt")]
impl DecryptionContext {
    /// Create a new decryption context from encryption info and file key.
    pub fn new(
        info: EncryptionInfo,
        file_key: Vec<u8>,
        password_source: PasswordValidation,
    ) -> Result<Self, DecryptionError> {
        // Get default stream and string filters
        let (stream_filter, string_filter, embedded_file_filter) =
            if let Some(ref cf) = info.crypt_filters {
                (
                    cf.stream_filter.clone(),
                    cf.string_filter.clone(),
                    cf.embedded_file_filter.clone(),
                )
            } else {
                // Pre-V=4: use RC4 for everything
                ("V2".to_string(), "V2".to_string(), "V2".to_string())
            };

        Ok(Self {
            info,
            file_key,
            password_source,
            stream_filter,
            string_filter,
            embedded_file_filter,
        })
    }

    /// Decrypt a stream using the per-object key.
    ///
    /// # Arguments
    ///
    /// * `encrypted_data` - The encrypted stream data (with IV prepended for AES)
    /// * `object_number` - The PDF object number
    /// * `generation` - The PDF object generation number
    ///
    /// # Returns
    ///
    /// The decrypted data, or an error if decryption fails.
    pub fn decrypt_stream(
        &self,
        encrypted_data: &[u8],
        object_number: u32,
        generation: u16,
    ) -> Result<Vec<u8>, DecryptionError> {
        self.decrypt_with_filter(
            encrypted_data,
            object_number,
            generation,
            &self.stream_filter,
        )
    }

    /// Decrypt a stream using an explicit crypt filter name from /Crypt /Name.
    pub fn decrypt_stream_with_filter(
        &self,
        encrypted_data: &[u8],
        object_number: u32,
        generation: u16,
        filter_name: Option<&str>,
    ) -> Result<Vec<u8>, DecryptionError> {
        let filter_name = filter_name.unwrap_or(&self.stream_filter);
        self.decrypt_with_filter(encrypted_data, object_number, generation, filter_name)
    }

    /// Decrypt an embedded-file stream using the encryption dictionary's /EFF filter.
    pub fn decrypt_embedded_file(
        &self,
        encrypted_data: &[u8],
        object_number: u32,
        generation: u16,
    ) -> Result<Vec<u8>, DecryptionError> {
        self.decrypt_with_filter(
            encrypted_data,
            object_number,
            generation,
            &self.embedded_file_filter,
        )
    }

    fn decrypt_with_filter(
        &self,
        encrypted_data: &[u8],
        object_number: u32,
        generation: u16,
        filter_name: &str,
    ) -> Result<Vec<u8>, DecryptionError> {
        // Get the crypt filter definition
        let cfm = if filter_name == "Identity" {
            CryptFilterMethod::Identity
        } else if let Some(ref cf) = self.info.crypt_filters {
            cf.filters
                .get(filter_name)
                .map(|def| def.cfm)
                .ok_or(DecryptionError::UnsupportedAlgorithm)?
        } else {
            // Pre-V=4: use RC4 (V2)
            match self.info.version {
                1 | 2 => CryptFilterMethod::V2,
                _ => CryptFilterMethod::Identity,
            }
        };

        // Decrypt based on filter method
        match cfm {
            CryptFilterMethod::Identity => Ok(encrypted_data.to_vec()),
            CryptFilterMethod::V2 => {
                // RC4 decryption
                let decrypted =
                    decrypt_object(&self.file_key, object_number, generation, encrypted_data);
                Ok(decrypted)
            }
            CryptFilterMethod::AesV2 => {
                // AES-128 decryption
                aes_128_decrypt(&self.file_key, object_number, generation, encrypted_data)
                    .map_err(|_| DecryptionError::DecryptionFailed)
            }
            CryptFilterMethod::AesV3 => {
                // AES-256 decryption (V=5)
                // For V=5, the file_key is used directly (no per-object key derivation)
                let key_array: [u8; 32] = self
                    .file_key
                    .as_slice()
                    .try_into()
                    .map_err(|_| DecryptionError::InvalidFormat)?;
                aes_256_decrypt(&key_array, encrypted_data)
                    .map_err(|_| DecryptionError::DecryptionFailed)
            }
        }
    }

    /// Decrypt a string using the file key.
    ///
    /// For strings, we use the string_filter instead of stream_filter.
    ///
    /// # Arguments
    ///
    /// * `encrypted_data` - The encrypted string data
    /// * `object_number` - The PDF object number
    /// * `generation` - The PDF object generation number
    ///
    /// # Returns
    ///
    /// The decrypted data, or an error if decryption fails.
    pub fn decrypt_string(
        &self,
        encrypted_data: &[u8],
        object_number: u32,
        generation: u16,
    ) -> Result<Vec<u8>, DecryptionError> {
        self.decrypt_string_with_filter(encrypted_data, object_number, generation, None)
    }

    /// Decrypt a string using an explicit crypt filter name.
    pub fn decrypt_string_with_filter(
        &self,
        encrypted_data: &[u8],
        object_number: u32,
        generation: u16,
        filter_name: Option<&str>,
    ) -> Result<Vec<u8>, DecryptionError> {
        let filter_name = filter_name.unwrap_or(&self.string_filter);
        self.decrypt_with_filter(encrypted_data, object_number, generation, filter_name)
    }

    /// Get the encryption version (V).
    pub fn version(&self) -> u8 {
        self.info.version
    }

    /// Get the encryption revision (R).
    pub fn revision(&self) -> u8 {
        self.info.revision
    }

    /// Get the key length in bits.
    pub fn key_length(&self) -> u32 {
        self.info.key_length
    }

    /// Check if which password was used.
    pub fn password_source(&self) -> PasswordValidation {
        self.password_source
    }
}

/// Detect and decrypt an encrypted PDF.
///
/// This function:
/// 1. Detects encryption from the trailer's /Encrypt dictionary
/// 2. Attempts empty password first
/// 3. Attempts user-provided password if provided
/// 4. Returns a DecryptionContext if successful
///
/// # Arguments
///
/// * `trailer` - The trailer dictionary
/// * `resolver` - The xref resolver
/// * `password` - Optional user-provided password
/// * `diagnostics` - Diagnostics buffer
///
/// # Returns
///
/// - `Ok(Some(ctx))` - Successfully decrypted
/// - `Ok(None)` - Not encrypted
/// - `Err(e)` - Decryption failed (wrong password or unsupported)
#[cfg(feature = "decrypt")]
pub fn decrypt_with_password(
    trailer: &crate::parser::object::PdfDict,
    resolver: &XrefResolver,
    password: Option<&str>,
    diagnostics: &mut Vec<Diagnostic>,
) -> Result<Option<DecryptionContext>, DecryptionError> {
    // Step 1: Detect encryption
    let has_encrypt = trailer.contains_key("/Encrypt") || trailer.contains_key("Encrypt");
    let info = match detect_encryption(trailer, resolver, diagnostics) {
        Some(info) => info,
        None if has_encrypt => return Err(DecryptionError::UnsupportedAlgorithm),
        None => return Ok(None), // Not encrypted
    };

    // Step 2: Validate /ID is present
    if info.file_id.is_empty() || info.file_id.len() < 16 {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::EncryptionUnsupported,
            "Cannot decrypt: /ID array missing or too short (required for key derivation)"
                .to_string(),
        ));
        return Err(DecryptionError::MissingField("/ID".to_string()));
    }

    // Step 3: Attempt password validation based on version
    let result = match info.version {
        5 => decrypt_v5(&info, password, diagnostics),
        _ => decrypt_v1_v4(&info, password, diagnostics),
    };

    match result {
        Ok((file_key, source)) => Ok(Some(DecryptionContext::new(info, file_key, source)?)),
        Err(e) => {
            // Emit diagnostic and return error
            let diag = e.to_diagnostic();
            diagnostics.push(diag);
            Err(e)
        }
    }
}

/// Decrypt V=5 (AES-256) encrypted PDF.
#[cfg(feature = "decrypt")]
fn decrypt_v5(
    info: &EncryptionInfo,
    password: Option<&str>,
    _diagnostics: &mut Vec<Diagnostic>,
) -> Result<(Vec<u8>, PasswordValidation), DecryptionError> {
    // Extract required fields for V=5 decryption
    let user_hash = &info.user_hash;
    let owner_hash = &info.owner_hash;
    let user_key_encrypted = info
        .user_key_encrypted
        .as_ref()
        .ok_or_else(|| DecryptionError::MissingField("/UE".to_string()))?;
    let owner_key_encrypted = info
        .owner_key_encrypted
        .as_ref()
        .ok_or_else(|| DecryptionError::MissingField("/OE".to_string()))?;
    let perms_encrypted = info
        .perms_encrypted
        .as_ref()
        .ok_or_else(|| DecryptionError::MissingField("/Perms".to_string()))?
        .clone();

    // Create AES-256 decryptor
    let decryptor = Aes256Decryptor::new_with_revision(
        user_hash.clone(),
        owner_hash.clone(),
        user_key_encrypted.clone(),
        owner_key_encrypted.clone(),
        perms_encrypted,
        info.file_id.clone(),
        info.revision,
    )
    .ok_or_else(|| DecryptionError::InvalidFormat)?;

    // Attempt 1: Empty password (for documents with empty owner password)
    let result = decryptor.derive_file_key_user("");
    if let Aes256FileKeyResult::Success(key) = result {
        return Ok((key.to_vec(), PasswordValidation::EmptyPassword));
    }

    // Attempt 2: User password
    if let Some(pwd) = password {
        let result = decryptor.derive_file_key_user(pwd);
        if let Aes256FileKeyResult::Success(key) = result {
            return Ok((key.to_vec(), PasswordValidation::UserPassword));
        }

        // Attempt 3: Owner password
        let result = decryptor.derive_file_key_owner(pwd);
        if let Aes256FileKeyResult::Success(key) = result {
            return Ok((key.to_vec(), PasswordValidation::OwnerPassword));
        }
    }

    Err(DecryptionError::WrongPassword)
}

/// Decrypt V=1, V=2, or V=4 encrypted PDF (RC4 or AES-128).
#[cfg(feature = "decrypt")]
fn decrypt_v1_v4(
    info: &EncryptionInfo,
    password: Option<&str>,
    _diagnostics: &mut Vec<Diagnostic>,
) -> Result<(Vec<u8>, PasswordValidation), DecryptionError> {
    // Attempt 1: Empty password
    let result = derive_file_key(
        b"".as_slice(),
        &info.owner_hash,
        info.perms,
        &info.file_id,
        info.key_length,
        info.revision as u32,
    );

    if let Rc4FileKeyResult::Success(ref key) = result {
        // Validate with /U hash
        if validate_user_password(
            b"",
            key,
            &info.user_hash,
            &info.file_id,
            info.revision as u32,
        ) {
            return Ok((key.clone(), PasswordValidation::EmptyPassword));
        }
    }

    // Attempt 2: User password
    if let Some(pwd) = password {
        let pwd_bytes = pwd.as_bytes();
        let result = derive_file_key(
            pwd_bytes,
            &info.owner_hash,
            info.perms,
            &info.file_id,
            info.key_length,
            info.revision as u32,
        );

        if let Rc4FileKeyResult::Success(ref key) = result {
            // Validate with /U hash
            if validate_user_password(
                pwd_bytes,
                key,
                &info.user_hash,
                &info.file_id,
                info.revision as u32,
            ) {
                return Ok((key.clone(), PasswordValidation::UserPassword));
            }
        }

        // Attempt 3: Owner password. The /O entry encrypts the padded user
        // password with an owner-derived RC4 key; it is not itself the file
        // key input for Algorithm 2.
        if let Some(user_password) = recover_user_password_from_owner(info, pwd_bytes) {
            let result = derive_file_key(
                &user_password,
                &info.owner_hash,
                info.perms,
                &info.file_id,
                info.key_length,
                info.revision as u32,
            );
            if let Rc4FileKeyResult::Success(key) = result {
                if validate_user_password(
                    &user_password,
                    &key,
                    &info.user_hash,
                    &info.file_id,
                    info.revision as u32,
                ) {
                    return Ok((key, PasswordValidation::OwnerPassword));
                }
            }
        }
    }

    Err(DecryptionError::WrongPassword)
}

#[cfg(feature = "decrypt")]
fn recover_user_password_from_owner(
    info: &EncryptionInfo,
    owner_password: &[u8],
) -> Option<Vec<u8>> {
    let key_len = usize::try_from(info.key_length / 8).ok()?;
    if !(5..=16).contains(&key_len) || info.owner_hash.len() < 32 {
        return None;
    }

    let padded_owner = pad_password(owner_password);
    let mut digest = md5::Md5::digest(padded_owner);
    if info.revision >= 3 {
        for _ in 0..50 {
            digest = md5::Md5::digest(&digest[..key_len]);
        }
    }
    let owner_key = digest[..key_len].to_vec();
    let mut recovered = info.owner_hash[..32].to_vec();
    if info.revision == 2 {
        recovered = rc4_decrypt(&owner_key, &recovered);
    } else {
        for round in (0..20).rev() {
            let round_key: Vec<u8> = owner_key.iter().map(|byte| byte ^ round as u8).collect();
            recovered = rc4_decrypt(&round_key, &recovered);
        }
    }
    Some(recovered)
}

#[cfg(test)]
mod tests {
    #[cfg(feature = "decrypt")]
    use super::*;

    #[cfg(feature = "decrypt")]
    #[test]
    fn test_decryption_error_to_diag_code() {
        assert_eq!(
            DecryptionError::UnsupportedAlgorithm.to_diag_code(),
            DiagCode::EncryptionUnsupported
        );
        assert_eq!(
            DecryptionError::WrongPassword.to_diag_code(),
            DiagCode::EncryptionWrongPassword
        );
    }

    #[cfg(feature = "decrypt")]
    #[test]
    fn test_password_validation_equality() {
        assert_eq!(
            PasswordValidation::EmptyPassword,
            PasswordValidation::EmptyPassword
        );
        assert_ne!(
            PasswordValidation::UserPassword,
            PasswordValidation::OwnerPassword
        );
    }
}
