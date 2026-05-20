//! Canonicalization functions for fingerprint computation.
//!
//! This module provides utilities for normalizing PDF content to ensure
//! deterministic fingerprinting regardless of producer-tool variations.
//!
//! # Canonicalization
//!
//! Per Phase 1.7 of the implementation plan, fingerprint computation requires
//! canonicalizing inputs to eliminate non-semantic variance:
//!
//! - **Geometry**: Float coordinates are rounded to 4 decimal places using
//!   banker's rounding (round half to even) to eliminate float-representation noise
//! - **Whitespace**: Content streams are re-tokenized and emitted with single
//!   space separators to ignore producer-tool whitespace formatting
//! - **Resource dicts**: Dictionary keys are sorted lexicographically for
//!   deterministic serialization regardless of insertion order

use crate::diagnostics::{Diagnostic, DiagCode};
use crate::parser::lexer::{Lexer, Token};
use std::collections::BTreeMap;
use std::sync::Arc;

use crate::parser::object::{PdfDict, PdfObject};

/// Canonicalize a float to 4 decimal places using banker's rounding.
///
/// Converts f64 to fixed-point i64 via (x * 10000).round_ties_even().
/// This is REQUIRED for deterministic fingerprint computation.
///
/// # Arguments
///
/// * `x` - The float value to canonicalize
/// * `diagnostics` - Optional diagnostics vector to receive STRUCT_INVALID_GEOMETRY errors
///
/// # Returns
///
/// The canonicalized i64 value. NaN and Inf are canonicalized to 0.
///
/// # Examples
///
/// ```
/// use pdftract_core::fingerprint::canonicalize::canonicalize_f64;
///
/// assert_eq!(canonicalize_f64(0.00005, &mut None), 0);  // 0.5 rounds to even (0)
/// assert_eq!(canonicalize_f64(1.23456, &mut None), 12346);
/// assert_eq!(canonicalize_f64(f64::NAN, &mut None), 0);  // NaN -> 0
/// ```
///
/// # Note
///
/// Due to floating point representation, 0.00015 * 10000 = 1.4999... (not exactly 1.5),
/// so it rounds to 1, not 2. This is a known limitation of binary floating point.
pub fn canonicalize_f64(x: f64, diagnostics: &mut Option<Vec<Diagnostic>>) -> i64 {
    if !x.is_finite() {
        // NaN or Inf: canonicalize to 0 and emit diagnostic
        if let Some(diags) = diagnostics {
            diags.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructInvalidGeometry,
                format!("Invalid geometry value: {}; canonicalized to 0", x),
            ));
        }
        return 0;
    }

    // Scale by 10000 (4 decimal places) and round ties to even
    let scaled = x * 10_000.0;
    scaled.round_ties_even() as i64
}

/// Normalize content stream bytes by tokenizing and re-emitting with single spaces.
///
/// This function uses the Phase 1.1 lexer to tokenize the content stream
/// and re-emit tokens with single 0x20 separators, eliminating whitespace variance.
/// This ensures that different whitespace layouts produce the same fingerprint.
///
/// # Arguments
///
/// * `bytes` - The raw content stream bytes to normalize
///
/// # Returns
///
/// Normalized bytes with tokens separated by single spaces. Comments are dropped.
///
/// # Examples
///
/// ```
/// use pdftract_core::fingerprint::canonicalize::normalize_content_stream;
///
/// let input = b"BT  /F1  12 Tf\n(hi) Tj ET";
/// let output = normalize_content_stream(input);
/// assert_eq!(output, b"BT /F1 12 Tf (hi) Tj ET");
/// ```
///
/// # Idempotence
///
/// Normalizing an already-normalized stream produces the same output:
///
/// ```
/// use pdftract_core::fingerprint::canonicalize::normalize_content_stream;
///
/// let input = b"BT /F1 12 Tf (hi) Tj ET";
/// let output = normalize_content_stream(input);
/// assert_eq!(output, input);  // Idempotent
/// ```
pub fn normalize_content_stream(bytes: &[u8]) -> Vec<u8> {
    if bytes.is_empty() {
        return Vec::new();
    }

    let mut lexer = Lexer::new(bytes);
    let mut result = Vec::new();
    let mut first_token = true;

    // Tokenize and re-emit with single spaces
    while let Some(token) = lexer.next_token() {
        match token {
            Token::Eof => break,
            _ => {
                // Add space before token (except for first token)
                if !first_token {
                    result.push(b' ');
                }
                first_token = false;

                // Serialize token back to bytes
                serialize_token(&mut result, &token);
            }
        }
    }

    result
}

