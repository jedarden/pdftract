//! XFA (XML Forms Architecture) stream parser.
//!
//! This module implements Phase 7.4.3: XFA stream parsing. It extracts form
//! field values from XFA XML streams, which are commonly found in government
//! and enterprise forms (tax forms, healthcare intake, etc.).
//!
//! XFA streams come in two layouts:
//! 1. **Single stream**: A complete XDP (XML Data Package) document
//! 2. **Array of streams**: Multiple named streams concatenated in order
//!
//! ## Architecture
//!
//! - **Stream extraction**: Read `/AcroForm /XFA` (stream or array)
//! - **XML parsing**: Use quick-xml to parse the XDP structure
//! - **Field extraction**: Walk the XFA data model to extract `<field>` values
//! - **Namespace handling**: XFA uses multiple namespaces (xfa, xdc, xdp, xfdf)

use crate::diagnostics::{DiagCode, Diagnostic};
use crate::parser::object::{PdfDict, PdfObject};
use crate::parser::stream::{decode_stream, ExtractionOptions, PdfSource};
use crate::parser::xref::XrefResolver;
use std::collections::HashMap;

/// Result type for XFA operations.
pub type Result<T> = std::result::Result<T, Vec<Diagnostic>>;

/// XFA field with full name and value.
///
/// Represents a single field extracted from the XFA data model.
#[derive(Debug, Clone, PartialEq)]
pub struct XfaField {
    /// Full field name (dot-separated path, e.g., "form1.section1.firstName")
    pub full_name: String,
    /// Field value (text content of the field element)
    pub value: Option<String>,
}

/// Extract XFA field values from the `/AcroForm /XFA` entry.
///
/// This is the main entry point for Phase 7.4.3. It handles both single-stream
/// and array-stream layouts, decodes compressed streams, parses the XML,
/// and walks the XFA data model to extract field values.
///
/// # Arguments
///
/// * `resolver` - Xref resolver for dereferencing indirect objects
/// * `acroform_dict` - The AcroForm dictionary containing the /XFA entry
/// * `source` - PDF data source for reading stream contents
/// * `opts` - Extraction options
///
/// # Returns
///
/// A `Vec<XfaField>` containing all discovered fields with their values.
/// Returns empty vec if the PDF has no XFA or if XFA parsing fails.
///
/// # Behavior
///
/// - If `/XFA` is absent, returns empty vec (not an error)
/// - If `/XFA` is a stream, decodes and parses it directly
/// - If `/XFA` is an array, concatenates named streams in array order
/// - Handles FlateDecode-compressed streams via Phase 1 stream decoder
/// - Malformed XML emits diagnostics and returns partial results
/// - Missing named streams in the array form are skipped (not an error)
///
/// # Example
///
/// ```ignore
/// use pdftract_core::forms::xfa::extract_xfa_fields;
///
/// let fields = extract_xfa_fields(&resolver, &acroform_dict, &source, &opts);
/// for field in fields {
///     println!("Field: {} = {:?}", field.full_name, field.value);
/// }
/// ```
pub fn extract_xfa_fields(
    resolver: &XrefResolver,
    acroform_dict: &PdfDict,
    source: &dyn PdfSource,
    opts: &ExtractionOptions,
) -> Vec<XfaField> {
    let mut diagnostics = Vec::new();
    let mut decompress_counter = 0u64;

    // Get the /XFA entry
    let xfa_obj = match acroform_dict.get("XFA") {
        Some(obj) => obj,
        None => return Vec::new(), // No XFA present
    };

    // Extract and decode the XFA XML bytes
    let xml_bytes = match extract_xfa_bytes(
        resolver,
        xfa_obj,
        source,
        opts,
        &mut decompress_counter,
        &mut diagnostics,
    ) {
        Some(bytes) => bytes,
        None => return Vec::new(),
    };

    // Parse the XML and extract fields
    parse_xfa_xml(&xml_bytes, &mut diagnostics)
}

