//! Digital signature field discovery and metadata extraction.
//!
//! This module implements Phase 7.3 of the plan: digital signature metadata.
//! It walks the AcroForm /Fields array to discover signature fields, extracts
//! metadata from signature dictionaries, and computes coverage statistics.
//!
//! ## Architecture
//!
//! - **Discovery** (7.3.1): Walk /Fields recursively, filter to /FT /Sig
//! - **Metadata extraction** (7.3.2): Extract /V dict properties (signer, date, reason, etc.)
//! - **Validation** (out of scope): Cryptographic validation requires certificate chains
//!
//! ## Reuse
//!
//! The `walk_acroform_fields` helper is designed for reuse by Phase 7.4 (form fields),
//! which walks the same tree but filters to all field types, not just /Sig.

use crate::parser::catalog::Catalog;
use crate::parser::object::{ObjRef, PdfObject, PdfDict, intern};
use crate::parser::xref::XrefResolver;
use crate::diagnostics::{Diagnostic, DiagCode};
use std::sync::Arc;

/// Result type for signature operations.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// A reference to a signature field in the AcroForm.
///
/// Represents a discovered signature field with its location and metadata.
/// This is the output of the discovery phase (7.3.1); metadata extraction
/// happens in 7.3.2.
#[derive(Debug, Clone, PartialEq)]
pub struct SigFieldRef {
    /// Absolute (dot-joined) field name, e.g., "employer_signature" or "form.employee_sig"
    pub full_name: String,

    /// Indirect reference to the /V dictionary (signature value) if present.
    ///
    /// Absent means the field exists but is unsigned (blank signature field).
    /// Present means the field has been signed at least once.
    pub v_ref: Option<ObjRef>,

    /// Bounding rectangle for the signature appearance on the page.
    ///
    /// Format: [x0, y0, x1, y1] in PDF user-space points.
    /// None if the field has no visual appearance (form-only signature).
    pub rect: Option<[f32; 4]>,

    /// Index of the page containing this signature field's widget annotation.
    ///
    /// None if the field has no widget on any page (form-only signature).
    pub page_index: Option<usize>,

    /// The field's own indirect reference.
    pub field_ref: ObjRef,
}

/// A digital signature with extracted metadata.
///
/// Represents a fully-extracted signature from a PDF signature field,
/// including signer identity, timestamp, and coverage information.
///
/// This is the output of Phase 7.3.2 (metadata extraction) and the
/// primary type emitted in the document-level `/signatures` array.
#[derive(Debug, Clone, PartialEq)]
pub struct Signature {
    /// The absolute (dot-joined) field name from the AcroForm.
    /// Example: "employer_signature" or "form.employee_sig"
    pub field_name: String,

    /// The signer's name from the /Name entry in the signature dictionary.
    ///
    /// Empty string if /Name is absent (not null — default to "").
    pub signer_name: String,

    /// The signing date as an ISO 8601 string (RFC 3339 format).
    ///
    /// Parsed from the PDF /M date string (D:YYYYMMDDHHmmSSOHH'mm format).
    /// None if the date is missing, malformed, or the field is unsigned.
    ///
    /// Format: "YYYY-MM-DDTHH:MM:SS+HH:MM" or "YYYY-MM-DDTHH:MM:SSZ"
    pub signing_date: Option<String>,

    /// The reason for signing from the /Reason entry.
    ///
    /// None if /Reason is absent.
    pub reason: Option<String>,

    /// The location of signing from the /Location entry.
    ///
    /// None if /Location is absent.
    pub location: Option<String>,

    /// The signature format / filter from the /SubFilter entry.
    ///
    /// Indicates the signature format: "adbe.pkcs7.detached", "adbe.x509.rsa.sha1", etc.
    /// None if /SubFilter is absent.
    pub sub_filter: Option<String>,

    /// The /ByteRange array defining which bytes of the file are signed.
    ///
    /// Format: [offset, length, offset, length] defining two byte ranges.
    /// The first range covers the file up to the signature; the second covers
    /// the file after the signature. The signature value itself is NOT covered.
    ///
    /// None if /ByteRange is missing or malformed.
    pub byte_range: Option<Vec<u64>>,

    /// Fraction of the file covered by the signature (0.0 to 1.0).
    ///
    /// Computed as `(byte_range[1] + byte_range[3]) / file_size`.
    /// None if /ByteRange is missing, malformed, or file_size is unknown.
    ///
    /// Values < 1.0 indicate partial signatures (a common red flag for tampered docs).
    pub coverage_fraction: Option<f64>,

    /// Validation status — always "not_checked" in v1.
    ///
    /// Future versions may add "valid", "invalid", "indeterminate" as cryptographic
    /// validation is implemented. This is a string enum for schema stability.
    pub validation_status: String,
}

impl Signature {
    /// Create a new unsigned signature (field exists but /V is absent).
    fn unsigned(field_name: String) -> Self {
        Signature {
            field_name,
            signer_name: String::new(),
            signing_date: None,
            reason: None,
            location: None,
            sub_filter: None,
            byte_range: None,
            coverage_fraction: None,
            validation_status: "not_checked".to_string(),
        }
    }
}

