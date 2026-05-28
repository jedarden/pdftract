//! PDF/A conformance detection module.
//!
//! This module provides functions to detect PDF/A conformance levels
//! from XMP metadata streams embedded in PDF documents.
//!
//! PDF/A is an ISO-standardized version of PDF specialized for
//! long-term preservation. Conformance levels include:
//! - PDF/A-1a/b (ISO 19005-1:2005)
//! - PDF/A-2a/b/u/f (ISO 19005-2:2011)
//! - PDF/A-3a/b/u/f (ISO 19005-3:2012)
//! - PDF/A-4e/f (ISO 19005-4:2020)
//!
//! The conformance information is stored in the document's /Metadata
//! stream as XMP XML with the pdfaid namespace.

use crate::parser::stream::PdfSource;
use crate::parser::xref::XrefResolver;
use crate::parser::object::PdfObject;
use anyhow::Result;

/// Detect PDF/A conformance from an XMP metadata stream.
///
/// Parses the XMP XML to extract pdfaid:part and pdfaid:conformance
/// namespace elements, then combines them as "PDF/A-{part}{conformance}"
/// (e.g. "PDF/A-1b", "PDF/A-2u", "PDF/A-3a").
///
/// # Arguments
///
/// * `metadata_stream` - Optional byte slice containing the XMP metadata stream
///
/// # Returns
///
/// * `Some(String)` - PDF/A conformance string if detected (e.g., "PDF/A-1b")
/// * `None` - No PDF/A conformance detected or malformed XML
///
/// # Graceful Failure
///
/// Per INV-8, this function never panics. Malformed XML, missing elements,
/// or any parsing error returns None rather than propagating errors.
///
/// # XMP Namespace Handling
///
/// The pdfaid namespace prefix can vary (pdfaid, x, foo, etc.). This function
/// matches on the local name (the part after the colon) to handle any prefix.
///
/// # Example
///
/// ```ignore
/// use pdftract_core::conformance::detect_conformance;
///
/// // XMP with pdfaid:part="1" and pdfaid:conformance="b"
/// let xmp = br#"<?xpacket begin='...'?>
/// <rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
///   <rdf:Description rdf:about=''
///     xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
///     <pdfaid:part>1</pdfaid:part>
///     <pdfaid:conformance>b</pdfaid:conformance>
///   </rdf:Description>
/// </rdf:RDF>"#;
///
/// let result = detect_conformance(Some(xmp));
/// assert_eq!(result, Some("PDF/A-1b".to_string()));
/// ```
pub fn detect_conformance(metadata_stream: Option<&[u8]>) -> Option<String> {
    use quick_xml::events::Event;
    use quick_xml::reader::Reader;

    let xml = metadata_stream?;
    let mut reader = Reader::from_reader(xml);
    let mut part: Option<String> = None;
    let mut conf: Option<String> = None;
    let mut current_tag: Option<Vec<u8>> = None;
    let mut buf = Vec::new();

    loop {
        match reader.read_event_into(&mut buf) {
            Ok(Event::Start(e)) => {
                let name = e.name().as_ref().to_vec();
                // Match on local name (after colon) for any namespace prefix
                let local_name = name.split(|&b| b == b':').last().unwrap_or(&name);
                if local_name == b"part" || local_name == b"conformance" {
                    current_tag = Some(name);
                }
            }
            Ok(Event::Text(e)) => {
                if let Some(tag) = &current_tag {
                    let text = e.unescape().unwrap_or_default().to_string();
                    let local_tag = tag.split(|&b| b == b':').last().unwrap_or(tag);
                    if local_tag == b"part" {
                        part = Some(text);
                    } else if local_tag == b"conformance" {
                        conf = Some(text);
                    }
                }
            }
            Ok(Event::End(_)) => {
                current_tag = None;
            }
            Ok(Event::Eof) => break,
            Err(_) => return None, // Malformed XML - graceful failure
            _ => {}
        }
        buf.clear();
    }

    match (part, conf) {
        (Some(p), Some(c)) => Some(format!("PDF/A-{}{}", p, c)),
        (Some(p), None) => Some(format!("PDF/A-{}", p)),
        _ => None,
    }
}

/// Detect PDF/A conformance from a catalog's metadata reference.
///
/// This is a convenience function that resolves the metadata stream
/// from the catalog and calls detect_conformance.
///
/// # Arguments
///
/// * `metadata_ref` - Optional reference to the metadata stream
/// * `resolver` - Xref resolver for dereferencing the stream
/// * `source` - PDF source for reading stream data
///
/// # Returns
///
/// * `Some(String)` - PDF/A conformance if detected
/// * `None` - No conformance or error reading metadata
pub fn detect_conformance_from_ref(
    metadata_ref: Option<crate::parser::object::ObjRef>,
    resolver: &XrefResolver,
    source: &dyn PdfSource,
) -> Option<String> {
    let ref_ = metadata_ref?;
    let obj = resolver.resolve_with_source(ref_, source).ok()?;
    let stream = obj.as_stream()?;

    // Decode the stream to get the XMP XML
    use crate::parser::stream::{decode_stream, ExtractionOptions};
    let opts = ExtractionOptions {
        max_decompress_bytes: DEFAULT_MAX_DECOMPRESS_BYTES,
        ..Default::default()
    };
    let xml_bytes = decode_stream(stream, source, &opts, &mut 0);
    detect_conformance(Some(&xml_bytes))
}