/// Extract and decode XFA XML bytes from the /XFA entry.
///
/// Handles both single-stream and array-stream layouts.
fn extract_xfa_bytes(
    resolver: &XrefResolver,
    xfa_obj: &PdfObject,
    source: &dyn PdfSource,
    opts: &ExtractionOptions,
    decompress_counter: &mut u64,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<u8>> {
    match xfa_obj {
        // Single stream: this is the full XDP
        PdfObject::Stream(stream) => Some(decode_stream_bytes(
            stream,
            source,
            opts,
            decompress_counter,
            diagnostics,
        )),
        // Array: alternating (Name, Stream) pairs
        PdfObject::Array(arr) => extract_xfa_bytes_from_array(
            resolver,
            arr,
            source,
            opts,
            decompress_counter,
            diagnostics,
        ),
        // Indirect reference: resolve and try again
        PdfObject::Ref(ref_) => {
            let resolved = resolver.resolve(*ref_).ok()?;
            extract_xfa_bytes(
                resolver,
                &resolved,
                source,
                opts,
                decompress_counter,
                diagnostics,
            )
        }
        // Invalid type
        _ => {
            diagnostics.push(Diagnostic::with_dynamic_no_offset(
                DiagCode::StructUnexpectedEof,
                format!(
                    "Invalid /XFA type: expected stream or array, got {}",
                    xfa_obj.type_name()
                ),
            ));
            None
        }
    }
}

/// Extract XFA bytes from an array of (Name, Stream) pairs.
///
/// The array contains alternating Name and Stream entries. We concatenate
/// the stream contents in array order to form the complete XDP.
fn extract_xfa_bytes_from_array(
    resolver: &XrefResolver,
    arr: &[PdfObject],
    source: &dyn PdfSource,
    opts: &ExtractionOptions,
    decompress_counter: &mut u64,
    diagnostics: &mut Vec<Diagnostic>,
) -> Option<Vec<u8>> {
    let mut xdp_bytes = Vec::new();

    // Known XFA stream names (per XFA spec 3.3)
    // These are the standard names in the array form
    let _known_names = [
        "preamble",
        "config",
        "template",
        "datasets",
        "form",
        "postamble",
    ];

    let mut chunks = Vec::new();

    // Process pairs: (Name, Stream)
    for chunk in arr.chunks(2) {
        if chunk.len() < 2 {
            break;
        }

        let name_obj = &chunk[0];
        let stream_obj = &chunk[1];

        // Get the stream name (for validation)
        let _name = name_obj.as_name().map(|n| n.to_string());

        // Resolve the stream
        let stream_ref = match stream_obj {
            PdfObject::Ref(ref_) => *ref_,
            PdfObject::Stream(_) => {
                // Inline stream - use directly
                let stream = stream_obj.as_stream()?;
                let bytes =
                    decode_stream_bytes(stream, source, opts, decompress_counter, diagnostics);
                let name_str = name_obj
                    .as_name()
                    .map(|n| n.to_string())
                    .unwrap_or_else(|| format!("stream_{}", chunks.len()));
                chunks.push((name_str, bytes));
                continue;
            }
            _ => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!(
                        "XFA array entry must be Name/Stream pair, got {}/{}",
                        name_obj.type_name(),
                        stream_obj.type_name()
                    ),
                ));
                continue;
            }
        };

        let resolved = match resolver.resolve(stream_ref) {
            Ok(obj) => obj,
            Err(_) => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!("Failed to resolve XFA stream reference {}", stream_ref),
                ));
                continue;
            }
        };

        let stream = match resolved.as_stream() {
            Some(s) => s,
            None => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!(
                        "XFA array entry is not a stream (type: {})",
                        resolved.type_name()
                    ),
                ));
                continue;
            }
        };

        let bytes = decode_stream_bytes(stream, source, opts, decompress_counter, diagnostics);
        let name_str = name_obj
            .as_name()
            .map(|n| n.to_string())
            .unwrap_or_else(|| format!("stream_{}", chunks.len()));
        chunks.push((name_str, bytes));
    }

    // Concatenate chunks in order
    // The array order defines the XDP structure
    for (_name, bytes) in &chunks {
        xdp_bytes.extend_from_slice(bytes);
    }

    if xdp_bytes.is_empty() {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::StructUnexpectedEof,
            "XFA array produced no data".to_string(),
        ));
        None
    } else {
        Some(xdp_bytes)
    }
}