/// Parse a PDF date string to ISO 8601 (RFC 3339) format.
///
/// Per PDF 1.7 spec section 7.9.4 "Dates":
/// - Format: D:YYYYMMDDHHmmSSOHH'mm
/// - D: is a literal prefix
/// - YYYY = year (4 digits)
/// - MM = month (01-12)
/// - DD = day (01-31)
/// - HH = hour (00-23)
/// - mm = minute (00-59)
/// - SS = second (00-59)
/// - O = relationship to UTC: +, -, or Z
/// - HH'mm = UTC offset hours and minutes
///
/// The function tolerates truncated dates (date only, no time, no timezone)
/// by filling defaults: 00 for time components, Z for timezone.
///
/// # Arguments
///
/// * `pdf_date` - The raw PDF date string from the /M entry
///
/// # Returns
///
/// * `Some(String)` - ISO 8601 formatted date if parsing succeeds
/// * `None` - If the input is malformed or empty
///
/// # Examples
///
/// ```ignore
/// // Full date with timezone
/// parse_pdf_date(b"D:20230115143045+05'30'"); // Some("2023-01-15T14:30:45+05:30")
///
/// // UTC timezone
/// parse_pdf_date(b"D:20230115143045Z"); // Some("2023-01-15T14:30:45Z")
///
/// // Date only (truncated)
/// parse_pdf_date(b"D:20230115"); // Some("2023-01-15T00:00:00Z")
///
/// // Malformed
/// parse_pdf_date(b"invalid"); // None
/// ```
fn parse_pdf_date(pdf_date: &[u8]) -> Option<String> {
    // PDF date strings are typically PDFDocEncoding or ASCII, so we can
    // work with them directly as UTF-8 lossy conversion
    let date_str = std::str::from_utf8(pdf_date).ok()?;

    // Strip the D: prefix if present
    let date_str = if date_str.starts_with("D:") {
        &date_str[2..]
    } else {
        date_str
    };

    // Minimum length: YYYYMMDD = 8 characters
    if date_str.len() < 8 {
        return None;
    }

    // Parse year, month, day (required)
    let year = date_str[0..4].parse::<u32>().ok()?;
    let month = date_str[4..6].parse::<u32>().ok()?;
    let day = date_str[6..8].parse::<u32>().ok()?;

    // Validate date ranges
    if month == 0 || month > 12 || day == 0 || day > 31 {
        return None;
    }

    // Parse time components if present
    let (hour, minute, second) = if date_str.len() >= 14 {
        let hour = date_str[8..10].parse::<u32>().ok()?;
        let minute = date_str[10..12].parse::<u32>().ok()?;
        let second = date_str[12..14].parse::<u32>().ok()?;

        // Validate time ranges
        if hour > 23 || minute > 59 || second > 59 {
            return None;
        }
        (hour, minute, second)
    } else {
        // Default to midnight if time not present
        (0, 0, 0)
    };

    // Parse timezone if present
    let tz_str = if date_str.len() > 14 {
        &date_str[14..]
    } else {
        ""
    };

    let timezone = if tz_str.is_empty() || tz_str == "Z" {
        // Default to UTC if no timezone specified
        "Z".to_string()
    } else if tz_str.starts_with('+') || tz_str.starts_with('-') {
        // Parse OHH'mm format (e.g., +05'30' or -08'00')
        let sign = if tz_str.starts_with('+') { "+" } else { "-" };

        // Extract HH and mm from format like +05'30' or +0530
        let tz_digits: String = tz_str[1..].chars().filter(|c| c.is_ascii_digit()).collect();
        if tz_digits.len() >= 4 {
            let tz_hour = &tz_digits[0..2];
            let tz_min = &tz_digits[2..4];
            // Check if this is UTC (+00'00' or +0000)
            if tz_hour == "00" && tz_min == "00" {
                "Z".to_string()
            } else {
                format!("{}{}:{}", sign, tz_hour, tz_min)
            }
        } else {
            // Malformed timezone, default to UTC
            "Z".to_string()
        }
    } else {
        // Unknown format, default to UTC
        "Z".to_string()
    };

    // Format as ISO 8601: YYYY-MM-DDTHH:MM:SS+HH:MM
    Some(format!(
        "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
        year, month, day, hour, minute, second, timezone
    ))
}

/// Decode a PDF text string to UTF-8.
///
/// Per PDF 1.7 spec section "Text String Type":
/// - If the string starts with UTF-16BE BOM (0xFE 0xFF), decode as UTF-16BE
/// - Otherwise, decode as PDFDocEncoding (Latin-1 with named character overrides)
///
/// This is a copy of the function from outline.rs; the original is private
/// to that module. We duplicate it here to avoid coupling the modules.
fn decode_pdf_string(bytes: &[u8]) -> Result<String> {
    // Check for UTF-16BE BOM
    if bytes.len() >= 2 && bytes[0] == 0xFE && bytes[1] == 0xFF {
        return decode_utf16be_bom(&bytes[2..]);
    }

    // Check for UTF-16BE without BOM (heuristic: every other byte is 0x00 for non-ASCII)
    if looks_like_utf16be(bytes) {
        if let Ok(s) = decode_utf16be_raw(bytes) {
            return Ok(s);
        }
    }

    // Fall back to PDFDocEncoding (treat as Latin-1 for basic use)
    decode_pdfdocencoding(bytes)
}

/// Decode UTF-16BE string with BOM (bytes after 0xFE 0xFF).
fn decode_utf16be_bom(bytes: &[u8]) -> Result<String> {
    if bytes.len() % 2 != 0 {
        return Err(vec![
            Diagnostic::with_static_no_offset(
                DiagCode::StructInvalidUtf16,
                "STRUCT_INVALID_UTF16: UTF-16BE string has odd length",
            )
        ]);
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).map_err(|_| {
        vec![
            Diagnostic::with_static_no_offset(
                DiagCode::StructInvalidUtf16,
                "STRUCT_INVALID_UTF16: Invalid UTF-16BE sequence",
            )
        ]
    })
}

/// Decode raw UTF-16BE (without BOM).
fn decode_utf16be_raw(bytes: &[u8]) -> std::result::Result<String, ()> {
    if bytes.len() % 2 != 0 {
        return Err(());
    }

    let utf16_chars: Vec<u16> = bytes
        .chunks_exact(2)
        .map(|chunk| u16::from_be_bytes([chunk[0], chunk[1]]))
        .collect();

    String::from_utf16(&utf16_chars).map_err(|_| ())
}

/// Heuristic check if bytes look like UTF-16BE.
///
/// Returns true if:
/// - Length is even
/// - Most high bytes (first byte of each pair) are 0x00
fn looks_like_utf16be(bytes: &[u8]) -> bool {
    if bytes.len() < 2 || bytes.len() % 2 != 0 {
        return false;
    }

    let mut zero_high_bytes = 0;
    let total_pairs = bytes.len() / 2;

    for chunk in bytes.chunks_exact(2) {
        if chunk[0] == 0x00 {
            zero_high_bytes += 1;
        }
    }

    zero_high_bytes >= total_pairs * 3 / 4
}