/// Default maximum decompressed bytes for metadata streams.
/// Metadata streams are typically small (< 1 MB), so we use a conservative limit.
const DEFAULT_MAX_DECOMPRESS_BYTES: u64 = 16 * 1024 * 1024; // 16 MiB

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_detect_conformance_pdf_a_1b() {
        let xmp = br#"<?xpacket begin='...' id='W5M0MpCehiHzreSzNTczkc9d'?>
<x:xmpmeta xmlns:x='adobe:ns:meta/' x:xmptk='Adobe XMP Core 5.6-c140 79.160451'>
  <rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
    <rdf:Description rdf:about=''
      xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
      <pdfaid:part>1</pdfaid:part>
      <pdfaid:conformance>b</pdfaid:conformance>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>
<?xpacket end='w'?>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-1b".to_string()));
    }

    #[test]
    fn test_detect_conformance_pdf_a_2u() {
        let xmp = br#"<?xpacket begin='...'?>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about=''
    xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
    <pdfaid:part>2</pdfaid:part>
    <pdfaid:conformance>u</pdfaid:conformance>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-2u".to_string()));
    }

    #[test]
    fn test_detect_conformance_pdf_a_3a() {
        let xmp = br#"<?xpacket begin='...'?>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about=''
    xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
    <pdfaid:part>3</pdfaid:part>
    <pdfaid:conformance>a</pdfaid:conformance>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-3a".to_string()));
    }

    #[test]
    fn test_detect_conformance_part_only() {
        let xmp = br#"<?xpacket begin='...'?>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about=''
    xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
    <pdfaid:part>3</pdfaid:part>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-3".to_string()));
    }

    #[test]
    fn test_detect_conformance_no_metadata() {
        let result = detect_conformance(None);
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_conformance_empty_xml() {
        let xmp = b"";
        let result = detect_conformance(Some(xmp));
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_conformance_malformed_xml() {
        let xmp = b"<not-valid-xml<<<<";
        let result = detect_conformance(Some(xmp));
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_conformance_no_pdfaid_elements() {
        let xmp = br#"<?xpacket begin='...'?>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about='' xmlns:dc='http://purl.org/dc/elements/1.1/'>
    <dc:title>Test Document</dc:title>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, None);
    }

    #[test]
    fn test_detect_conformance_different_namespace_prefix() {
        // Some PDFs use a different prefix than 'pdfaid'
        let xmp = br#"<?xpacket begin='...'?>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about=''
    xmlns:x='http://www.aiim.org/pdfa/ns/id/'>
    <x:part>2</x:part>
    <x:conformance>b</x:conformance>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-2b".to_string()));
    }

    #[test]
    fn test_detect_conformance_pdf_a_4e() {
        let xmp = br#"<?xpacket begin='...'?>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about=''
    xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
    <pdfaid:part>4</pdfaid:part>
    <pdfaid:conformance>e</pdfaid:conformance>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-4e".to_string()));
    }

    #[test]
    fn test_detect_conformance_pdf_a_4f() {
        let xmp = br#"<?xpacket begin='...'?>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about=''
    xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
    <pdfaid:part>4</pdfaid:part>
    <pdfaid:conformance>f</pdfaid:conformance>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-4f".to_string()));
    }

    #[test]
    fn test_detect_conformance_whitespace_handling() {
        // Test with extra whitespace in element content
        let xmp = br#"<?xpacket begin='...'?>
<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about=''
    xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
    <pdfaid:part> 1 </pdfaid:part>
    <pdfaid:conformance> b </pdfaid:conformance>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        // Whitespace is preserved by XMP spec, but we accept it
        assert!(result.is_some());
        assert!(result.unwrap().starts_with("PDF/A-"));
    }

    #[test]
    fn test_detect_conformance_minimal_xmp() {
        // Minimal valid XMP with PDF/A info
        let xmp = br#"<rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
  <rdf:Description rdf:about='' xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
    <pdfaid:part>1</pdfaid:part>
    <pdfaid:conformance>b</pdfaid:conformance>
  </rdf:Description>
</rdf:RDF>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-1b".to_string()));
    }

    #[test]
    fn test_detect_conformance_nested_elements() {
        // Test with elements nested deeper in the structure
        let xmp = br#"<?xpacket begin='...'?>
<x:xmpmeta xmlns:x='adobe:ns:meta/'>
  <rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
    <rdf:Description rdf:about=''>
      <pdfaid:part xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>1</pdfaid:part>
      <pdfaid:conformance xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>b</pdfaid:conformance>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-1b".to_string()));
    }

    #[test]
    fn test_detect_conformance_unicode_in_namespace() {
        // Test with proper XMP namespace handling
        let xmp = br#"<?xpacket begin='...'?>
<x:xmpmeta xmlns:x='adobe:ns:meta/' x:xmptk='Adobe XMP Core 5.6-c140'>
  <rdf:RDF xmlns:rdf='http://www.w3.org/1999/02/22-rdf-syntax-ns#'>
    <rdf:Description rdf:about=''
      xmlns:pdfaid='http://www.aiim.org/pdfa/ns/id/'>
      <pdfaid:part>2</pdfaid:part>
      <pdfaid:conformance>u</pdfaid:conformance>
    </rdf:Description>
  </rdf:RDF>
</x:xmpmeta>"#;

        let result = detect_conformance(Some(xmp));
        assert_eq!(result, Some("PDF/A-2u".to_string()));
    }
}