/// Decode a PDF stream to bytes, applying filters.
///
/// Uses the Phase 1 stream decoder to handle FlateDecode and other filters.
fn decode_stream_bytes(
    stream: &crate::parser::object::PdfStream,
    source: &dyn PdfSource,
    opts: &ExtractionOptions,
    decompress_counter: &mut u64,
    diagnostics: &mut Vec<Diagnostic>,
) -> Vec<u8> {
    let bytes = decode_stream(stream, source, opts, decompress_counter);
    // Note: decode_stream returns Vec<u8> directly (not a Result)
    // If it fails, it returns empty Vec
    if bytes.is_empty() && stream.len_hint.is_some() {
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::StructUnexpectedEof,
            "Failed to decode XFA stream (returned empty bytes)".to_string(),
        ));
    }
    bytes
}

/// Parse XFA XML and extract field values.
///
/// Uses quick-xml to parse the XDP structure and walk the XFA data model.
/// Field values are extracted from the `<xfa:datasets>` section.
#[allow(dead_code, unused_variables)]
fn parse_xfa_xml(xml_bytes: &[u8], diagnostics: &mut Vec<Diagnostic>) -> Vec<XfaField> {
    // Quick-xml is optional, gated behind the `ocr` feature
    // If it's not available, return empty vec
    #[cfg(feature = "ocr")]
    {
        use quick_xml::events::Event;
        use quick_xml::Reader;

        let mut fields = Vec::new();
        let mut xml = match Reader::from_reader(xml_bytes) {
            Ok(r) => r,
            Err(e) => {
                diagnostics.push(Diagnostic::with_dynamic_no_offset(
                    DiagCode::StructUnexpectedEof,
                    format!("Failed to create XML reader: {}", e),
                ));
                return fields;
            }
        };

        // Configure the reader
        xml.check_end_names(false).trim_markup(false);

        // Track namespace prefixes
        let mut ns_map = HashMap::new();
        let mut current_path = Vec::new();
        let mut in_datasets = false;
        let mut in_data = false;
        let mut capture_text = false;
        let mut current_value = String::new();

        let mut buf = Vec::new();

        loop {
            match xml.read_event_into(&mut buf) {
                Ok(Event::Start(ref e)) => {
                    // Register namespace bindings
                    for attr_result in e.attributes() {
                        if let Ok(attr) = attr_result {
                            let key = attr.key.into_owned();
                            if key.starts_with(b"xmlns:") || key == b"xmlns" {
                                let prefix = if key == b"xmlns" {
                                    b"default".to_vec()
                                } else {
                                    key[6..].to_vec() // Skip "xmlns:"
                                };
                                ns_map.insert(prefix, attr.value.into_owned());
                            }
                        }
                    }

                    let name = String::from_utf8_lossy(e.name()).to_string();

                    // Track path
                    current_path.push(name.clone());

                    // Check for xfa:datasets and xfa:data
                    if is_xfa_element(&name, &ns_map, "datasets") {
                        in_datasets = true;
                    } else if is_xfa_element(&name, &ns_map, "data") {
                        in_data = true;
                    } else if in_datasets && in_data {
                        // We're in the data section, capture text content of any element
                        capture_text = true;
                        current_value.clear();
                    }
                }
                Ok(Event::End(ref e)) => {
                    let name = String::from_utf8_lossy(e.name()).to_string();

                    if capture_text && is_xfa_element(&name, &ns_map, "data") {
                        in_data = false;
                    } else if is_xfa_element(&name, &ns_map, "datasets") {
                        in_datasets = false;
                    } else if capture_text {
                        // Emit the field
                        let full_name = current_path.join(".");
                        let value = if current_value.is_empty() {
                            None
                        } else {
                            Some(current_value.trim().to_string())
                        };

                        fields.push(XfaField { full_name, value });

                        capture_text = false;
                        current_value.clear();
                    }

                    current_path.pop();
                }
                Ok(Event::Text(ref e)) => {
                    if capture_text {
                        current_value
                            .push_str(&e.unescape().unwrap_or_else(|_| current_value.clone()));
                    }
                }
                Ok(Event::CData(ref e)) => {
                    if capture_text {
                        current_value.push_str(&String::from_utf8_lossy(e));
                    }
                }
                Ok(Event::Eof) => break,
                Err(e) => {
                    diagnostics.push(Diagnostic::with_dynamic_no_offset(
                        DiagCode::StructUnexpectedEof,
                        format!("XML parsing error: {}", e),
                    ));
                    break;
                }
                _ => {}
            }

            buf.clear();
        }

        fields
    }

    #[cfg(not(feature = "ocr"))]
    {
        // Suppress unused variable warning
        let _ = diagnostics;
        diagnostics.push(Diagnostic::with_dynamic_no_offset(
            DiagCode::StructUnexpectedEof,
            "XFA parsing requires the 'ocr' feature (quick-xml)".to_string(),
        ));
        Vec::new()
    }
}