/// Decode PDFDocEncoding (Latin-1 subset).
///
/// PDFDocEncoding is defined in PDF spec Annex D.2.
/// For basic use, we treat it as Latin-1 (ISO-8859-1).
fn decode_pdfdocencoding(bytes: &[u8]) -> Result<String> {
    // Latin-1 bytes 0-255 map directly to Unicode code points 0-255
    let s: String = bytes.iter().map(|&b| b as char).collect();
    Ok(s)
}

/// Extract metadata for a single signature field.
///
/// This is the core of Phase 7.3.2: resolve the /V dictionary and extract
/// all signature metadata fields (signer, date, reason, location, subfilter,
/// byte range, coverage fraction).
///
/// # Arguments
///
/// * `field_ref` - The signature field reference from discovery
/// * `resolver` - Xref resolver for dereferencing indirect objects
/// * `file_size` - Total size of the PDF file in bytes (for coverage computation)
///
/// # Returns
///
/// A `Signature` struct with all extracted metadata. If the field has no /V
/// (unsigned), returns an unsigned signature with minimal metadata.
fn extract_signature_metadata(
    field_ref: &SigFieldRef,
    resolver: &XrefResolver,
    file_size: Option<u64>,
) -> Signature {
    // If no /V reference, the field is unsigned
    let v_ref = match field_ref.v_ref {
        Some(ref_) => ref_,
        None => return Signature::unsigned(field_ref.full_name.clone()),
    };

    // Resolve the /V dictionary (signature dictionary)
    let v_obj = match resolver.resolve(v_ref) {
        Ok(obj) => obj,
        Err(_) => return Signature::unsigned(field_ref.full_name.clone()),
    };

    let v_dict = match v_obj.as_dict() {
        Some(d) => d,
        None => return Signature::unsigned(field_ref.full_name.clone()),
    };

    // Extract /Name (signer name) - default to empty string if absent
    let signer_name = v_dict.get("Name")
        .and_then(|o| o.as_string())
        .and_then(|bytes| decode_pdf_string(bytes).ok())
        .unwrap_or_else(String::new);

    // Extract /M (signing date) - parse to ISO 8601
    let signing_date = v_dict.get("M")
        .and_then(|o| o.as_string())
        .and_then(|bytes| parse_pdf_date(bytes));

    // Extract /Reason (optional)
    let reason = v_dict.get("Reason")
        .and_then(|o| o.as_string())
        .and_then(|bytes| decode_pdf_string(bytes).ok());

    // Extract /Location (optional)
    let location = v_dict.get("Location")
        .and_then(|o| o.as_string())
        .and_then(|bytes| decode_pdf_string(bytes).ok());

    // Extract /SubFilter (signature format) - this is a Name, not a String
    let sub_filter = v_dict.get("SubFilter")
        .and_then(|o| o.as_name())
        .map(|n| n.to_string());

    // Extract /ByteRange (array of 4 integers: [offset, length, offset, length])
    let byte_range = v_dict.get("ByteRange")
        .and_then(|o| o.as_array())
        .and_then(|arr| {
            if arr.len() != 4 {
                return None;
            }
            let mut result = Vec::with_capacity(4);
            for item in arr.iter() {
                let val = item.as_int().or_else(|| item.as_real().map(|r| r as i64))?;
                if val < 0 {
                    return None;
                }
                result.push(val as u64);
            }
            Some(result)
        });

    // Compute coverage_fraction: (byte_range[1] + byte_range[3]) / file_size
    let coverage_fraction = match (byte_range.as_ref(), file_size) {
        (Some(br), Some(fs)) if fs > 0 => {
            let covered = br[1].saturating_add(br[3]);
            Some(covered as f64 / fs as f64)
        }
        _ => None,
    };

    Signature {
        field_name: field_ref.full_name.clone(),
        signer_name,
        signing_date,
        reason,
        location,
        sub_filter,
        byte_range,
        coverage_fraction,
        validation_status: "not_checked".to_string(),
    }
}

/// Extract metadata for all discovered signature fields.
///
/// This is the main entry point for Phase 7.3.2. Takes the output of
/// 7.3.1 discovery and resolves all signature dictionaries to extract
/// metadata.
///
/// # Arguments
///
/// * `fields` - Discovered signature fields from `discover()`
/// * `resolver` - Xref resolver for dereferencing indirect objects
/// * `file_size` - Total size of the PDF file in bytes (for coverage computation)
///
/// # Returns
///
/// A `Vec<Signature>` containing extracted metadata for all signature fields.
/// Unsigned fields (no /V) are included with minimal metadata.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::signature::{discover, extract_signatures};
///
/// let sig_fields = discover(&resolver, &catalog);
/// let signatures = extract_signatures(&sig_fields, &resolver, Some(file_size));
///
/// for sig in signatures {
///     println!("Signature: {}", sig.field_name);
///     println!("  Signer: {}", sig.signer_name);
///     if let Some(date) = &sig.signing_date {
///         println!("  Date: {}", date);
///     }
/// }
/// ```
pub fn extract_signatures(
    fields: &[SigFieldRef],
    resolver: &XrefResolver,
    file_size: Option<u64>,
) -> Vec<Signature> {
    fields
        .iter()
        .map(|field| extract_signature_metadata(field, resolver, file_size))
        .collect()
}

/// A field reference from AcroForm walking.
///
/// Internal type used by `walk_acroform_fields` to represent any field
/// (signature, text, button, choice). This is the reusable primitive that
/// 7.4 will consume directly.
#[derive(Debug, Clone)]
struct FieldRef {
    /// Absolute (dot-joined) field name
    full_name: String,

    /// Field type (/FT): Tx, Btn, Ch, Sig (or None if inherited)
    field_type: Option<String>,

