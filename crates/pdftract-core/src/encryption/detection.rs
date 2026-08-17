//! Encryption dictionary detection for PDF trailers.
//!
//! This module implements detection of PDF encryption metadata from the trailer's
//! /Encrypt dictionary. It parses the encryption version, revision, key length,
//! owner/user password hashes, permissions, and crypt filters.
//!
//! Per PDF 2.0 spec (ISO 32000-2:2017), sections 7.6.1-7.6.3.

use std::collections::BTreeMap;

use crate::parser::object::{ObjRef, PdfDict, PdfObject};
use crate::{
    diagnostics::Diagnostic,
    emit,
};

/// Encryption metadata extracted from the PDF's /Encrypt dictionary.
#[derive(Debug, Clone)]
pub struct EncryptionInfo {
    /// Algorithm version (V): 1, 2, 4, or 5
    pub version: u8,
    /// Algorithm revision (R): 2, 3, 4, 5, or 6
    pub revision: u8,
    /// Key length in bits: 40, 128, or 256
    pub key_length: u32,
    /// Owner password hash (/O)
    pub owner_hash: Vec<u8>,
    /// User password hash (/U)
    pub user_hash: Vec<u8>,
    /// Permissions flags (/P for V<5, /Perms for V=5)
    pub perms: u32,
    /// File ID (first 16 bytes of /ID\[0\] from trailer)
    pub file_id: Vec<u8>,
    /// Crypt filter dictionary for V=4 and V=5
    pub crypt_filters: Option<CryptFiltersV4>,
    /// Encrypted user encryption key (/UE) for V=5 (AES-256)
    pub user_key_encrypted: Option<Vec<u8>>,
    /// Encrypted owner encryption key (/OE) for V=5 (AES-256)
    pub owner_key_encrypted: Option<Vec<u8>>,
    /// Encrypted permissions (/Perms) for V=5 (AES-256)
    pub perms_encrypted: Option<Vec<u8>>,
}

/// Crypt filter metadata for V=4 and V=5 encryption.
///
/// Per PDF 2.0 spec 7.6.5, crypt filters allow different encryption methods
/// for streams and strings.
#[derive(Debug, Clone)]
pub struct CryptFiltersV4 {
    /// Default crypt filter for streams (/StmF)
    pub stream_filter: String,
    /// Default crypt filter for strings (/StrF)
    pub string_filter: String,
    /// Named crypt filter definitions (/CF)
    pub filters: BTreeMap<String, CryptFilterDef>,
}

/// Individual crypt filter definition.
///
/// Per PDF 2.0 spec 7.6.5, Table 23.
#[derive(Debug, Clone)]
pub struct CryptFilterDef {
    /// Crypt filter method (/CFM): V2 (RC4), AESV2, AESV3
    pub cfm: CryptFilterMethod,
    /// Key length in bits (/Length)
    pub length: Option<u32>,
    /// When this filter is applied (/AuthEvent)
    pub auth_event: AuthEvent,
}

/// Crypt filter method (CFM).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CryptFilterMethod {
    /// No encryption (identity)
    Identity,
    /// RC4 (V2)
    V2,
    /// AES-128 (AESV2)
    AesV2,
    /// AES-256 (AESV3)
    AesV3,
}

/// When a crypt filter is applied.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthEvent {
    /// Applied when opening the document (default)
    DocOpen,
    /// Applied when performing specific EFS operations
    EfoOpen,
}