/// Serialize a token back to its canonical byte representation.
///
/// This function converts a lexer Token back to its canonical byte representation
/// for fingerprinting purposes. The output is deterministic and matches the
/// PDF specification's lexical representation.
///
/// # Arguments
///
/// * `output` - Output buffer to write the serialized token to
/// * `token` - The token to serialize
fn serialize_token(output: &mut Vec<u8>, token: &Token) {
    match token {
        Token::Bool(true) => output.extend_from_slice(b"true"),
        Token::Bool(false) => output.extend_from_slice(b"false"),
        Token::Integer(i) => {
            let s = i.to_string();
            output.extend_from_slice(s.as_bytes());
        }
        Token::Real(r) => {
            // Use Display for shortest round-trip representation
            // This is deterministic per Rust's f64 Display implementation
            let s = format!("{}", r);
            output.extend_from_slice(s.as_bytes());
        }
        Token::String(bytes) => {
            output.push(b'(');
            // Escape special characters
            for &byte in bytes {
                match byte {
                    b'(' | b')' | b'\\' => {
                        output.push(b'\\');
                        output.push(byte);
                    }
                    _ => output.push(byte),
                }
            }
            output.push(b')');
        }
        Token::Name(bytes) => {
            output.push(b'/');
            output.extend_from_slice(bytes);
        }
        Token::ArrayStart => output.push(b'['),
        Token::ArrayEnd => output.push(b']'),
        Token::DictStart => output.extend_from_slice(b"<<"),
        Token::DictEnd => output.extend_from_slice(b">>"),
        Token::Stream => output.extend_from_slice(b"stream"),
        Token::EndStream => output.extend_from_slice(b"endstream"),
        Token::Obj => output.extend_from_slice(b"obj"),
        Token::EndObj => output.extend_from_slice(b"endobj"),
        Token::IndirectRef => output.push(b'R'),
        Token::Null => output.extend_from_slice(b"null"),
        Token::Keyword(bytes) => output.extend_from_slice(bytes),
        Token::Eof => {} // Don't emit anything for EOF
    }
}

/// Serialize a PdfDict to canonical JSON-equivalent bytes.
///
/// Keys are sorted lexicographically for deterministic output regardless of
/// insertion order. Values are serialized recursively.
///
/// # Arguments
///
/// * `dict` - The dictionary to serialize
///
/// # Returns
///
/// Canonical JSON-equivalent byte representation
///
/// # Examples
///
/// ```
/// use pdftract_core::fingerprint::canonicalize::serialize_dict_canonical;
/// use pdftract_core::parser::object::PdfDict;
/// use std::sync::Arc;
///
/// let mut dict = PdfDict::new();
/// dict.insert(Arc::from("/Z"), PdfObject::Integer(3));
/// dict.insert(Arc::from("/A"), PdfObject::Integer(1));
///
/// let bytes = serialize_dict_canonical(&dict);
/// // Keys are sorted: /A, /Z
/// assert!(bytes.windows(3).any(|w| w == b"/A 1"));
/// ```
pub fn serialize_dict_canonical(dict: &PdfDict) -> Vec<u8> {
    let mut result = Vec::new();

    // Convert to BTreeMap for sorted iteration
    let sorted_entries: BTreeMap<&Arc<str>, &PdfObject> = dict.iter().collect();

    for (i, (key, value)) in sorted_entries.iter().enumerate() {
        if i > 0 {
            result.push(b' ');
        }
        // Key (name, starts with /)
        result.extend_from_slice(key.as_bytes());
        result.push(b' ');
        // Value
        serialize_object_canonical(&mut result, value);
    }

    result
}