    /// Indirect reference to /V (current value) if present
    v_ref: Option<ObjRef>,

    /// Bounding rectangle if present
    rect: Option<[f32; 4]>,

    /// Page index if resolvable
    page_index: Option<usize>,

    /// The field's own indirect reference
    field_ref: ObjRef,

    /// Parent field type (for /FT inheritance)
    parent_ft: Option<String>,
}

impl FieldRef {
    /// Check if this field is a signature field.
    ///
    /// A field is a signature field if its /FT (or inherited /FT) is /Sig.
    fn is_signature(&self) -> bool {
        let ft = self.field_type.as_ref().or(self.parent_ft.as_ref());
        ft.map(|t| t == "Sig").unwrap_or(false)
    }

    /// Convert to SigFieldRef if this is a signature field.
    fn into_sig_field(self) -> Option<SigFieldRef> {
        if self.is_signature() {
            Some(SigFieldRef {
                full_name: self.full_name,
                v_ref: self.v_ref,
                rect: self.rect,
                page_index: self.page_index,
                field_ref: self.field_ref,
            })
        } else {
            None
        }
    }
}

/// Walk the AcroForm /Fields array recursively and collect all fields.
///
/// This is the reusable walker that both signature discovery (7.3) and
/// form field extraction (7.4) will use. It performs DFS traversal of
/// the /Kids hierarchy, resolves /FT inheritance, and constructs absolute
/// field names.
///
/// # Arguments
///
/// * `resolver` - Xref resolver for dereferencing indirect objects
/// * `catalog` - Document catalog containing the AcroForm reference
///
/// # Returns
///
/// A `Vec<FieldRef>` containing all discovered fields (not just signatures).
///
/// # Behavior
///
/// - If /AcroForm is absent, returns empty vec (not an error)
/// - If /Fields is absent or empty, returns empty vec
/// - Descends recursively via /Kids arrays
/// - Resolves /FT inheritance from parent to child fields
/// - Constructs absolute names by joining /T values with "."
/// - Emits diagnostics for malformed structures but continues
fn walk_acroform_fields(
    resolver: &XrefResolver,
    catalog: &Catalog,
) -> Vec<FieldRef> {
    let mut fields = Vec::new();
    let mut diagnostics = Vec::new();

    // AcroForm is optional; absent means no fields
    let acroform_ref = match catalog.acroform_ref {
        Some(ref_) => ref_,
        None => return fields,
    };

    // Resolve the AcroForm dictionary
    let acroform = match resolver.resolve(acroform_ref) {
        Ok(obj) => obj,
        Err(_) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve /AcroForm reference {}", acroform_ref),
            ));
            return fields;
        }
    };

    let acroform_dict = match acroform.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("/AcroForm is not a dictionary (type: {})", acroform.type_name()),
            ));
            return fields;
        }
    };

    // /Fields is an array of indirect references to field dictionaries
    let fields_array = match acroform_dict.get("Fields").and_then(|o| o.as_array()) {
        Some(arr) => arr,
        None => return fields, // No /Fields means no form fields
    };

    // Walk each field in the /Fields array
    for field_obj in fields_array.iter() {
        let field_ref = match field_obj {
            PdfObject::Ref(ref_) => *ref_,
            _ => continue, // Skip non-reference entries
        };

        walk_field_recursive(
            resolver,
            field_ref,
            &mut fields,
            String::new(),
            None,
            &mut diagnostics,
        );
    }

    fields
}

/// Recursively walk a field dictionary and its /Kids.
///
/// This helper function performs DFS traversal of the field hierarchy,
/// building absolute field names and tracking /FT inheritance.
///
/// # Arguments
///
/// * `resolver` - Xref resolver
/// * `field_ref` - Indirect reference to the current field dictionary
/// * `fields` - Output accumulator for discovered fields
/// * `parent_name` - Accumulated absolute name from parent path
/// * `parent_ft` - Inherited field type from parent (/FT value)
/// * `diagnostics` - Diagnostic accumulator
fn walk_field_recursive(
    resolver: &XrefResolver,
    field_ref: ObjRef,
    fields: &mut Vec<FieldRef>,
    parent_name: String,
    parent_ft: Option<String>,
    diagnostics: &mut Vec<Diagnostic>,
) {
    // Resolve the field dictionary
    let field_obj = match resolver.resolve(field_ref) {
        Ok(obj) => obj,
        Err(_) => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Failed to resolve field reference {}", field_ref),
            ));
            return;
        }
    };

    let field_dict = match field_obj.as_dict() {
        Some(d) => d,
        None => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!("Field {} is not a dictionary", field_ref),
            ));
            return;
        }
    };

    // Extract /T (partial name) for building absolute name
    let partial_name = field_dict.get("T")
        .and_then(|o| o.as_string())
        .and_then(|bytes| String::from_utf8(bytes.to_vec()).ok());

    // Build the absolute field name
    let full_name = if let Some(ref pn) = partial_name {
        if parent_name.is_empty() {
            pn.clone()
        } else {
            format!("{}.{}", parent_name, pn)
        }
    } else {
        parent_name.clone()
    };

    // Extract /FT (field type) - may be absent on child fields (inherit from parent)
    let field_type = field_dict.get("FT")
        .and_then(|o| o.as_name())
        .map(|n| n.to_string());

    // Use parent's /FT if this field doesn't have one
    let effective_ft = field_type.as_ref().or(parent_ft.as_ref());

    // Extract /V (current value) if present
    let v_ref = field_dict.get("V")
        .and_then(|o| match o {
            PdfObject::Ref(r) => Some(*r),
            _ => None,
        });

    // Extract /Rect (bounding rectangle) if present
    let rect = field_dict.get("Rect")
        .and_then(|o| o.as_array())
        .and_then(|arr| {
            if arr.len() == 4 {
                let coords: Vec<Option<f64>> = arr.iter()
                    .map(|o| o.as_real().or_else(|| o.as_int().map(|i| i as f64)))
                    .collect();
                if coords.iter().all(|c| c.is_some()) {
                    Some([
                        coords[0].unwrap() as f32,
                        coords[1].unwrap() as f32,
                        coords[2].unwrap() as f32,
                        coords[3].unwrap() as f32,
                    ])
                } else {
                    None
                }
            } else {
                None
            }
        });

    // TODO: Resolve page_index by searching page /Annots arrays
    // This requires access to the page tree, which we don't have here.
    // For now, page_index is always None.
    let page_index = None;

    // Check for /Kids (nested fields)
    let kids = field_dict.get("Kids").and_then(|o| o.as_array());

    if let Some(kids_array) = kids {
        // This is a parent field with children - recurse into /Kids
        for kid_obj in kids_array.iter() {
            let kid_ref = match kid_obj {
                PdfObject::Ref(ref_) => *ref_,
                _ => continue,
            };

            walk_field_recursive(
                resolver,
                kid_ref,
                fields,
                full_name.clone(),
                effective_ft.map(|s| s.clone()),
                diagnostics,
            );
        }
    } else {
        // This is a leaf field - emit it
        fields.push(FieldRef {
            full_name,
            field_type,
            v_ref,
            rect,
            page_index,
            field_ref,
            parent_ft,
        });
    }
}