/// Detect encryption metadata from the trailer's /Encrypt dictionary.
///
/// This function parses the trailer's /Encrypt dictionary and returns
/// structured encryption metadata. It validates the encryption filter
/// (must be /Standard) and checks that required fields are present.
///
/// # Arguments
///
/// * `trailer` - The trailer dictionary from the PDF
/// * `resolver` - The cross-reference resolver for dereferencing indirect objects
/// * `diagnostics` - Diagnostics vector to emit errors to
///
/// # Returns
///
/// * `Some(EncryptionInfo)` - If the PDF is encrypted and the /Encrypt dictionary is valid
/// * `None` - If the PDF is not encrypted, or if the encryption dictionary is invalid
///
/// # Diagnostics
///
/// This function emits diagnostics for:
/// * `ENCRYPTION_UNSUPPORTED` - Non-Standard encryption filter
/// * `ENCRYPTION_INVALID_DICT` - Missing required fields or invalid field values
pub fn detect_encryption(
    trailer: &PdfDict,
    resolver: &impl XrefResolver,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<EncryptionInfo> {
    // Step 1: Look up /Encrypt in trailer
    let encrypt_ref = trailer.get("/Encrypt")?;

    // Step 2: Resolve ObjRef via XrefResolver
    let encrypt_dict = match encrypt_ref {
        PdfObject::Ref(obj_ref) => resolver.resolve(*obj_ref).ok()?,
        PdfObject::Dict(dict) => PdfObject::Dict(dict.clone()),
        _ => return None,
    };

    let encrypt_dict = encrypt_dict.as_dict()?;

    // Step 3: Check /Filter == /Standard
    let filter = encrypt_dict.get("/Filter")?;
    let filter_name = filter.as_name()?;
    if filter_name != "Standard" {
        // Emit ENCRYPTION_UNSUPPORTED with the filter name
        emit!(
            diagnostics,
            EncryptionUnsupported,
            message = format!(
                "Unsupported encryption filter: /{}. Only /Standard is supported.",
                filter_name
            )
        );
        return None;
    }

    // Step 4: Parse /V, /R, /KeyLength
    let version = parse_version(encrypt_dict)?;
    let revision = parse_revision(encrypt_dict)?;
    let key_length = parse_key_length(encrypt_dict, version)?;

    // Step 5: Parse /O, /U with length validation
    let owner_hash = match parse_hash_with_diagnostics(encrypt_dict, "/O", revision, diagnostics) {
        Some(hash) => hash,
        None => return None,
    };
    let user_hash = match parse_hash_with_diagnostics(encrypt_dict, "/U", revision, diagnostics) {
        Some(hash) => hash,
        None => return None,
    };

    // Step 6: Parse /P (32-bit signed int; perms bitfield)
    let perms = parse_permissions(encrypt_dict)?;

    // Step 7: For V>=4, parse /CF, /StmF, /StrF
    let crypt_filters = if version >= 4 {
        Some(parse_crypt_filters(encrypt_dict)?)
    } else {
        None
    };

    // Step 8: For V=5, parse /Perms, /UE, /OE
    let (perms, user_key_encrypted, owner_key_encrypted, perms_encrypted) = if version == 5 {
        let perms = parse_v5_perms(encrypt_dict)?;
        let user_key_encrypted = parse_v5_key(encrypt_dict, "/UE")?;
        let owner_key_encrypted = parse_v5_key(encrypt_dict, "/OE")?;
        let perms_encrypted = parse_v5_perms_bytes(encrypt_dict)?;
        (
            perms,
            Some(user_key_encrypted),
            Some(owner_key_encrypted),
            Some(perms_encrypted),
        )
    } else {
        (perms, None, None, None)
    };

    // Step 9: Extract /ID[0] from trailer
    let file_id = extract_file_id(trailer);

    // Step 10: Return Some(EncryptionInfo)
    Some(EncryptionInfo {
        version,
        revision,
        key_length,
        owner_hash,
        user_hash,
        perms,
        file_id,
        crypt_filters,
        user_key_encrypted,
        owner_key_encrypted,
        perms_encrypted,
    })
}

/// Trait for xref resolution (to avoid coupling to specific resolver type).
///
/// This trait is implemented by the actual XrefResolver from the xref module,
/// and also by MockResolver for testing.
pub trait XrefResolver {
    /// Resolve an object reference to its underlying PDF object.
    ///
    /// # Arguments
    ///
    /// * `obj_ref` - The object reference to resolve
    ///
    /// # Returns
    ///
    /// * `Ok(PdfObject)` - The resolved object
    /// * `Err(ResolveError)` - If the object cannot be resolved
    fn resolve(&self, obj_ref: ObjRef) -> Result<PdfObject, ResolveError>;
}

/// Resolution error type.
#[derive(Debug, Clone)]
pub enum ResolveError {
    /// Object reference not found in the xref table
    NotFound(ObjRef),
    /// Circular reference detected during resolution
    CircularRef(ObjRef),
    /// I/O error during resolution (with error message)
    Io(String),
}

// Implement the detection module's XrefResolver trait for the actual xref::XrefResolver
impl XrefResolver for crate::parser::xref::XrefResolver {
    fn resolve(&self, obj_ref: ObjRef) -> Result<PdfObject, ResolveError> {
        // Convert ResolveError from xref module to detection module's ResolveError
        self.resolve(obj_ref).map_err(|e| match e {
            crate::parser::xref::ResolveError::NotFound(obj_ref) => ResolveError::NotFound(obj_ref),
            crate::parser::xref::ResolveError::CircularRef(obj_ref) => {
                ResolveError::CircularRef(obj_ref)
            }
            crate::parser::xref::ResolveError::Io(msg) => ResolveError::Io(msg),
        })
    }
}

/// Parse /V field from encryption dictionary.
fn parse_version(dict: &PdfDict) -> Option<u8> {
    dict.get("/V")?.as_int()?.try_into().ok()
}

/// Parse /R field from encryption dictionary.
fn parse_revision(dict: &PdfDict) -> Option<u8> {
    dict.get("/R")?.as_int()?.try_into().ok()
}

/// Parse /KeyLength field from encryption dictionary.
///
/// If not present, derive from V: V=1/2 -> 40, V=4 -> 128, V=5 -> 256
fn parse_key_length(dict: &PdfDict, version: u8) -> Option<u32> {
    if let Some(key_length) = dict.get("/Length") {
        let length = key_length.as_int()? as u32;
        // Validate key length is a multiple of 8
        if length % 8 != 0 {
            return None;
        }
        return Some(length);
    }

    // Default key lengths per version
    match version {
        1 | 2 => Some(40),
        4 => Some(128),
        5 => Some(256),
        _ => None,
    }
}

/// Parse a hash field (/O or /U) with length validation and diagnostic emission.
fn parse_hash_with_diagnostics(
    dict: &PdfDict,
    key: &str,
    revision: u8,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<u8>> {
    let hash_bytes = dict.get(key)?.as_string()?.to_vec();

    // Validate length
    let expected_len = if revision >= 5 { 48 } else { 32 };
    if hash_bytes.len() != expected_len {
        emit!(
            diagnostics,
            EncryptionInvalidDict,
            message = format!(
                "Invalid {} length: expected {} bytes, got {}",
                key,
                expected_len,
                hash_bytes.len()
            )
        );
        return None;
    }

    Some(hash_bytes)
}

/// Parse /P permissions field.
fn parse_permissions(dict: &PdfDict) -> Option<u32> {
    dict.get("/P")?.as_int()?.try_into().ok()
}

/// Parse /Perms field for V=5 encryption.
fn parse_v5_perms(dict: &PdfDict) -> Option<u32> {
    let perms_bytes = dict.get("/Perms")?.as_string()?;
    if perms_bytes.len() != 16 {
        return None;
    }
    // First 4 bytes are the permissions (little-endian)
    let mut bytes = [0u8; 4];
    bytes.copy_from_slice(&perms_bytes[..4]);
    Some(u32::from_le_bytes(bytes))
}

/// Parse /UE or /OE field for V=5 encryption (32-byte encrypted key).
fn parse_v5_key(dict: &PdfDict, key: &str) -> Option<Vec<u8>> {
    let key_bytes = dict.get(key)?.as_string()?.to_vec();
    if key_bytes.len() != 32 {
        return None;
    }
    Some(key_bytes)
}

/// Parse /Perms field as raw bytes for V=5 encryption (16-byte encrypted permissions).
fn parse_v5_perms_bytes(dict: &PdfDict) -> Option<Vec<u8>> {
    let perms_bytes = dict.get("/Perms")?.as_string()?.to_vec();
    if perms_bytes.len() != 16 {
        return None;
    }
    Some(perms_bytes)
}

/// Extract first 16 bytes of /ID[0] from trailer.
fn extract_file_id(trailer: &PdfDict) -> Vec<u8> {
    trailer
        .get("/ID")
        .and_then(|id| id.as_array())
        .and_then(|arr| arr.first())
        .and_then(|id| id.as_string())
        .map(|s| s.iter().copied().take(16).collect())
        .unwrap_or_default()
}

/// Parse crypt filter dictionary for V>=4 encryption.
fn parse_crypt_filters(dict: &PdfDict) -> Option<CryptFiltersV4> {
    let stream_filter = parse_filter_name(dict.get("/StmF"))?;
    let string_filter = parse_filter_name(dict.get("/StrF"))?;

    // /CF is optional - if not present, use empty filters map
    let filters = if let Some(cf_obj) = dict.get("/CF") {
        let cf_dict = cf_obj.as_dict()?;
        let mut filters = BTreeMap::new();

        for (name, filter_def) in cf_dict {
            let name_str = name.strip_prefix('/')?;
            let def = parse_crypt_filter_def(filter_def.as_dict()?)?;
            filters.insert(name_str.to_string(), def);
        }
        filters
    } else {
        BTreeMap::new()
    };

    Some(CryptFiltersV4 {
        stream_filter,
        string_filter,
        filters,
    })
}

/// Parse a filter name, defaulting to "Identity" if not present.
fn parse_filter_name(obj: Option<&PdfObject>) -> Option<String> {
    match obj {
        Some(PdfObject::Name(name)) => Some(name.strip_prefix('/').unwrap_or(name).to_string()),
        Some(_) => None,
        None => Some("Identity".to_string()),
    }
}

/// Parse a single crypt filter definition.
fn parse_crypt_filter_def(dict: &PdfDict) -> Option<CryptFilterDef> {
    let cfm = parse_cfm(dict.get("/CFM"))?;
    let length = dict
        .get("/Length")
        .and_then(|l| l.as_int())
        .map(|l| l as u32);
    let auth_event = parse_auth_event(dict.get("/AuthEvent")).unwrap_or(AuthEvent::DocOpen);

    Some(CryptFilterDef {
        cfm,
        length,
        auth_event,
    })
}

/// Parse crypt filter method (/CFM).
fn parse_cfm(obj: Option<&PdfObject>) -> Option<CryptFilterMethod> {
    match obj {
        Some(PdfObject::Name(name)) => match name.strip_prefix('/') {
            Some("Identity") => Some(CryptFilterMethod::Identity),
            Some("V2") => Some(CryptFilterMethod::V2),
            Some("AESV2") => Some(CryptFilterMethod::AesV2),
            Some("AESV3") => Some(CryptFilterMethod::AesV3),
            _ => None,
        },
        None => Some(CryptFilterMethod::Identity),
        _ => None,
    }
}

/// Parse auth event (/AuthEvent).
fn parse_auth_event(obj: Option<&PdfObject>) -> Option<AuthEvent> {
    match obj {
        Some(PdfObject::Name(name)) => match name.strip_prefix('/') {
            Some("DocOpen") => Some(AuthEvent::DocOpen),
            Some("EFOpen") => Some(AuthEvent::EfoOpen),
            _ => None,
        },
        None => Some(AuthEvent::DocOpen),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // Mock resolver for testing
    struct MockResolver;

    impl XrefResolver for MockResolver {
        fn resolve(&self, _obj_ref: ObjRef) -> Result<PdfObject, ResolveError> {
            Err(ResolveError::NotFound(ObjRef::new(0, 0)))
        }
    }

    fn make_dict(entries: Vec<(&str, PdfObject)>) -> PdfDict {
        entries.into_iter().map(|(k, v)| (k.into(), v)).collect()
    }

    #[test]
    fn test_no_encrypt_key() {
        let trailer = make_dict(vec![]);
        let resolver = MockResolver;
        let mut diagnostics = Vec::new();

        let result = detect_encryption(&trailer, &resolver, &mut diagnostics);
        assert!(result.is_none());
        assert!(diagnostics.is_empty());
    }

    #[test]
    fn test_v1_r2_rc4_40() {
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(1)),
            ("/R", PdfObject::Integer(2)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ]);

        let trailer = make_dict(vec![
            ("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict))),
            (
                "/ID",
                PdfObject::Array(Box::new(vec![PdfObject::String(Box::new(vec![0u8; 16]))])),
            ),
        ]);

        let resolver = MockResolver;
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

        assert!(result.is_some());
        assert!(diagnostics.is_empty());
        let info = result.unwrap();
        assert_eq!(info.version, 1);
        assert_eq!(info.revision, 2);
        assert_eq!(info.key_length, 40);
        assert_eq!(info.owner_hash.len(), 32);
        assert_eq!(info.user_hash.len(), 32);
        assert_eq!(info.perms, 0xFFFFFFFF);
        assert!(info.crypt_filters.is_none());
    }

    #[test]
    fn test_v5_r6_aes_256() {
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(5)),
            ("/R", PdfObject::Integer(6)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 48]))),
            ("/U", PdfObject::String(Box::new(vec![0u8; 48]))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
            ("/UE", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/OE", PdfObject::String(Box::new(vec![0u8; 32]))),
            (
                "/Perms",
                PdfObject::String(Box::new({
                    let mut perms = [0u8; 16];
                    perms[0..4].copy_from_slice(&0xFFFFFFFFu32.to_le_bytes());
                    perms.to_vec()
                })),
            ),
        ]);

        let trailer = make_dict(vec![
            ("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict))),
            (
                "/ID",
                PdfObject::Array(Box::new(vec![PdfObject::String(Box::new(vec![0u8; 16]))])),
            ),
        ]);

        let resolver = MockResolver;
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

        assert!(result.is_some());
        assert!(diagnostics.is_empty());
        let info = result.unwrap();
        assert_eq!(info.version, 5);
        assert_eq!(info.revision, 6);
        assert_eq!(info.key_length, 256);
        assert_eq!(info.owner_hash.len(), 48);
        assert_eq!(info.user_hash.len(), 48);
        assert_eq!(info.perms, 0xFFFFFFFF);
    }

    #[test]
    fn test_non_standard_filter_emits_diagnostic() {
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Custom".into())),
            ("/V", PdfObject::Integer(1)),
            ("/R", PdfObject::Integer(2)),
        ]);

        let trailer = make_dict(vec![("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict)))]);

        let resolver = MockResolver;
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

        // Non-Standard filter returns None
        assert!(result.is_none());
        // Should emit ENCRYPTION_UNSUPPORTED diagnostic
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::EncryptionUnsupported);
        assert!(diagnostics[0].message.contains("Custom"));
    }

    #[test]
    fn test_invalid_o_length_emits_diagnostic() {
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(1)),
            ("/R", PdfObject::Integer(2)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 31]))), // Wrong length
            ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ]);

        let trailer = make_dict(vec![("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict)))]);

        let resolver = MockResolver;
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

        // Invalid /O length returns None
        assert!(result.is_none());
        // Should emit ENCRYPTION_INVALID_DICT diagnostic
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::EncryptionInvalidDict);
        assert!(diagnostics[0].message.contains("/O"));
        assert!(diagnostics[0].message.contains("expected 32"));
    }

    #[test]
    fn test_invalid_u_length_emits_diagnostic() {
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(1)),
            ("/R", PdfObject::Integer(2)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/U", PdfObject::String(Box::new(vec![0u8; 31]))), // Wrong length
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ]);

        let trailer = make_dict(vec![("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict)))]);

        let resolver = MockResolver;
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

        // Invalid /U length returns None
        assert!(result.is_none());
        // Should emit ENCRYPTION_INVALID_DICT diagnostic
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::EncryptionInvalidDict);
        assert!(diagnostics[0].message.contains("/U"));
        assert!(diagnostics[0].message.contains("expected 32"));
    }

    #[test]
    fn test_v5_invalid_hash_length_emits_diagnostic() {
        // For R>=5, /O and /U should be 48 bytes
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(5)),
            ("/R", PdfObject::Integer(6)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 32]))), // Wrong length for R=6
            ("/U", PdfObject::String(Box::new(vec![0u8; 48]))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ]);

        let trailer = make_dict(vec![("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict)))]);

        let resolver = MockResolver;
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

        // Invalid /O length returns None
        assert!(result.is_none());
        // Should emit ENCRYPTION_INVALID_DICT diagnostic
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(diagnostics[0].code, DiagCode::EncryptionInvalidDict);
        assert!(diagnostics[0].message.contains("/O"));
        assert!(diagnostics[0].message.contains("expected 48"));
    }

    #[test]
    fn test_v4_crypt_filters() {
        let cf_dict = make_dict(vec![
            ("/CFM", PdfObject::Name("/AESV2".into())),
            ("/Length", PdfObject::Integer(128)),
        ]);

        let filters = make_dict(vec![("/Identity", PdfObject::Dict(Box::new(cf_dict)))]);

        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(4)),
            ("/R", PdfObject::Integer(4)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
            ("/StmF", PdfObject::Name("/Identity".into())),
            ("/StrF", PdfObject::Name("/Identity".into())),
            ("/CF", PdfObject::Dict(Box::new(filters))),
        ]);

        let trailer = make_dict(vec![("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict)))]);

        let resolver = MockResolver;
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

        assert!(result.is_some());
        assert!(diagnostics.is_empty());
        let info = result.unwrap();
        assert_eq!(info.version, 4);
        assert!(info.crypt_filters.is_some());
        let cf = info.crypt_filters.unwrap();
        assert_eq!(cf.stream_filter, "Identity");
        assert_eq!(cf.string_filter, "Identity");
        assert_eq!(cf.filters.len(), 1);
    }

    #[test]
    fn test_missing_id() {
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(1)),
            ("/R", PdfObject::Integer(2)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ]);

        let trailer = make_dict(vec![("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict)))]);

        let resolver = MockResolver;
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &resolver, &mut diagnostics);

        assert!(result.is_some());
        assert!(diagnostics.is_empty());
        let info = result.unwrap();
        // Missing /ID should result in empty file_id
        assert!(info.file_id.is_empty());
    }

    #[test]
    fn test_all_v_r_combinations() {
        // Test V=1, R=2 (RC4-40)
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(1)),
            ("/R", PdfObject::Integer(2)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ]);
        let trailer = make_dict(vec![
            ("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict))),
            (
                "/ID",
                PdfObject::Array(Box::new(vec![PdfObject::String(Box::new(vec![0u8; 16]))])),
            ),
        ]);
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &MockResolver, &mut diagnostics);
        assert!(result.is_some());
        assert_eq!(result.unwrap().key_length, 40);

        // Test V=2, R=3 (RC4-128)
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(2)),
            ("/R", PdfObject::Integer(3)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ]);
        let trailer = make_dict(vec![
            ("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict))),
            (
                "/ID",
                PdfObject::Array(Box::new(vec![PdfObject::String(Box::new(vec![0u8; 16]))])),
            ),
        ]);
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &MockResolver, &mut diagnostics);
        assert!(result.is_some());
        assert_eq!(result.unwrap().key_length, 40);

        // Test V=4, R=4 (RC4 or AES-128)
        let encrypt_dict = make_dict(vec![
            ("/Filter", PdfObject::Name("Standard".into())),
            ("/V", PdfObject::Integer(4)),
            ("/R", PdfObject::Integer(4)),
            ("/O", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/U", PdfObject::String(Box::new(vec![0u8; 32]))),
            ("/P", PdfObject::Integer(0xFFFFFFFF_i64)),
        ]);
        let trailer = make_dict(vec![
            ("/Encrypt", PdfObject::Dict(Box::new(encrypt_dict))),
            (
                "/ID",
                PdfObject::Array(Box::new(vec![PdfObject::String(Box::new(vec![0u8; 16]))])),
            ),
        ]);
        let mut diagnostics = Vec::new();
        let result = detect_encryption(&trailer, &MockResolver, &mut diagnostics);
        assert!(result.is_some());
        assert_eq!(result.unwrap().key_length, 128);
    }
}