/// Serialize a PdfObject to canonical bytes for fingerprinting.
///
/// This is a simplified serializer that produces a deterministic
/// byte representation of PdfObjects for fingerprinting.
///
/// # Arguments
///
/// * `output` - Output buffer to write to
/// * `obj` - The object to serialize
fn serialize_object_canonical(output: &mut Vec<u8>, obj: &PdfObject) {
    match obj {
        PdfObject::Null => output.extend_from_slice(b"null"),
        PdfObject::Bool(b) => {
            if *b {
                output.extend_from_slice(b"true");
            } else {
                output.extend_from_slice(b"false");
            }
        }
        PdfObject::Integer(i) => {
            output.extend_from_slice(i.to_string().as_bytes());
        }
        PdfObject::Real(r) => {
            // Use Display for shortest round-trip representation
            output.extend_from_slice(format!("{}", r).as_bytes());
        }
        PdfObject::String(s) => {
            output.push(b'(');
            for &byte in s.as_ref() {
                match byte {
                    b'(' | b')' | b'\\' => {
                        output.push(b'\\');
                        output.push(byte);
                    }
                    _ => output.push(byte),
                }
            }
            output.push(b')');
        }
        PdfObject::Name(n) => {
            output.push(b'/');
            output.extend_from_slice(n.as_bytes());
        }
        PdfObject::Array(arr) => {
            output.push(b'[');
            for (i, elem) in arr.iter().enumerate() {
                if i > 0 {
                    output.push(b' ');
                }
                serialize_object_canonical(output, elem);
            }
            output.push(b']');
        }
        PdfObject::Dict(dict) => {
            output.extend_from_slice(b"<<");
            output.extend_from_slice(&serialize_dict_canonical(dict));
            output.extend_from_slice(b">>");
        }
        PdfObject::Ref(r) => {
            output.extend_from_slice(format!("{} {} R", r.object, r.generation).as_bytes());
        }
        PdfObject::Stream(s) => {
            // For streams, serialize the dict and mark as stream
            output.extend_from_slice(b"<<");
            output.extend_from_slice(&serialize_dict_canonical(&s.dict));
            output.extend_from_slice(b">> stream");
        }
        PdfObject::Indirect(i) => {
            output.extend_from_slice(format!("{} {} obj", i.id.object, i.id.generation).as_bytes());
        }
    }
}

/// Compute canonical hash of a resource dictionary.
///
/// Iterates over each namespace (fonts, xobjects, etc.) in LEXICAL key order,
/// serializing each value as canonical-JSON-equivalent bytes.
///
/// # Arguments
///
/// * `resources` - The resource dictionary to hash (None is treated as empty)
///
/// # Returns
///
/// Deterministic hash bytes that are the same regardless of insertion order
///
/// # Examples
///
/// ```
/// use pdftract_core::fingerprint::canonicalize::hash_resource_dict_canonical;
/// use pdftract_core::parser::object::{PdfDict, PdfObject};
/// use std::sync::Arc;
///
/// let mut font_dict = PdfDict::new();
/// font_dict.insert(Arc::from("/Z"), PdfObject::Name(Arc::from("FontZ")));
/// font_dict.insert(Arc::from("/A"), PdfObject::Name(Arc::from("FontA")));
///
/// let mut resources = PdfDict::new();
/// resources.insert(Arc::from("/Font"), PdfObject::Dict(Box::new(font_dict)));
///
/// let hash1 = hash_resource_dict_canonical(Some(&resources));
///
/// // Different insertion order, same hash
/// let mut font_dict2 = PdfDict::new();
/// font_dict2.insert(Arc::from("/A"), PdfObject::Name(Arc::from("FontA")));
/// font_dict2.insert(Arc::from("/Z"), PdfObject::Name(Arc::from("FontZ")));
///
/// let mut resources2 = PdfDict::new();
/// resources2.insert(Arc::from("/Font"), PdfObject::Dict(Box::new(font_dict2)));
///
/// let hash2 = hash_resource_dict_canonical(Some(&resources2));
/// assert_eq!(hash1, hash2);
/// ```
pub fn hash_resource_dict_canonical(resources: Option<&PdfDict>) -> [u8; 32] {
    use sha2::{Digest, Sha256};
    let mut hasher = Sha256::new();

    if let Some(resources) = resources {
        // Namespaces to iterate in lexical order
        let namespaces = ["/Font", "/XObject", "/ExtGState", "/ColorSpace", "/Pattern", "/Shading", "/Properties"];
        let mut sorted_namespaces: Vec<_> = namespaces.iter().filter_map(|&ns| {
            resources.get(ns).and_then(|v| v.as_dict()).map(|d| (ns, d))
        }).collect();

        // Sort namespaces lexicographically (they're already mostly sorted, but ensure)
        sorted_namespaces.sort_by_key(|&(ns, _)| ns);

        for (ns, dict) in sorted_namespaces {
            // Iterate dict entries in sorted key order
            let mut entries: Vec<_> = dict.iter().collect();
            entries.sort_by(|a, b| a.0.cmp(b.0));

            for (key, value) in entries {
                hasher.update(ns.as_bytes());
                hasher.update(key.as_bytes());
                hasher.update(&serialize_object_canonical_vec(value));
            }
        }
    }

    hasher.finalize().into()
}