/// Discover all signature fields in the PDF document.
///
/// This is the main entry point for Phase 7.3.1: signature field discovery.
/// It walks the AcroForm /Fields array and filters to fields whose /FT
/// (field type) is /Sig.
///
/// # Arguments
///
/// * `resolver` - Xref resolver for dereferencing indirect objects
/// * `catalog` - Document catalog containing the AcroForm reference
///
/// # Returns
///
/// A `Vec<SigFieldRef>` containing all discovered signature fields.
/// Returns empty vec if the PDF has no AcroForm or no signature fields.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::signature::discover;
///
/// let sig_fields = discover(&resolver, &catalog);
/// for sig in sig_fields {
///     println!("Signature field: {}", sig.full_name);
///     if let Some(v_ref) = sig.v_ref {
///         println!("  Signed: {}", v_ref);
///     } else {
///         println!("  Unsigned (blank)");
///     }
/// }
/// ```
pub fn discover(
    resolver: &XrefResolver,
    catalog: &Catalog,
) -> Vec<SigFieldRef> {
    walk_acroform_fields(resolver, catalog)
        .into_iter()
        .filter_map(|f| f.into_sig_field())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, PdfObject};

    /// Helper to create a test catalog with an AcroForm.
    fn make_test_acroform(fields: Vec<PdfObject>) -> (Catalog, XrefResolver) {
        let mut resolver = XrefResolver::new();

        // Create the AcroForm dictionary
        let mut acroform_dict = indexmap::IndexMap::new();
        acroform_dict.insert(intern("Fields"), PdfObject::Array(Box::new(fields)));

        let acroform_ref = ObjRef::new(10, 0);
        resolver.cache_object(acroform_ref, PdfObject::Dict(Box::new(acroform_dict)));

        // Create a minimal catalog
        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.acroform_ref = Some(acroform_ref);

        (catalog, resolver)
    }

    /// Helper to create a field dictionary with a specific ID.
    fn make_field_dict_with_id(
        id: u32,
        ft: Option<&str>,
        t: Option<&str>,
        v: Option<ObjRef>,
        rect: Option<[f32; 4]>,
        kids: Option<Vec<ObjRef>>,
    ) -> (ObjRef, PdfObject) {
        let mut dict = indexmap::IndexMap::new();

        if let Some(ft_val) = ft {
            dict.insert(intern("FT"), PdfObject::Name(intern(ft_val)));
        }

        if let Some(t_val) = t {
            dict.insert(intern("T"), PdfObject::String(Box::new(t_val.as_bytes().to_vec())));
        }

        if let Some(v_ref) = v {
            dict.insert(intern("V"), PdfObject::Ref(v_ref));
        }

        if let Some(rect_val) = rect {
            let rect_array: Vec<PdfObject> = rect_val.iter()
                .map(|&c| PdfObject::Real(c as f64))
                .collect();
            dict.insert(intern("Rect"), PdfObject::Array(Box::new(rect_array)));
        }

        if let Some(kids_refs) = kids {
            let kids_array: Vec<PdfObject> = kids_refs.iter()
                .map(|&r| PdfObject::Ref(r))
                .collect();
            dict.insert(intern("Kids"), PdfObject::Array(Box::new(kids_array)));
        }

        let field_ref = ObjRef::new(100 + id, 0);
        (field_ref, PdfObject::Dict(Box::new(dict)))
    }

    #[test]
    fn test_discover_no_acroform() {
        let catalog = Catalog::new(ObjRef::new(1, 0));
        let resolver = XrefResolver::new();

        let sig_fields = discover(&resolver, &catalog);

        assert!(sig_fields.is_empty());
    }

    #[test]
    fn test_discover_no_fields() {
        let mut resolver = XrefResolver::new();

        let acroform_ref = ObjRef::new(10, 0);
        let acroform_dict = indexmap::IndexMap::new();
        resolver.cache_object(acroform_ref, PdfObject::Dict(Box::new(acroform_dict)));

        let mut catalog = Catalog::new(ObjRef::new(1, 0));
        catalog.acroform_ref = Some(acroform_ref);

        let sig_fields = discover(&resolver, &catalog);

        assert!(sig_fields.is_empty());
    }

    #[test]
    fn test_discover_two_flat_signatures() {
        let (field1_ref, field1) = make_field_dict_with_id(
            1,
            Some("Sig"),
            Some("employer_sig"),
            None,
            None,
            None,
        );

        let (field2_ref, field2) = make_field_dict_with_id(
            2,
            Some("Sig"),
            Some("employee_sig"),
            None,
            None,
            None,
        );

        let fields = vec![
            PdfObject::Ref(field1_ref),
            PdfObject::Ref(field2_ref),
        ];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(field1_ref, field1);
        resolver.cache_object(field2_ref, field2);

        let sig_fields = discover(&resolver, &catalog);

        assert_eq!(sig_fields.len(), 2);

        let sig1 = sig_fields.iter().find(|s| s.full_name == "employer_sig").unwrap();
        assert_eq!(sig1.full_name, "employer_sig");
        assert!(sig1.v_ref.is_none());

        let sig2 = sig_fields.iter().find(|s| s.full_name == "employee_sig").unwrap();
        assert_eq!(sig2.full_name, "employee_sig");
        assert!(sig2.v_ref.is_none());
    }

    #[test]
    fn test_discover_non_signature_fields_excluded() {
        let (text_field_ref, text_field) = make_field_dict_with_id(
            1,
            Some("Tx"),
            Some("employee_name"),
            None,
            None,
            None,
        );

        let (sig_field_ref, sig_field) = make_field_dict_with_id(
            2,
            Some("Sig"),
            Some("employee_sig"),
            None,
            None,
            None,
        );

        let fields = vec![
            PdfObject::Ref(text_field_ref),
            PdfObject::Ref(sig_field_ref),
        ];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(text_field_ref, text_field);
        resolver.cache_object(sig_field_ref, sig_field);

        let sig_fields = discover(&resolver, &catalog);

        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].full_name, "employee_sig");
    }

    #[test]
    fn test_discover_nested_signature_inherits_ft() {
        // Parent field with /FT /Sig and /Kids array
        let (kid_field_ref, kid_field) = make_field_dict_with_id(
            2,
            None, // No /FT on child - inherits from parent
            Some("sub_sig"),
            None,
            None,
            None,
        );

        let (parent_field_ref, parent_field) = make_field_dict_with_id(
            1,
            Some("Sig"), // Parent has /FT /Sig
            Some("parent_sig"),
            None,
            None,
            Some(vec![kid_field_ref]),
        );

        let fields = vec![PdfObject::Ref(parent_field_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(parent_field_ref, parent_field);
        resolver.cache_object(kid_field_ref, kid_field);

        let sig_fields = discover(&resolver, &catalog);

        // Should find the nested signature field
        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].full_name, "parent_sig.sub_sig");
    }

    #[test]
    fn test_discover_nested_mixed_field_types() {
        // Parent with /FT /Sig has two kids: one inherits, one overrides
        let (kid1_ref, kid1) = make_field_dict_with_id(
            2,
            None, // Inherits /FT /Sig from parent
            Some("kid1"),
            None,
            None,
            None,
        );

        let (kid2_ref, kid2) = make_field_dict_with_id(
            3,
            Some("Tx"), // Overrides to text field
            Some("kid2"),
            None,
            None,
            None,
        );

        let (parent_ref, parent) = make_field_dict_with_id(
            1,
            Some("Sig"),
            Some("parent"),
            None,
            None,
            Some(vec![kid1_ref, kid2_ref]),
        );

        let fields = vec![PdfObject::Ref(parent_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(parent_ref, parent);
        resolver.cache_object(kid1_ref, kid1);
        resolver.cache_object(kid2_ref, kid2);

        let sig_fields = discover(&resolver, &catalog);

        // Only kid1 should be a signature (inherits /FT /Sig)
        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].full_name, "parent.kid1");
    }

    #[test]
    fn test_discover_with_rect() {
        let (field_ref, field) = make_field_dict_with_id(
            1,
            Some("Sig"),
            Some("signature"),
            None,
            Some([100.0, 200.0, 300.0, 400.0]),
            None,
        );

        let fields = vec![PdfObject::Ref(field_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(field_ref, field);

        let sig_fields = discover(&resolver, &catalog);

        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].rect, Some([100.0, 200.0, 300.0, 400.0]));
    }

    #[test]
    fn test_discover_with_v_ref() {
        let v_ref = ObjRef::new(999, 0);

        let (field_ref, field) = make_field_dict_with_id(
            1,
            Some("Sig"),
            Some("signature"),
            Some(v_ref),
            None,
            None,
        );

        let fields = vec![PdfObject::Ref(field_ref)];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(field_ref, field);

        let sig_fields = discover(&resolver, &catalog);

        assert_eq!(sig_fields.len(), 1);
        assert_eq!(sig_fields[0].v_ref, Some(v_ref));
    }

    #[test]
    fn test_walk_acroform_fields_reusable() {
        // Verify that walk_acroform_fields returns all field types
        let (text_ref, text) = make_field_dict_with_id(
            1,
            Some("Tx"),
            Some("text_field"),
            None,
            None,
            None,
        );

        let (sig_ref, sig) = make_field_dict_with_id(
            2,
            Some("Sig"),
            Some("sig_field"),
            None,
            None,
            None,
        );

        let fields = vec![
            PdfObject::Ref(text_ref),
            PdfObject::Ref(sig_ref),
        ];

        let (mut catalog, mut resolver) = make_test_acroform(fields);
        resolver.cache_object(text_ref, text);
        resolver.cache_object(sig_ref, sig);

        let all_fields = walk_acroform_fields(&resolver, &catalog);

        assert_eq!(all_fields.len(), 2);

        // Verify field types are preserved
        let text_field = all_fields.iter().find(|f| f.full_name == "text_field").unwrap();
        assert_eq!(text_field.field_type.as_deref(), Some("Tx"));

        let sig_field = all_fields.iter().find(|f| f.full_name == "sig_field").unwrap();
        assert_eq!(sig_field.field_type.as_deref(), Some("Sig"));
    }

    // === Phase 7.3.2: Metadata extraction tests ===

    /// Helper to create a signature dictionary (/V)
    fn make_signature_dict(
        name: Option<&str>,
        m: Option<&[u8]>,
        reason: Option<&str>,
        location: Option<&str>,
        subfilter: Option<&str>,
        byte_range: Option<Vec<i64>>,
    ) -> (ObjRef, PdfObject) {
        let mut dict = indexmap::IndexMap::new();

        if let Some(name_val) = name {
            dict.insert(intern("Name"), PdfObject::String(Box::new(name_val.as_bytes().to_vec())));
        }

        if let Some(m_val) = m {
            dict.insert(intern("M"), PdfObject::String(Box::new(m_val.to_vec())));
        }

        if let Some(reason_val) = reason {
            dict.insert(intern("Reason"), PdfObject::String(Box::new(reason_val.as_bytes().to_vec())));
        }

        if let Some(location_val) = location {
            dict.insert(intern("Location"), PdfObject::String(Box::new(location_val.as_bytes().to_vec())));
        }

        if let Some(subfilter_val) = subfilter {
            dict.insert(intern("SubFilter"), PdfObject::Name(intern(subfilter_val)));
        }

        if let Some(br_val) = byte_range {
            let br_array: Vec<PdfObject> = br_val.iter()
                .map(|&v| PdfObject::Integer(v))
                .collect();
            dict.insert(intern("ByteRange"), PdfObject::Array(Box::new(br_array)));
        }

        let v_ref = ObjRef::new(500, 0);
        (v_ref, PdfObject::Dict(Box::new(dict)))
    }

    #[test]
    fn test_extract_signature_metadata_full() {
        let v_ref = ObjRef::new(500, 0);
        let (v_ref, v_dict) = make_signature_dict(
            Some("John Doe"),
            Some(b"D:20230115143045Z"),
            Some("Contract approval"),
            Some("New York, NY"),
            Some("adbe.pkcs7.detached"),
            Some(vec![0, 1000, 2000, 500]),
        );

        let field = SigFieldRef {
            full_name: "employer_sig".to_string(),
            v_ref: Some(v_ref),
            rect: None,
            page_index: None,
            field_ref: ObjRef::new(100, 0),
        };

        let mut resolver = XrefResolver::new();
        resolver.cache_object(v_ref, v_dict);

        let sig = extract_signature_metadata(&field, &resolver, Some(3000));

        assert_eq!(sig.field_name, "employer_sig");
        assert_eq!(sig.signer_name, "John Doe");
        assert_eq!(sig.signing_date, Some("2023-01-15T14:30:45Z".to_string()));
        assert_eq!(sig.reason, Some("Contract approval".to_string()));
        assert_eq!(sig.location, Some("New York, NY".to_string()));
        assert_eq!(sig.sub_filter, Some("adbe.pkcs7.detached".to_string()));
        assert_eq!(sig.byte_range, Some(vec![0, 1000, 2000, 500]));
        assert_eq!(sig.coverage_fraction, Some(1500.0 / 3000.0)); // (1000 + 500) / 3000
        assert_eq!(sig.validation_status, "not_checked");
    }

    #[test]
    fn test_extract_signature_metadata_unsigned() {
        let field = SigFieldRef {
            full_name: "blank_sig".to_string(),
            v_ref: None, // No /V = unsigned
            rect: None,
            page_index: None,
            field_ref: ObjRef::new(100, 0),
        };

        let resolver = XrefResolver::new();

        let sig = extract_signature_metadata(&field, &resolver, Some(1000));

        assert_eq!(sig.field_name, "blank_sig");
        assert_eq!(sig.signer_name, "");
        assert!(sig.signing_date.is_none());
        assert!(sig.reason.is_none());
        assert!(sig.location.is_none());
        assert!(sig.sub_filter.is_none());
        assert!(sig.byte_range.is_none());
        assert!(sig.coverage_fraction.is_none());
        assert_eq!(sig.validation_status, "not_checked");
    }

    #[test]
    fn test_extract_signature_metadata_missing_optional_fields() {
        let v_ref = ObjRef::new(500, 0);
        let mut dict = indexmap::IndexMap::new();
        dict.insert(intern("Name"), PdfObject::String(Box::new(b"Alice Smith".to_vec())));

        let field = SigFieldRef {
            full_name: "minimal_sig".to_string(),
            v_ref: Some(v_ref),
            rect: None,
            page_index: None,
            field_ref: ObjRef::new(100, 0),
        };

        let mut resolver = XrefResolver::new();
        resolver.cache_object(v_ref, PdfObject::Dict(Box::new(dict)));

        let sig = extract_signature_metadata(&field, &resolver, None);

        assert_eq!(sig.field_name, "minimal_sig");
        assert_eq!(sig.signer_name, "Alice Smith");
        assert!(sig.signing_date.is_none());
        assert!(sig.reason.is_none());
        assert!(sig.location.is_none());
        assert!(sig.sub_filter.is_none());
        assert!(sig.byte_range.is_none());
        assert!(sig.coverage_fraction.is_none());
    }

    #[test]
    fn test_extract_signatures_multiple() {
        // Create two signature fields with different /V dicts
        let v_ref1 = ObjRef::new(500, 0);
        let (_, v_dict1) = make_signature_dict(
            Some("Signer One"),
            Some(b"D:20230101000000Z"),
            None,
            None,
            Some("adbe.pkcs7.detached"),
            None,
        );

        let v_ref2 = ObjRef::new(501, 0);
        let (_, v_dict2) = make_signature_dict(
            Some("Signer Two"),
            Some(b"D:20230201000000Z"),
            Some("Approved"),
            None,
            Some("adbe.x509.rsa.sha1"),
            None,
        );

        let field1 = SigFieldRef {
            full_name: "sig1".to_string(),
            v_ref: Some(v_ref1),
            rect: None,
            page_index: None,
            field_ref: ObjRef::new(100, 0),
        };

        let field2 = SigFieldRef {
            full_name: "sig2".to_string(),
            v_ref: Some(v_ref2),
            rect: None,
            page_index: None,
            field_ref: ObjRef::new(101, 0),
        };

        let fields = vec![field1.clone(), field2.clone()];

        let mut resolver = XrefResolver::new();
        resolver.cache_object(v_ref1, v_dict1);
        resolver.cache_object(v_ref2, v_dict2);

        let sigs = extract_signatures(&fields, &resolver, None);

        assert_eq!(sigs.len(), 2);

        let sig1 = sigs.iter().find(|s| s.field_name == "sig1").unwrap();
        assert_eq!(sig1.signer_name, "Signer One");
        assert_eq!(sig1.sub_filter, Some("adbe.pkcs7.detached".to_string()));

        let sig2 = sigs.iter().find(|s| s.field_name == "sig2").unwrap();
        assert_eq!(sig2.signer_name, "Signer Two");
        assert_eq!(sig2.reason, Some("Approved".to_string()));
        assert_eq!(sig2.sub_filter, Some("adbe.x509.rsa.sha1".to_string()));
    }

    // === PDF date parsing tests ===

    #[test]
    fn test_parse_pdf_date_full_with_timezone() {
        let result = parse_pdf_date(b"D:20230115143045+05'30'");
        assert_eq!(result, Some("2023-01-15T14:30:45+05:30".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_utc() {
        let result = parse_pdf_date(b"D:20230115143045Z");
        assert_eq!(result, Some("2023-01-15T14:30:45Z".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_negative_timezone() {
        let result = parse_pdf_date(b"D:20230115143045-08'00'");
        assert_eq!(result, Some("2023-01-15T14:30:45-08:00".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_only() {
        let result = parse_pdf_date(b"D:20230115");
        assert_eq!(result, Some("2023-01-15T00:00:00Z".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_no_timezone() {
        let result = parse_pdf_date(b"D:20230115143045");
        assert_eq!(result, Some("2023-01-15T14:30:45Z".to_string()));
    }

    #[test]
    fn test_parse_pdf_date_malformed() {
        assert!(parse_pdf_date(b"invalid").is_none());
        assert!(parse_pdf_date(b"D:2023").is_none()); // Too short
        assert!(parse_pdf_date(b"D:20231301").is_none()); // Invalid month
        assert!(parse_pdf_date(b"D:20230132").is_none()); // Invalid day
    }

    #[test]
    fn test_parse_pdf_date_without_d_prefix() {
        let result = parse_pdf_date(b"20230115143045Z");
        assert_eq!(result, Some("2023-01-15T14:30:45Z".to_string()));
    }

    // === ByteRange coverage tests ===

    #[test]
    fn test_coverage_fraction_full_coverage() {
        let v_ref = ObjRef::new(500, 0);
        let (_, v_dict) = make_signature_dict(
            Some("Signer"),
            None,
            None,
            None,
            None,
            Some(vec![0, 1000, 2000, 3000]), // Covers 4000 bytes
        );

        let field = SigFieldRef {
            full_name: "sig".to_string(),
            v_ref: Some(v_ref),
            rect: None,
            page_index: None,
            field_ref: ObjRef::new(100, 0),
        };

        let mut resolver = XrefResolver::new();
        resolver.cache_object(v_ref, v_dict);

        let sig = extract_signature_metadata(&field, &resolver, Some(4000));

        assert_eq!(sig.coverage_fraction, Some(1.0));
    }

    #[test]
    fn test_coverage_fraction_partial() {
        let v_ref = ObjRef::new(500, 0);
        let (_, v_dict) = make_signature_dict(
            Some("Signer"),
            None,
            None,
            None,
            None,
            Some(vec![0, 1000, 2000, 500]), // Covers 1500 bytes
        );

        let field = SigFieldRef {
            full_name: "sig".to_string(),
            v_ref: Some(v_ref),
            rect: None,
            page_index: None,
            field_ref: ObjRef::new(100, 0),
        };

        let mut resolver = XrefResolver::new();
        resolver.cache_object(v_ref, v_dict);

        let sig = extract_signature_metadata(&field, &resolver, Some(3000));

        assert_eq!(sig.coverage_fraction, Some(0.5));
    }

    #[test]
    fn test_coverage_fraction_no_file_size() {
        let v_ref = ObjRef::new(500, 0);
        let (_, v_dict) = make_signature_dict(
            Some("Signer"),
            None,
            None,
            None,
            None,
            Some(vec![0, 1000, 2000, 500]),
        );

        let field = SigFieldRef {
            full_name: "sig".to_string(),
            v_ref: Some(v_ref),
            rect: None,
            page_index: None,
            field_ref: ObjRef::new(100, 0),
        };

        let mut resolver = XrefResolver::new();
        resolver.cache_object(v_ref, v_dict);

        let sig = extract_signature_metadata(&field, &resolver, None);

        assert!(sig.coverage_fraction.is_none());
    }

    #[test]
    fn test_coverage_fraction_invalid_byte_range() {
        let v_ref = ObjRef::new(500, 0);
        // Only 3 elements instead of 4
        let mut dict = indexmap::IndexMap::new();
        dict.insert(intern("Name"), PdfObject::String(Box::new(b"Signer".to_vec())));
        dict.insert(intern("ByteRange"), PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(1000),
            PdfObject::Integer(2000),
        ])));

        let field = SigFieldRef {
            full_name: "sig".to_string(),
            v_ref: Some(v_ref),
            rect: None,
            page_index: None,
            field_ref: ObjRef::new(100, 0),
        };

        let mut resolver = XrefResolver::new();
        resolver.cache_object(v_ref, PdfObject::Dict(Box::new(dict)));

        let sig = extract_signature_metadata(&field, &resolver, Some(3000));

        assert!(sig.byte_range.is_none());
        assert!(sig.coverage_fraction.is_none());
    }

    // === PDF string decoding tests ===

    #[test]
    fn test_decode_pdf_string_ascii() {
        let result = decode_pdf_string(b"Hello World");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hello World");
    }

    #[test]
    fn test_decode_pdf_string_utf16be_bom() {
        let utf16be = vec![0xFE, 0xFF, 0x00, 0x48, 0x00, 0x69]; // "Hi"
        let result = decode_pdf_string(&utf16be);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "Hi");
    }

    #[test]
    fn test_decode_pdf_string_empty() {
        let result = decode_pdf_string(b"");
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "");
    }
}