/// Check if an element name matches an XFA element.
///
/// Handles namespace prefixes by checking against registered namespaces.
#[allow(dead_code)]
fn is_xfa_element(name: &str, ns_map: &HashMap<Vec<u8>, Vec<u8>>, local_name: &str) -> bool {
    // Check for unprefixed name
    if name == local_name {
        return true;
    }

    // Check for namespaced variants (xfa:, xdp:, etc.)
    if let Some((prefix, local)) = name.split_once(':') {
        if local == local_name {
            // Check if the prefix is registered as an XFA namespace
            if let Some(ns_uri) = ns_map.get(prefix.as_bytes()) {
                let ns_uri_str = String::from_utf8_lossy(ns_uri);
                // XFA namespace URI pattern
                return ns_uri_str.contains("adobe.com/2003/xmlfxa")
                    || ns_uri_str.contains("adobe.com/2006/xfa");
            }
        }
    }

    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parser::object::{intern, ObjRef};
    use crate::parser::stream::MemorySource;
    use crate::parser::xref::XrefResolver;
    use indexmap::IndexMap;

    /// Helper to create a minimal XFA test setup.
    #[allow(dead_code)]
    fn make_test_xfa_setup(xml_content: &[u8]) -> (XrefResolver, PdfDict, MemorySource) {
        let resolver = XrefResolver::new();
        let source = MemorySource::new(xml_content.to_vec());

        let mut stream_dict = IndexMap::new();
        stream_dict.insert(
            intern("Length"),
            PdfObject::Integer(xml_content.len() as i64),
        );

        let stream = crate::parser::object::PdfStream::new(
            stream_dict,
            0, // offset - data starts at beginning of source
            Some(xml_content.len() as u64),
        );

        let stream_ref = ObjRef::new(100, 0);
        resolver.cache_object(stream_ref, PdfObject::Stream(Box::new(stream)));

        // Create AcroForm dict with XFA
        let mut acroform_dict = IndexMap::new();
        acroform_dict.insert(intern("XFA"), PdfObject::Ref(stream_ref));

        (resolver, acroform_dict, source)
    }

    #[test]
    #[cfg(feature = "ocr")]
    fn test_parse_xfa_xml_simple_fields() {
        let xml = br#"<?xml version="1.0"?>
<xfa:datasets xmlns:xfa="http://www.adobe.com/2003/xmlfxa">
    <xfa:data>
        <firstName>John</firstName>
        <lastName>Doe</lastName>
        <email>john.doe@example.com</email>
    </xfa:data>
</xfa:datasets>"#;

        let fields = parse_xfa_xml(xml, &mut Vec::new());

        assert_eq!(fields.len(), 3);

        let first = fields
            .iter()
            .find(|f| f.full_name.contains("firstName"))
            .unwrap();
        assert_eq!(first.value, Some("John".to_string()));

        let last = fields
            .iter()
            .find(|f| f.full_name.contains("lastName"))
            .unwrap();
        assert_eq!(last.value, Some("Doe".to_string()));

        let email = fields
            .iter()
            .find(|f| f.full_name.contains("email"))
            .unwrap();
        assert_eq!(email.value, Some("john.doe@example.com".to_string()));
    }

    #[test]
    #[cfg(feature = "ocr")]
    fn test_parse_xfa_xml_nested_fields() {
        let xml = br#"<?xml version="1.0"?>
<xfa:datasets xmlns:xfa="http://www.adobe.com/2003/xmlfxa">
    <xfa:data>
        <employee>
            <name>
                <first>Jane</first>
                <last>Smith</last>
            </name>
            <department>Engineering</department>
        </employee>
    </xfa:data>
</xfa:datasets>"#;

        let fields = parse_xfa_xml(xml, &mut Vec::new());

        // Should capture all elements with their full paths
        assert!(fields.len() >= 4);

        let first = fields
            .iter()
            .find(|f| f.full_name.contains("first"))
            .unwrap();
        assert_eq!(first.value, Some("Jane".to_string()));

        let dept = fields
            .iter()
            .find(|f| f.full_name.contains("department"))
            .unwrap();
        assert_eq!(dept.value, Some("Engineering".to_string()));
    }

    #[test]
    #[cfg(feature = "ocr")]
    fn test_parse_xfa_xml_empty_fields() {
        let xml = br#"<?xml version="1.0"?>
<xfa:datasets xmlns:xfa="http://www.adobe.com/2003/xmlfxa">
    <xfa:data>
        <field1></field1>
        <field2>value</field2>
        <field3/>
    </xfa:data>
</xfa:datasets>"#;

        let fields = parse_xfa_xml(xml, &mut Vec::new());

        // Empty fields should have None value
        let field1 = fields
            .iter()
            .find(|f| f.full_name.contains("field1"))
            .unwrap();
        assert_eq!(field1.value, None);

        let field3 = fields
            .iter()
            .find(|f| f.full_name.contains("field3"))
            .unwrap();
        assert_eq!(field3.value, None);
    }

    #[test]
    #[cfg(feature = "ocr")]
    fn test_parse_xfa_xml_malformed() {
        let xml = b"<?xml version=\"1.0\"?>\n<broken>";

        let mut diagnostics = Vec::new();
        let fields = parse_xfa_xml(xml, &mut diagnostics);

        // Should return empty vec and emit diagnostic
        assert!(fields.is_empty() || fields.len() < 2);
        assert!(!diagnostics.is_empty());
    }

    #[test]
    #[cfg(feature = "ocr")]
    fn test_extract_xfa_fields_single_stream() {
        let xml = br#"<?xml version="1.0"?>
<xfa:datasets xmlns:xfa="http://www.adobe.com/2003/xmlfxa">
    <xfa:data>
        <testField>testValue</testField>
    </xfa:data>
</xfa:datasets>"#;

        let (resolver, acroform_dict, source) = make_test_xfa_setup(xml);
        let opts = crate::parser::stream::ExtractionOptions::default();

        let fields = extract_xfa_fields(&resolver, &acroform_dict, &source, &opts);

        assert_eq!(fields.len(), 1);
        assert_eq!(fields[0].value, Some("testValue".to_string()));
    }

    #[test]
    fn test_extract_xfa_fields_no_xfa() {
        let resolver = XrefResolver::new();
        let source = MemorySource::new(vec![]);
        let acroform_dict = IndexMap::new();
        let opts = crate::parser::stream::ExtractionOptions::default();

        let fields = extract_xfa_fields(&resolver, &acroform_dict, &source, &opts);

        assert!(fields.is_empty());
    }

    #[test]
    fn test_is_xfa_element() {
        let mut ns_map = HashMap::new();
        ns_map.insert(
            b"xfa".to_vec(),
            b"http://www.adobe.com/2003/xmlfxa".to_vec(),
        );

        // Unprefixed name
        assert!(is_xfa_element("datasets", &ns_map, "datasets"));

        // Prefixed name with correct namespace
        assert!(is_xfa_element("xfa:datasets", &ns_map, "datasets"));

        // Wrong local name
        assert!(!is_xfa_element("xfa:datasets", &ns_map, "data"));

        // Unknown prefix
        assert!(!is_xfa_element("foo:datasets", &ns_map, "datasets"));
    }
}