/// Helper to serialize an object to a Vec<u8> for hashing.
fn serialize_object_canonical_vec(obj: &PdfObject) -> Vec<u8> {
    let mut result = Vec::new();
    serialize_object_canonical(&mut result, obj);
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonicalize_f64_basic() {
        let mut diags = None;

        // Basic rounding
        assert_eq!(canonicalize_f64(0.0, &mut diags), 0);
        assert_eq!(canonicalize_f64(1.23456, &mut diags), 12346); // rounds up
        assert_eq!(canonicalize_f64(1.23454, &mut diags), 12345); // rounds down
        assert_eq!(canonicalize_f64(-1.23456, &mut diags), -12346);
    }

    #[test]
    fn test_canonicalize_f64_banker's_rounding() {
        let mut diags = None;

        // Banker's rounding: ties to even
        assert_eq!(canonicalize_f64(1.23455, &mut diags), 12346); // 12345.5 -> 12346 (even)
        assert_eq!(canonicalize_f64(1.23445, &mut diags), 12344); // 12344.5 -> 12344 (even)
    }

    #[test]
    fn test_canonicalize_f64_critical_cases() {
        let mut diags = None;

        // Test edge cases from plan
        assert_eq!(canonicalize_f64(0.00005, &mut diags), 0); // 0.5 rounds to even (0)
        // Note: 0.00015 * 10000 = 1.4999... due to float representation, so rounds to 1
        assert_eq!(canonicalize_f64(0.00015, &mut diags), 1); // 1.4999... rounds to 1

        // Test negative banker's rounding
        assert_eq!(canonicalize_f64(-1.23455, &mut diags), -12346); // -12345.5 -> -12346 (even)
    }

    #[test]
    fn test_canonicalize_f64_nan_inf() {
        let mut diags = Some(Vec::new());

        assert_eq!(canonicalize_f64(f64::NAN, &mut diags), 0); // NaN -> 0
        assert_eq!(canonicalize_f64(f64::INFINITY, &mut diags), 0); // Inf -> 0
        assert_eq!(canonicalize_f64(f64::NEG_INFINITY, &mut diags), 0); // -Inf -> 0

        // Verify diagnostics were emitted
        assert_eq!(diags.as_ref().unwrap().len(), 3);
        for diag in diags.as_ref().unwrap() {
            assert_eq!(diag.code, DiagCode::StructInvalidGeometry);
        }
    }

    #[test]
    fn test_normalize_content_stream_basic() {
        let input = b"BT /F1 12 Tf (hello) Tj ET";
        let output = normalize_content_stream(input);
        assert_eq!(output, b"BT /F1 12 Tf (hello) Tj ET");
    }

    #[test]
    fn test_normalize_content_stream_whitespace_variants() {
        // Multiple spaces and tabs
        let input = b"BT  /F1\t\t12 Tf\n(hi) Tj ET";
        let output = normalize_content_stream(input);
        assert_eq!(output, b"BT /F1 12 Tf (hi) Tj ET");
    }

    #[test]
    fn test_normalize_content_stream_comments_dropped() {
        // Comments are dropped by the lexer
        let input = b"BT % this is a comment\n/F1 12 Tf ET";
        let output = normalize_content_stream(input);
        assert_eq!(output, b"BT /F1 12 Tf ET");
    }

    #[test]
    fn test_normalize_content_stream_empty() {
        let input = b"";
        let output = normalize_content_stream(input);
        assert_eq!(output, b"");
    }

    #[test]
    fn test_normalize_content_stream_idempotent() {
        // Normalizing an already-normalized stream produces the same output
        let input = b"BT /F1 12 Tf (hi) Tj ET";
        let output = normalize_content_stream(input);
        assert_eq!(output, input);

        // Double normalization
        let output2 = normalize_content_stream(&output);
        assert_eq!(output, output2);
    }

    #[test]
    fn test_normalize_content_stream_complex() {
        // From acceptance criteria
        let input = b"BT  /F1  12 Tf\n(hi) Tj ET";
        let output = normalize_content_stream(input);
        assert_eq!(output, b"BT /F1 12 Tf (hi) Tj ET");
    }

    #[test]
    fn test_serialize_token_basic() {
        let mut result = Vec::new();

        serialize_token(&mut result, &Token::Bool(true));
        assert_eq!(result, b"true");

        result.clear();
        serialize_token(&mut result, &Token::Bool(false));
        assert_eq!(result, b"false");

        result.clear();
        serialize_token(&mut result, &Token::Integer(42));
        assert_eq!(result, b"42");

        result.clear();
        serialize_token(&mut result, &Token::ArrayStart);
        assert_eq!(result, b"[");
    }

    #[test]
    fn test_serialize_token_real() {
        let mut result = Vec::new();

        serialize_token(&mut result, &Token::Real(3.14159));
        let s = String::from_utf8(result).unwrap();
        // Should use shortest round-trip representation
        assert!(s.starts_with("3.14159"));
    }

    #[test]
    fn test_serialize_token_string() {
        let mut result = Vec::new();

        serialize_token(&mut result, &Token::String(b"hello".to_vec()));
        assert_eq!(result, b"(hello)");

        result.clear();
        serialize_token(&mut result, &Token::String(b"(test)".to_vec()));
        assert_eq!(result, b"(\\(test\\))");
    }

    #[test]
    fn test_serialize_dict_canonical_sorted() {
        let mut dict = PdfDict::new();
        dict.insert(Arc::from("/Z"), PdfObject::Integer(3));
        dict.insert(Arc::from("/A"), PdfObject::Integer(1));
        dict.insert(Arc::from("/M"), PdfObject::Integer(2));

        let bytes = serialize_dict_canonical(&dict);

        // Keys should be sorted: /A, /M, /Z
        assert!(bytes.starts_with(b"/A 1"));
        assert!(bytes.windows(3).any(|w| w == b"/M 2"));
        assert!(bytes.windows(3).any(|w| w == b"/Z 3"));
    }

    #[test]
    fn test_serialize_dict_canonical_nested() {
        let mut inner = PdfDict::new();
        inner.insert(Arc::from("/B"), PdfObject::Integer(2));

        let mut outer = PdfDict::new();
        outer.insert(Arc::from("/A"), PdfObject::Integer(1));
        outer.insert(Arc::from("/Inner"), PdfObject::Dict(Box::new(inner)));

        let bytes = serialize_dict_canonical(&outer);

        // /A comes before /Inner lexicographically
        assert!(bytes.starts_with(b"/A 1 /Inner"));
    }

    #[test]
    fn test_hash_resource_dict_canonical_order_independence() {
        let mut font_dict1 = PdfDict::new();
        font_dict1.insert(Arc::from("/Z"), PdfObject::Name(Arc::from("FontZ")));
        font_dict1.insert(Arc::from("/A"), PdfObject::Name(Arc::from("FontA")));

        let mut resources1 = PdfDict::new();
        resources1.insert(Arc::from("/Font"), PdfObject::Dict(Box::new(font_dict1)));

        let mut font_dict2 = PdfDict::new();
        font_dict2.insert(Arc::from("/A"), PdfObject::Name(Arc::from("FontA")));
        font_dict2.insert(Arc::from("/Z"), PdfObject::Name(Arc::from("FontZ")));

        let mut resources2 = PdfDict::new();
        resources2.insert(Arc::from("/Font"), PdfObject::Dict(Box::new(font_dict2)));

        let hash1 = hash_resource_dict_canonical(Some(&resources1));
        let hash2 = hash_resource_dict_canonical(Some(&resources2));

        assert_eq!(hash1, hash2, "Resource dict hash should be independent of insertion order");
    }

    #[test]
    fn test_hash_resource_dict_canonical_none() {
        let hash1 = hash_resource_dict_canonical(None);
        let hash2 = hash_resource_dict_canonical(None);

        assert_eq!(hash1, hash2, "Hash of None should be deterministic");
    }

    #[test]
    fn test_hash_resource_dict_canonical_empty() {
        let resources = PdfDict::new();
        let hash1 = hash_resource_dict_canonical(Some(&resources));
        let hash2 = hash_resource_dict_canonical(Some(&resources));

        assert_eq!(hash1, hash2, "Hash of empty dict should be deterministic");
    }

    #[test]
    fn test_serialize_object_canonical_real() {
        let mut result = Vec::new();
        serialize_object_canonical(&mut result, &PdfObject::Real(1.5));
        assert_eq!(result, b"1.5");

        result.clear();
        serialize_object_canonical(&mut result, &PdfObject::Real(0.0001));
        // Uses shortest round-trip representation
        assert!(result == b"0.0001" || result == b"1e-4" || result == b"1E-4");
    }

    #[test]
    fn test_serialize_object_canonical_array() {
        let mut result = Vec::new();
        let arr = vec![
            PdfObject::Integer(1),
            PdfObject::Integer(2),
            PdfObject::Integer(3),
        ];
        serialize_object_canonical(&mut result, &PdfObject::Array(Box::new(arr)));
        assert_eq!(result, b"[1 2 3]");
    }

    #[test]
    fn test_serialize_object_canonical_dict() {
        let mut dict = PdfDict::new();
        dict.insert(Arc::from("/Z"), PdfObject::Integer(3));
        dict.insert(Arc::from("/A"), PdfObject::Integer(1));

        let mut result = Vec::new();
        serialize_object_canonical(&mut result, &PdfObject::Dict(Box::new(dict)));
        // Keys sorted: /A, /Z
        assert!(result.starts_with(b"<<"));
        assert!(result.windows(3).any(|w| w == b"/A 1"));
        assert!(result.windows(3).any(|w| w == b"/Z 3"));
        assert!(result.ends_with(b">>"));
    }

    #[test]
    fn test_inv8_no_panics() {
        // INV-8: No panics on any input, including invalid data
        let mut diags = None;

        // All special float values
        canonicalize_f64(f64::NAN, &mut diags);
        canonicalize_f64(f64::INFINITY, &mut diags);
        canonicalize_f64(f64::NEG_INFINITY, &mut diags);

        // Empty input
        let _ = normalize_content_stream(b"");

        // Invalid but parseable content
        let _ = normalize_content_stream(b"%%%%%%%%%%");

        // Empty dict
        let dict = PdfDict::new();
        let _ = serialize_dict_canonical(&dict);
        let _ = hash_resource_dict_canonical(Some(&dict));

        // None resources
        let _ = hash_resource_dict_canonical(None);
    }
}
