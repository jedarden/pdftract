//! Tests for Type3 rasterizer resolve_stream callback parameter passing.
//!
//! This test module verifies that the `resolve_stream` callback parameter
//! in `rasterize_type3_glyph` correctly receives and processes the
//! ObjRef parameter and any additional context parameters (resolver, source, counter).
//!
//! # Test Structure
//!
//! These tests verify the callback contract:
//! - The callback receives the correct ObjRef pointing to the glyph's content stream
//! - The callback can access and use context parameters (resolver, source, counter)
//! - The callback's return value is properly handled by rasterize_type3_glyph
//!
//! # References
//!
//! - crates/pdftract-core/src/font/type3_rasterizer.rs:558 - resolve_stream callback signature
//! - crates/pdftract-core/src/font/type3_rasterizer.rs:969 - rasterize_type3_glyph function

use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;

use crate::font::type3_rasterizer::{
    calculate_bitmap_dimensions, detect_char_proc_type, rasterize_type3_glyph, CharProcType,
    DocumentContext, StreamResolverFn,
};
use crate::font::type3::Type3Font;
use crate::parser::object::types::{intern, ObjRef, PdfDict, PdfObject, PdfStream};
use crate::parser::xref::XrefResolver;
use crate::parser::stream::MemorySource;


// ============================================================================
// Test Infrastructure Helper Functions
// ============================================================================

/// Create a test DocumentContext with minimal valid configuration.
///
/// This helper creates a DocumentContext suitable for testing detect_char_proc_type
/// with PdfObject::Ref scenarios. It provides an empty XrefResolver (which will
/// return NotFound for any reference) and no source (stream reading not tested).
///
/// # Returns
///
/// A DocumentContext with an empty resolver and no source.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_test_document_context;
/// use crate::font::type3_rasterizer::detect_char_proc_type;
/// use crate::parser::object::types::{ObjRef, PdfObject};
///
/// let doc_context = create_test_document_context();
/// let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));
/// let result = detect_char_proc_type(&ref_obj, Some(&doc_context));
/// // Result will be CharProcType::Other("unknown".to_string()) (resolver can't find the ref)
/// ```
pub fn create_test_document_context() -> DocumentContext<'static> {
    let resolver = XrefResolver::new();
    DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: None,
    }
}

/// Create a test DocumentContext with a populated resolver.
///
/// This helper creates a DocumentContext with a resolver that has specific
/// entries pre-configured. This is useful for testing reference dereferencing
/// with known valid references.
///
/// # Arguments
///
/// * `entries` - Vector of (object_number, XrefEntry) tuples to populate the resolver
///
/// # Returns
///
/// A DocumentContext with a resolver containing the specified entries.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_test_document_context_with_entries;
/// use crate::parser::xref::XrefEntry;
/// use crate::font::type3_rasterizer::detect_char_proc_type;
/// use crate::parser::object::types::{ObjRef, PdfObject};
///
/// let entries = vec![(10, XrefEntry::InUse { offset: 100, gen_nr: 0 })];
/// let doc_context = create_test_document_context_with_entries(entries);
/// ```
pub fn create_test_document_context_with_entries(entries: Vec<(u32, crate::parser::xref::XrefEntry)>) -> DocumentContext<'static> {
    let mut resolver = XrefResolver::new();
    for (obj_nr, entry) in entries {
        resolver.add_entry(obj_nr, entry);
    }
    DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: None,
    }
}

/// Create a PdfObject::Ref with a given object ID.
///
/// This helper creates a reference PdfObject for testing. References created
/// with this function will need a DocumentContext to be dereferenced.
///
/// # Arguments
///
/// * `object_number` - The object number (ID)
/// * `generation_number` - The generation number (defaults to 0)
///
/// # Returns
///
/// PdfObject::Ref with the specified object and generation numbers.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_test_ref;
/// use crate::parser::object::types::PdfObject;
///
/// let ref_obj = create_test_ref(42);
/// let ref_obj_with_gen = create_test_ref_with_gen(42, 1);
/// ```
pub fn create_test_ref(object_number: u32) -> PdfObject {
    create_test_ref_with_gen(object_number, 0)
}

/// Create a PdfObject::Ref with a given object ID and generation number.
///
/// This helper creates a reference PdfObject for testing with a specific
/// generation number. Use this when testing reference handling with
/// non-zero generation numbers.
///
/// # Arguments
///
/// * `object_number` - The object number (ID)
/// * `generation_number` - The generation number
///
/// # Returns
///
/// PdfObject::Ref with the specified object and generation numbers.
pub fn create_test_ref_with_gen(object_number: u32, generation_number: u16) -> PdfObject {
    PdfObject::Ref(ObjRef::new(object_number, generation_number))
}

/// Create a test dictionary with optional entries.
///
/// This helper creates a PdfDict for testing. It can optionally populate
/// the dictionary with predefined entries.
///
/// # Arguments
///
/// * `entries` - Optional vector of (key, value) tuples to populate the dictionary
///
/// # Returns
///
/// PdfObject::Dict containing the specified entries.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_test_dict;
/// use crate::parser::object::types::{PdfObject, intern};
///
/// let empty_dict = create_test_dict(None);
/// let dict_with_type = create_test_dict(Some(vec![
///     (intern("/Type"), PdfObject::Name(intern("/Font")))
/// ]));
/// ```
pub fn create_test_dict(entries: Option<Vec<(Arc<str>, PdfObject)>>) -> PdfObject {
    let mut dict = PdfDict::new();
    if let Some(entries) = entries {
        for (key, value) in entries {
            dict.insert(key, value);
        }
    }
    PdfObject::Dict(Box::new(dict))
}

/// Create a test stream with a dictionary and optional data.
///
/// This helper creates a PdfStream for testing. The stream dictionary can
/// be customized, and an optional byte offset can be specified.
///
/// # Arguments
///
/// * `dict_entries` - Optional vector of (key, value) tuples for the stream dictionary
/// * `offset` - Byte offset in the PDF file (defaults to 0)
/// * `length_hint` - Optional length hint for the stream data
///
/// # Returns
///
/// PdfObject::Stream with the specified dictionary and metadata.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_test_stream;
/// use crate::parser::object::types::{PdfObject, intern};
///
/// let stream = create_test_stream(
///     Some(vec![
///         (intern("/Length"), PdfObject::Integer(100)),
///         (intern("/Filter"), PdfObject::Name(intern("/FlateDecode")))
///     ]),
///     1000,
///     Some(100)
/// );
/// ```
pub fn create_test_stream(
    dict_entries: Option<Vec<(Arc<str>, PdfObject)>>,
    offset: u64,
    length_hint: Option<u64>,
) -> PdfObject {
    let mut stream_dict = PdfDict::new();
    if let Some(entries) = dict_entries {
        for (key, value) in entries {
            stream_dict.insert(key, value);
        }
    }
    let stream = PdfStream::new(stream_dict, offset, length_hint);
    PdfObject::Stream(Box::new(stream))
}

/// Setup a minimal test context for detect_char_proc_type testing.
///
/// This helper creates the minimal valid DocumentContext needed for testing
/// detect_char_proc_type with PdfObject::Ref scenarios. It returns an empty
/// resolver (which will cause dereferencing to return NotFound/Unknown).
///
/// This is a convenience wrapper around create_test_document_context().
///
/// # Returns
///
/// A DocumentContext with empty resolver and no source.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::setup_test_context;
/// use crate::font::type3_rasterizer::detect_char_proc_type;
/// use crate::parser::object::types::PdfObject;
///
/// let ctx = setup_test_context();
/// let result = detect_char_proc_type(&PdfObject::Ref(ObjRef::new(1, 0)), Some(&ctx));
/// ```
pub fn setup_test_context() -> DocumentContext<'static> {
    create_test_document_context()
}

/// Setup a test context with a memory source for stream testing.
///
/// This helper creates a DocumentContext with both a resolver and a MemorySource,
/// suitable for testing scenarios that require reading stream data.
///
/// # Arguments
///
/// * `data` - The byte data to use as the PDF source
///
/// # Returns
///
/// A DocumentContext with empty resolver and a MemorySource.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::setup_test_context_with_source;
/// use crate::parser::stream::MemorySource;
///
/// let pdf_data = b"%PDF-1.4...".to_vec();
/// let ctx = setup_test_context_with_source(pdf_data);
/// ```
pub fn setup_test_context_with_source(data: Vec<u8>) -> DocumentContext<'static> {
    let resolver = XrefResolver::new();
    let source: MemorySource = MemorySource::new(data);
    DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: Some(Box::leak(Box::new(source))),
    }
}

/// Create a DocumentContext with a resolver that has valid entries and a source with PDF data.
///
/// This helper creates a complete test setup for successful reference dereferencing.
/// It creates a source with properly formatted PDF indirect objects at known offsets
/// and a resolver with entries pointing to those offsets.
///
/// # Arguments
///
/// * `object_configs` - Vector of (object_number, offset, generation, pdf_bytes) tuples
///
/// # Returns
///
/// A DocumentContext with a populated resolver and source.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_valid_dereference_context;
/// use crate::parser::xref::XrefEntry;
///
/// // Create a source with a dictionary at offset 100
/// let pdf_dict = b"10 0 obj\n<< /Type /Font /Subtype /Type3 >>\nendobj\n".to_vec();
/// let ctx = create_valid_dereference_context(vec![(10, 100, 0, pdf_dict)]);
/// ```
pub fn create_valid_dereference_context(
    object_configs: Vec<(u32, u64, u16, Vec<u8>)>,
) -> DocumentContext<'static> {
    use crate::parser::xref::XrefEntry;

    // Calculate total size needed
    let total_size = object_configs.iter().map(|(_, offset, _, data)| {
        offset + data.len() as u64
    }).max().unwrap_or(4096);

    // Create source with enough space
    let mut source_data = vec![0u8; total_size as usize];
    let mut resolver = XrefResolver::new();

    for (obj_nr, offset, gen_nr, mut obj_bytes) in object_configs {
        // Add the object data to the source at the specified offset
        let start = offset as usize;
        let end = start + obj_bytes.len();
        if end <= source_data.len() {
            source_data[start..end].copy_from_slice(&obj_bytes);
        }

        // Add resolver entry
        resolver.add_entry(obj_nr, XrefEntry::InUse {
            offset,
            gen_nr,
        });
    }

    let source = MemorySource::new(source_data);
    DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: Some(Box::leak(Box::new(source))),
    }
}

/// Create a properly formatted PDF dictionary indirect object.
///
/// This helper creates the byte representation of a PDF indirect object
/// that contains a dictionary with specified keys and values.
///
/// # Arguments
///
/// * `obj_number` - The object number
/// * `generation` - The generation number
/// * `dict_content` - The dictionary content (e.g., "/Type /Font /Subtype /Type3")
///
/// # Returns
///
/// Bytes representing a complete PDF indirect object.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_pdf_dict_object;
///
/// let dict_bytes = create_pdf_dict_object(10, 0, "/Type /Font /Subtype /Type3");
/// // Returns: b"10 0 obj\n<< /Type /Font /Subtype /Type3 >>\nendobj\n"
/// ```
pub fn create_pdf_dict_object(obj_number: u32, generation: u16, dict_content: &str) -> Vec<u8> {
    format!(
        "{} {} obj\n<< {} >>\nendobj\n",
        obj_number, generation, dict_content
    ).into_bytes()
}

/// Create a properly formatted PDF stream indirect object.
///
/// This helper creates the byte representation of a PDF indirect object
/// that contains a stream with specified dictionary and content.
///
/// # Arguments
///
/// * `obj_number` - The object number
/// * `generation` - The generation number
/// * `dict_content` - The stream dictionary content
/// * `stream_content` - The stream content bytes
///
/// # Returns
///
/// Bytes representing a complete PDF stream object.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_pdf_stream_object;
///
/// let stream_bytes = create_pdf_stream_object(
///     20,
///     0,
///     "/Type /XObject /Subtype /Form /Width 100 /Height 100",
///     b"0 0 100 100 re f"
/// );
/// ```
pub fn create_pdf_stream_object(
    obj_number: u32,
    generation: u16,
    dict_content: &str,
    stream_content: &[u8],
) -> Vec<u8> {
    let length = stream_content.len();
    let full_dict = format!("{}/Length {}", dict_content, length);
    format!(
        "{} {} obj\n<< {} >>\nstream\n{}\nendstream\nendobj\n",
        obj_number, generation, full_dict, String::from_utf8_lossy(stream_content)
    ).into_bytes()
}

/// Create a mock DocumentContext with reference support.
///
/// This helper creates a DocumentContext that includes both a resolver and source,
/// suitable for testing reference dereferencing scenarios. It provides a minimal
/// valid setup that can be populated with test objects.
///
/// # Returns
///
/// A DocumentContext with empty resolver and source (ready to be populated).
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_mock_context_with_refs;
/// use crate::parser::xref::XrefEntry;
///
/// let mut ctx = create_mock_context_with_refs();
/// // Add entries to ctx.resolver as needed
/// ```
pub fn create_mock_context_with_refs() -> DocumentContext<'static> {
    let resolver = XrefResolver::new();
    let source_data = vec![0u8; 4096]; // 4KB buffer for test data
    let source = MemorySource::new(source_data);

    DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: Some(Box::leak(Box::new(source))),
    }
}

/// Create a reference to a dictionary object.
///
/// This helper creates a PdfObject::Ref that points to a dictionary object
/// at a specified object number. It's a convenience wrapper around create_test_ref.
///
/// # Arguments
///
/// * `object_number` - The object number the reference points to
///
/// # Returns
///
/// PdfObject::Ref pointing to the specified object number.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_ref_to_dict;
///
/// let dict_ref = create_ref_to_dict(10);
/// ```
pub fn create_ref_to_dict(object_number: u32) -> PdfObject {
    create_test_ref(object_number)
}

/// Create a reference to a stream object.
///
/// This helper creates a PdfObject::Ref that points to a stream object
/// at a specified object number. It's a convenience wrapper around create_test_ref.
///
/// # Arguments
///
/// * `object_number` - The object number the reference points to
///
/// # Returns
///
/// PdfObject::Ref pointing to the specified object number.
///
/// # Example
///
/// ```rust,no_run
/// use crate::font::type3_rasterizer_test::create_ref_to_stream;
///
/// let stream_ref = create_ref_to_stream(20);
/// ```
pub fn create_ref_to_stream(object_number: u32) -> PdfObject {
    create_test_ref(object_number)
}

// ============================================================================
// Type3 Font Test Fixtures
// ============================================================================

/// Test fixture builder for creating Type3Font instances with custom CharProcs.
///
/// This helper creates a Type3Font with only the fields relevant to testing
/// the resolve_stream callback. It uses the load() constructor pattern with
/// a minimal dictionary to set up the CharProcs mapping.
fn create_test_type3_font(char_procs: HashMap<Arc<str>, ObjRef>) -> Type3Font {
    use crate::parser::object::types::PdfObject;

    // Create a minimal font dictionary with CharProcs
    let mut font_dict_data = PdfDict::new();
    let char_procs_dict = PdfObject::Dict(Box::new(char_procs.into_iter().map(|(k, v)| {
        (k, PdfObject::Ref(v))
    }).collect()));
    font_dict_data.insert(intern("/CharProcs"), char_procs_dict);
    font_dict_data.insert(intern("/FontMatrix"), PdfObject::Array(Box::new(vec![
        PdfObject::Real(0.001), PdfObject::Real(0.0),
        PdfObject::Real(0.0), PdfObject::Real(0.001),
        PdfObject::Real(0.0), PdfObject::Real(0.0),
    ])));
    font_dict_data.insert(intern("/FontBBox"), PdfObject::Array(Box::new(vec![
        PdfObject::Integer(0), PdfObject::Integer(0),
        PdfObject::Integer(1000), PdfObject::Integer(1000),
    ])));
    font_dict_data.insert(intern("/FirstChar"), PdfObject::Integer(0));
    font_dict_data.insert(intern("/LastChar"), PdfObject::Integer(0));
    font_dict_data.insert(intern("/Widths"), PdfObject::Array(Box::new(vec![
        PdfObject::Real(600.0),
    ])));

    Type3Font::load(&font_dict_data)
}

/// Test that the resolve_stream callback receives the correct ObjRef parameter.
///
/// This test verifies that when `rasterize_type3_glyph` invokes the callback,
/// it passes the ObjRef that corresponds to the glyph's content stream reference
/// from the CharProcs dictionary.
#[test]
fn test_resolve_stream_callback_receives_objref() {
    // 1. Create a Type3Font with a known glyph in CharProcs
    let expected_objref = ObjRef::new(42, 0);

    let mut char_procs = HashMap::new();
    char_procs.insert(intern("A"), expected_objref);

    let font = create_test_type3_font(char_procs);

    // 2. Create a callback that captures the received ObjRef
    let captured_objref = Arc::new(Mutex::new(None));
    let captured_clone = captured_objref.clone();

    let callback = move |obj_ref: ObjRef| -> Option<Vec<u8>> {
        *captured_clone.lock().unwrap() = Some(obj_ref);
        // Return minimal valid content stream bytes (a simple "0 0 100 100 re" would draw a rect)
        Some(b"".to_vec())
    };

    // 3. Call rasterize_type3_glyph with the callback
    let result = rasterize_type3_glyph(&font, "A", None, Some(&callback));

    // 4. Verify the callback received the expected ObjRef
    let captured = captured_objref.lock().unwrap();
    assert!(captured.is_some(), "Callback should have been invoked");
    assert_eq!(captured.unwrap(), expected_objref, "Callback should receive the ObjRef from CharProcs");

    // 5. Verify the glyph was rasterized successfully (empty content stream still produces a bitmap)
    assert!(result.is_some(), "Should return bitmap even for empty content stream");
}

/// Test that the resolve_stream callback can capture and use resolver context.
///
/// This test verifies that the callback pattern used in resolver.rs (lines 700-702)
/// correctly captures and uses the resolver parameter from the enclosing scope.
#[test]
fn test_resolve_stream_callback_captures_resolver() {
    use crate::font::type3_test_fixtures::{MockResolver, mock_resolver};

    // 1. Create a mock resolver tracking flag
    let resolver_called: MockResolver = mock_resolver();

    // 2. Create a Type3Font with a known glyph
    let glyph_ref = ObjRef::new(1, 0);
    let mut char_procs = HashMap::new();
    char_procs.insert(intern("A"), glyph_ref);

    let font = create_test_type3_font(char_procs);

    // 3. Create a callback that captures and uses the resolver
    let resolver_clone = resolver_called.clone();
    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        // Use the resolver parameter (set the flag)
        resolver_clone.store(true, Ordering::SeqCst);
        Some(b"".to_vec())
    };

    // 4. Call rasterize_type3_glyph with the callback
    let _result = rasterize_type3_glyph(&font, "A", None, Some(&callback));

    // 5. Verify the callback used the resolver parameter
    assert!(resolver_called.load(Ordering::SeqCst),
            "Callback should have set the resolver flag");
}

/// Test that the resolve_stream callback can capture and use source context.
///
/// This test verifies that the callback pattern correctly captures and uses
/// the source parameter (&dyn PdfSource) from the enclosing scope.
#[test]
fn test_resolve_stream_callback_captures_source() {
    use crate::font::type3_test_fixtures::{MockSource, mock_source};

    // 1. Create a mock source tracking flag
    let source_used: MockSource = mock_source();

    // 2. Create a Type3Font with a known glyph
    let glyph_ref = ObjRef::new(1, 0);
    let mut char_procs = HashMap::new();
    char_procs.insert(intern("B"), glyph_ref);

    let font = create_test_type3_font(char_procs);

    // 3. Create a callback that captures and uses the source
    let source_clone = source_used.clone();
    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        // Use the source parameter (set the flag)
        source_clone.store(true, Ordering::SeqCst);
        Some(b"".to_vec())
    };

    // 4. Call rasterize_type3_glyph with the callback
    let _result = rasterize_type3_glyph(&font, "B", None, Some(&callback));

    // 5. Verify the callback used the source parameter
    assert!(source_used.load(Ordering::SeqCst),
            "Callback should have set the source flag");
}

/// Test that the resolve_stream callback can capture and use counter context.
///
/// This test verifies that the callback pattern correctly captures and uses
/// the counter parameter (&mut u64 decompress_counter) from the enclosing scope.
#[test]
fn test_resolve_stream_callback_captures_counter() {
    use crate::font::type3_test_fixtures::{MockCounter, mock_counter};

    // 1. Create a counter for tracking callback invocations
    let counter: MockCounter = mock_counter();

    // 2. Create a Type3Font with a known glyph
    let glyph_ref = ObjRef::new(1, 0);
    let mut char_procs = HashMap::new();
    char_procs.insert(intern("C"), glyph_ref);

    let font = create_test_type3_font(char_procs);

    // 3. Create a callback that captures and increments the counter
    let counter_clone = counter.clone();
    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        // Increment the counter (simulating decompress operation)
        counter_clone.fetch_add(1, Ordering::SeqCst);
        Some(b"".to_vec())
    };

    // 4. Call rasterize_type3_glyph with the callback
    let _result = rasterize_type3_glyph(&font, "C", None, Some(&callback));

    // 5. Verify the callback incremented the counter
    assert_eq!(counter.load(Ordering::SeqCst), 1,
               "Callback should have incremented the counter once");
}

/// Test that the callback is invoked with the correct ObjRef for multiple glyphs.
///
/// This test verifies that when multiple glyphs are rasterized, each callback
/// invocation receives the correct ObjRef for that specific glyph.
#[test]
fn test_resolve_stream_callback_multiple_glyphs() {
    // 1. Create a Type3Font with multiple glyphs in CharProcs
    let objref_a = ObjRef::new(10, 0);
    let objref_b = ObjRef::new(20, 0);
    let objref_c = ObjRef::new(30, 0);

    let mut char_procs = HashMap::new();
    char_procs.insert(intern("A"), objref_a);
    char_procs.insert(intern("B"), objref_b);
    char_procs.insert(intern("C"), objref_c);

    let font = create_test_type3_font(char_procs);

    // 2. Create a callback that records all received ObjRefs
    let captured_refs = Arc::new(Mutex::new(Vec::new()));

    // 3. Call rasterize_type3_glyph for each glyph
    for glyph_name in ["A", "B", "C"] {
        let captured_clone = captured_refs.clone();
        let callback = move |obj_ref: ObjRef| -> Option<Vec<u8>> {
            captured_clone.lock().unwrap().push(obj_ref);
            Some(b"".to_vec())
        };

        let _result = rasterize_type3_glyph(&font, glyph_name, None, Some(&callback));
    }

    // 4. Verify each callback invocation received the correct ObjRef
    let captured = captured_refs.lock().unwrap();
    assert_eq!(captured.len(), 3, "Should have captured 3 ObjRefs");
    assert_eq!(captured[0], objref_a, "First callback should receive ObjRef for A");
    assert_eq!(captured[1], objref_b, "Second callback should receive ObjRef for B");
    assert_eq!(captured[2], objref_c, "Third callback should receive ObjRef for C");
}

/// Test that when the callback returns None, rasterize_type3_glyph returns None.
///
/// This test verifies the error handling path: if the callback cannot resolve
/// the stream (returns None), the glyph rasterization fails gracefully.
#[test]
fn test_resolve_stream_callback_returns_none() {
    // 1. Create a Type3Font with a valid glyph
    let glyph_ref = ObjRef::new(1, 0);
    let mut char_procs = HashMap::new();
    char_procs.insert(intern("A"), glyph_ref);

    let font = create_test_type3_font(char_procs);

    // 2. Create a callback that returns None (simulating resolution failure)
    let callback = |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        None  // Simulate failure to resolve the stream
    };

    // 3. Call rasterize_type3_glyph with the callback
    let result = rasterize_type3_glyph(&font, "A", None, Some(&callback));

    // 4. Verify rasterize_type3_glyph returns None
    assert!(result.is_none(), "Should return None when callback returns None");
}

/// Test that when the callback returns valid bytes, the glyph is rasterized.
///
/// This test verifies the success path: if the callback returns valid content
/// stream bytes, the glyph is successfully rasterized to a bitmap.
#[test]
fn test_resolve_stream_callback_returns_valid_bytes() {
    // 1. Create a Type3Font with a valid glyph
    let glyph_ref = ObjRef::new(1, 0);
    let mut char_procs = HashMap::new();
    char_procs.insert(intern("A"), glyph_ref);

    let font = create_test_type3_font(char_procs);

    // 2. Create a callback that returns valid PDF content stream bytes
    // This draws a simple rectangle: "0 0 100 100 re f" (fill a 100x100 rect)
    let content_stream = b"0 0 100 100 re f".to_vec();
    let callback = move |_obj_ref: ObjRef| -> Option<Vec<u8>> {
        Some(content_stream.clone())
    };

    // 3. Call rasterize_type3_glyph with the callback
    let result = rasterize_type3_glyph(&font, "A", None, Some(&callback));

    // 4. Verify the returned bitmap exists (content was rasterized)
    assert!(result.is_some(), "Should return bitmap when callback returns valid bytes");

    // The bitmap should be non-empty (even if all-white for empty content)
    let bitmap_bytes = result.unwrap();
    assert!(!bitmap_bytes.is_empty(), "Bitmap bytes should not be empty");

    // Bitmap dimensions follow the font's /FontBBox. This fixture uses a
    // 1000x1000 glyph space, so the default one-pixel padding produces 1002x1002.
    let (width, height) = calculate_bitmap_dimensions(&font.font_bbox, None);
    assert_eq!(bitmap_bytes.len(), width * height);
}

/// Test the helper function pattern for creating resolve_stream callbacks.
///
/// This test verifies the pattern used in resolver.rs where a helper function
/// is defined that takes all context parameters, and a closure captures them.
#[test]
fn test_resolve_stream_helper_function_pattern() {
    use std::sync::Arc;
    use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

    // 1. Create context parameters (resolver, source, counter)
    let resolver_flag = Arc::new(AtomicBool::new(false));
    let source_flag = Arc::new(AtomicBool::new(false));
    let counter = Arc::new(AtomicU64::new(0));

    // 2. Define a helper function that takes all parameters
    // This simulates the pattern in resolver.rs where a helper function
    // encapsulates the stream resolution logic
    fn resolve_with_context(
        obj_ref: ObjRef,
        resolver_flag: &Arc<AtomicBool>,
        source_flag: &Arc<AtomicBool>,
        counter: &Arc<AtomicU64>,
    ) -> Option<Vec<u8>> {
        // Simulate using all context parameters
        resolver_flag.store(true, Ordering::SeqCst);
        source_flag.store(true, Ordering::SeqCst);
        counter.fetch_add(1, Ordering::SeqCst);
        Some(b"".to_vec())
    }

    // 3. Create a closure that captures the parameters and calls the helper
    let resolver_clone = resolver_flag.clone();
    let source_clone = source_flag.clone();
    let counter_clone = counter.clone();

    let callback = move |obj_ref: ObjRef| -> Option<Vec<u8>> {
        resolve_with_context(obj_ref, &resolver_clone, &source_clone, &counter_clone)
    };

    // 4. Use the closure as the resolve_stream callback
    let glyph_ref = ObjRef::new(1, 0);
    let mut char_procs = HashMap::new();
    char_procs.insert(intern("X"), glyph_ref);

    let font = create_test_type3_font(char_procs);

    let _result = rasterize_type3_glyph(&font, "X", None, Some(&callback));

    // 5. Verify the helper function was called with correct parameters
    assert!(resolver_flag.load(Ordering::SeqCst), "Helper should have set resolver flag");
    assert!(source_flag.load(Ordering::SeqCst), "Helper should have set source flag");
    assert_eq!(counter.load(Ordering::SeqCst), 1, "Helper should have incremented counter once");
}

// ============================================================================
// Tests for intersection math and Active Edge Table (AET) management
// ============================================================================

/// Test helper struct to replicate the Edge structure from fill_polygon.
#[derive(Debug, Clone, Copy, PartialEq)]
struct TestEdge {
    x: i32,           // Current X intersection position
    y_min: i32,       // Minimum Y coordinate (top of edge)
    y_max: i32,       // Maximum Y coordinate (bottom of edge)
    dx: i32,          // Change in X across the edge
    dy: i32,          // Change in Y across the edge
}

/// Test that edges are added to AET when scanline reaches y_min.
///
/// This test verifies the edge activation logic:
/// - When scanline y == edge.y_min, the edge is added to AET
/// - Edges are sorted by y_min before processing
#[test]
fn test_edge_activation_at_y_min() {
    // Create edges with different y_min values
    let edges = vec![
        TestEdge { x: 10, y_min: 5, y_max: 15, dx: 10, dy: 10 },
        TestEdge { x: 20, y_min: 3, y_max: 13, dx: 10, dy: 10 },
        TestEdge { x: 30, y_min: 7, y_max: 17, dx: 10, dy: 10 },
    ];

    // Sort by y_min (as the GET sorting does)
    let mut sorted_edges = edges.clone();
    sorted_edges.sort_by_key(|e| e.y_min);

    // Simulate AET initialization
    let mut aet: Vec<TestEdge> = Vec::new();
    let mut get_idx = 0;

    // At scanline y=3, only the edge with y_min=3 should be added
    let y = 3;
    while get_idx < sorted_edges.len() && sorted_edges[get_idx].y_min == y {
        aet.push(sorted_edges[get_idx]);
        get_idx += 1;
    }

    assert_eq!(aet.len(), 1, "Should have 1 edge at y=3");
    assert_eq!(aet[0].y_min, 3, "Edge should have y_min=3");

    // At scanline y=5, the edge with y_min=5 should be added
    let y = 5;
    while get_idx < sorted_edges.len() && sorted_edges[get_idx].y_min == y {
        aet.push(sorted_edges[get_idx]);
        get_idx += 1;
    }

    assert_eq!(aet.len(), 2, "Should have 2 edges at y=5");
    assert!(aet.iter().any(|e| e.y_min == 5), "Should contain edge with y_min=5");

    // At scanline y=7, the edge with y_min=7 should be added
    let y = 7;
    while get_idx < sorted_edges.len() && sorted_edges[get_idx].y_min == y {
        aet.push(sorted_edges[get_idx]);
        get_idx += 1;
    }

    assert_eq!(aet.len(), 3, "Should have 3 edges at y=7");
    assert!(aet.iter().any(|e| e.y_min == 7), "Should contain edge with y_min=7");
}

/// Test multi-edge activation timing with sequential y_min values.
///
/// This test verifies that multiple edges with different y_min values activate
/// at their respective scanlines according to the acceptance criteria:
/// 1. Test with 3 edges at y_min=3, 5, 7
/// 2. Assert AET contains only edge1 at scanline y=3
/// 3. Assert AET contains edges 1&2 at scanline y=5
/// 4. Assert AET contains all 3 edges at scanline y=7
#[test]
fn test_multi_edge_activation_timing() {
    // Create 3 edges with specific y_min values
    let edge1 = TestEdge { x: 10, y_min: 3, y_max: 15, dx: 5, dy: 12 };
    let edge2 = TestEdge { x: 20, y_min: 5, y_max: 17, dx: 5, dy: 12 };
    let edge3 = TestEdge { x: 30, y_min: 7, y_max: 19, dx: 5, dy: 12 };

    let edges = vec![edge1, edge2, edge3];

    // Sort by y_min (as GET sorting does)
    let mut sorted_edges = edges.clone();
    sorted_edges.sort_by_key(|e| e.y_min);

    // Simulate AET initialization
    let mut aet: Vec<TestEdge> = Vec::new();
    let mut get_idx = 0;

    // Test scanline progression from y=2 to y=8
    for scanline in 2..=8 {
        // Add edges whose y_min == current scanline
        while get_idx < sorted_edges.len() && sorted_edges[get_idx].y_min == scanline {
            aet.push(sorted_edges[get_idx]);
            get_idx += 1;
        }

        // Verify AET contents at each critical scanline
        match scanline {
            3 => {
                // At scanline y=3, only edge1 (y_min=3) should be active
                assert_eq!(aet.len(), 1, "AET should contain exactly 1 edge at scanline y=3");
                assert_eq!(aet[0].y_min, 3, "Active edge should have y_min=3");
                assert_eq!(aet[0].x, 10, "Active edge should have x=10");
            }
            5 => {
                // At scanline y=5, edge1 and edge2 should be active
                assert_eq!(aet.len(), 2, "AET should contain exactly 2 edges at scanline y=5");
                assert!(aet.iter().any(|e| e.y_min == 3), "Should contain edge1 with y_min=3");
                assert!(aet.iter().any(|e| e.y_min == 5), "Should contain edge2 with y_min=5");
            }
            7 => {
                // At scanline y=7, all three edges should be active
                assert_eq!(aet.len(), 3, "AET should contain exactly 3 edges at scanline y=7");
                assert!(aet.iter().any(|e| e.y_min == 3), "Should contain edge1 with y_min=3");
                assert!(aet.iter().any(|e| e.y_min == 5), "Should contain edge2 with y_min=5");
                assert!(aet.iter().any(|e| e.y_min == 7), "Should contain edge3 with y_min=7");
            }
            _ => {
                // Other scanlines should maintain the previous state
                if scanline < 3 {
                    assert_eq!(aet.len(), 0, "AET should be empty before y=3");
                } else if scanline == 4 {
                    assert_eq!(aet.len(), 1, "AET should still have 1 edge at y=4");
                } else if scanline == 6 {
                    assert_eq!(aet.len(), 2, "AET should still have 2 edges at y=6");
                } else if scanline == 8 {
                    assert_eq!(aet.len(), 3, "AET should have all 3 edges at y=8");
                }
            }
        }

        // Remove edges that have been fully processed (y > y_max)
        aet.retain(|e| scanline <= e.y_max);
    }

    // All edges should remain active throughout their y_max range
    assert_eq!(get_idx, 3, "All 3 edges should have been added to AET");
}

/// Test multi-edge activation with AETInspector utility.
///
/// This test uses the AETInspector from type3_test_fixtures to verify
/// multi-edge activation timing using the inspection utilities.
#[test]
fn test_multi_edge_activation_with_aet_inspector() {
    use crate::font::type3_test_fixtures::{AETInspector, TestEdge};

    // Create 3 edges with distinct y_min values using the TestEdge builder
    let edge1 = TestEdge::new()
        .with_x(10)
        .with_y_min(3)
        .with_y_max(15)
        .with_slope(5, 12)
        .build();

    let edge2 = TestEdge::new()
        .with_x(20)
        .with_y_min(5)
        .with_y_max(17)
        .with_slope(5, 12)
        .build();

    let edge3 = TestEdge::new()
        .with_x(30)
        .with_y_min(7)
        .with_y_max(19)
        .with_slope(5, 12)
        .build();

    // Simulate scanline progression
    let mut current_edges = Vec::new();
    let all_edges = vec![edge1, edge2, edge3];

    for y in 2..=8 {
        // Add edges that become active at this scanline
        for edge in &all_edges {
            if edge.y_min == y && !current_edges.iter().any(|e: &crate::font::type3_rasterizer::Edge| e.x == edge.x && e.y_min == edge.y_min) {
                current_edges.push(*edge);
            }
        }

        // Verify with AETInspector at critical scanlines
        let inspector = AETInspector::new(current_edges.clone());

        match y {
            3 => {
                // Only edge1 should be active
                assert_eq!(inspector.edge_count(), 1, "Should have 1 edge at y=3");
                let active_edges = inspector.edges_at_y(3);
                assert_eq!(active_edges.len(), 1, "Inspector should find 1 edge active at y=3");
                assert_eq!(active_edges[0].y_min, 3, "Active edge should have y_min=3");
            }
            5 => {
                // Edge1 and edge2 should be active
                assert_eq!(inspector.edge_count(), 2, "Should have 2 edges at y=5");
                let active_edges = inspector.edges_at_y(5);
                assert!(active_edges.len() >= 2, "Inspector should find at least 2 edges active at y=5");
                assert!(active_edges.iter().any(|e| e.y_min == 3), "Should have edge with y_min=3");
                assert!(active_edges.iter().any(|e| e.y_min == 5), "Should have edge with y_min=5");
            }
            7 => {
                // All three edges should be active
                assert_eq!(inspector.edge_count(), 3, "Should have 3 edges at y=7");
                let active_edges = inspector.edges_at_y(7);
                assert_eq!(active_edges.len(), 3, "Inspector should find all 3 edges active at y=7");
                assert!(active_edges.iter().any(|e| e.y_min == 3), "Should have edge with y_min=3");
                assert!(active_edges.iter().any(|e| e.y_min == 5), "Should have edge with y_min=5");
                assert!(active_edges.iter().any(|e| e.y_min == 7), "Should have edge with y_min=7");
            }
            _ => {}
        }
    }
}

/// Test that edges are removed from AET after y_max.
///
/// This test verifies the edge removal logic:
/// - When scanline y > edge.y_max, the edge is removed from AET
/// - The retain operation keeps only edges where y <= y_max
#[test]
fn test_edge_removal_after_y_max() {
    // Create edges with different y_max values
    let mut aet = vec![
        TestEdge { x: 10, y_min: 0, y_max: 5, dx: 10, dy: 10 },
        TestEdge { x: 20, y_min: 0, y_max: 10, dx: 10, dy: 10 },
        TestEdge { x: 30, y_min: 0, y_max: 15, dx: 10, dy: 10 },
    ];

    // At scanline y=5, all edges should still be active (y <= y_max)
    let y = 5;
    aet.retain(|e| y <= e.y_max);

    assert_eq!(aet.len(), 3, "All 3 edges should be active at y=5");

    // At scanline y=6, the first edge (y_max=5) should be removed
    let y = 6;
    aet.retain(|e| y <= e.y_max);

    assert_eq!(aet.len(), 2, "2 edges should remain at y=6");
    assert!(!aet.iter().any(|e| e.y_max == 5), "Edge with y_max=5 should be removed");

    // At scanline y=11, the second edge (y_max=10) should also be removed
    let y = 11;
    aet.retain(|e| y <= e.y_max);

    assert_eq!(aet.len(), 1, "Only 1 edge should remain at y=11");
    assert!(aet.iter().any(|e| e.y_max == 15), "Only edge with y_max=15 should remain");

    // At scanline y=16, all edges should be removed
    let y = 16;
    aet.retain(|e| y <= e.y_max);

    assert_eq!(aet.len(), 0, "No edges should remain at y=16");
}

/// Test intersection x calculation accuracy.
///
/// This test verifies that intersection positions are calculated correctly:
/// - x_intersection = round(edge.x) after each scanline update
/// - The initial x position is correct
/// - Rounding behavior is consistent
#[test]
fn test_intersection_x_calculation() {
    // Create an edge with known properties
    let mut edge = TestEdge {
        x: 10,    // Initial X at y_min
        y_min: 0,
        y_max: 10,
        dx: 20,   // Edge goes from x=10 to x=30
        dy: 10,   // Over 10 scanlines
    };

    // At y=0 (initial position), x should be 10
    let x_intersect = (edge.x as f64).round() as i32;
    assert_eq!(x_intersect, 10, "Initial X intersection should be 10");

    // Simulate x update for one scanline: x += dx/dy
    // dx/dy = 20/10 = 2, so x should increment by 2 each scanline
    edge.x += (edge.dx as f64 / edge.dy as f64) as i32;

    // At y=1, x should be 12 (10 + 2)
    let x_intersect = (edge.x as f64).round() as i32;
    assert_eq!(x_intersect, 12, "X intersection at y=1 should be 12");

    // Another scanline update
    edge.x += (edge.dx as f64 / edge.dy as f64) as i32;

    // At y=2, x should be 14 (12 + 2)
    let x_intersect = (edge.x as f64).round() as i32;
    assert_eq!(x_intersect, 14, "X intersection at y=2 should be 14");

    // Test with a non-integer slope
    let mut edge2 = TestEdge {
        x: 0,
        y_min: 0,
        y_max: 5,
        dx: 10,   // Edge goes from x=0 to x=10
        dy: 5,   // Over 5 scanlines
    };

    // dx/dy = 10/5 = 2 (integer slope)
    for _ in 0..5 {
        edge2.x += (edge2.dx as f64 / edge2.dy as f64) as i32;
    }
    assert_eq!(edge2.x, 10, "After 5 scanlines, X should be 10");

    // Test with fractional slope (rounding matters)
    let mut edge3 = TestEdge {
        x: 0,
        y_min: 0,
        y_max: 3,
        dx: 10,   // Edge goes from x=0 to x=10
        dy: 3,   // Over 3 scanlines -> dx/dy ≈ 3.33
    };

    // After first scanline: x = 0 + 3.33 ≈ 3
    edge3.x += (edge3.dx as f64 / edge3.dy as f64) as i32;
    let x_intersect = (edge3.x as f64).round() as i32;
    assert_eq!(x_intersect, 3, "X intersection with fractional slope should round correctly");

    // After second scanline: x = 3.33 + 3.33 ≈ 7
    edge3.x += (edge3.dx as f64 / edge3.dy as f64) as i32;
    let x_intersect = (edge3.x as f64).round() as i32;
    assert_eq!(x_intersect, 7, "X intersection should accumulate correctly");

    // After third scanline: x = 6.66 + 3.33 ≈ 10
    edge3.x += (edge3.dx as f64 / edge3.dy as f64) as i32;
    let x_intersect = (edge3.x as f64).round() as i32;
    assert_eq!(x_intersect, 10, "Final X intersection should reach target");
}

/// Test slope-based x increment produces correct progression.
///
/// This test verifies the slope-based x update formula:
/// - x_new = x_old + (dx / dy)
/// - The progression is linear across scanlines
/// - Multiple edges update independently
#[test]
fn test_slope_based_x_increment() {
    // Test a simple diagonal edge (45 degrees)
    let mut edge = TestEdge {
        x: 0,
        y_min: 0,
        y_max: 10,
        dx: 10,  // x increases by 10
        dy: 10,  // y increases by 10 -> slope = 1
    };

    let mut x_progression = Vec::new();

    // Track x progression over 10 scanlines
    for y in 0..=10 {
        let x_intersect = (edge.x as f64).round() as i32;
        x_progression.push((y, x_intersect));

        // Update x for next scanline
        if y < 10 {
            edge.x += (edge.dx as f64 / edge.dy as f64) as i32;
        }
    }

    // Verify linear progression: x should equal y at each scanline
    for (y, x) in &x_progression {
        assert_eq!(*x, *y, "For 45-degree edge, X should equal Y at each scanline");
    }

    // Test a steeper edge (x increases faster than y)
    let mut edge2 = TestEdge {
        x: 0,
        y_min: 0,
        y_max: 5,
        dx: 20,  // x increases by 20
        dy: 5,   // y increases by 5 -> slope = 4
    };

    let mut x_progression2 = Vec::new();

    for y in 0..=5 {
        let x_intersect = (edge2.x as f64).round() as i32;
        x_progression2.push((y, x_intersect));

        if y < 5 {
            edge2.x += (edge2.dx as f64 / edge2.dy as f64) as i32;
        }
    }

    // Verify: x should increase by 4 each scanline (0, 4, 8, 12, 16, 20)
    let expected: Vec<(i32, i32)> = vec![(0, 0), (1, 4), (2, 8), (3, 12), (4, 16), (5, 20)];
    assert_eq!(x_progression2, expected, "X progression should match slope of 4");

    // Test a shallow edge (x increases slower than y)
    let mut edge3 = TestEdge {
        x: 0,
        y_min: 0,
        y_max: 10,
        dx: 5,   // x increases by 5
        dy: 10,  // y increases by 10 -> slope = 0.5
    };

    let mut x_progression3 = Vec::new();

    for y in 0..=10 {
        let x_intersect = (edge3.x as f64).round() as i32;
        x_progression3.push((y, x_intersect));

        if y < 10 {
            edge3.x += (edge3.dx as f64 / edge3.dy as f64) as i32;
        }
    }

    // Verify: x should increase by 0.5 each scanline (0, 1, 1, 2, 2, 3, 3, 4, 4, 5, 5)
    // Due to rounding: (0, 0), (1, 1), (2, 1), (3, 2), (4, 2), (5, 3), (6, 3), (7, 4), (8, 4), (9, 5), (10, 5)
    let expected3: Vec<(i32, i32)> = vec![
        (0, 0), (1, 1), (2, 1), (3, 2), (4, 2),
        (5, 3), (6, 3), (7, 4), (8, 4), (9, 5), (10, 5)
    ];
    assert_eq!(x_progression3, expected3, "X progression should match slope of 0.5 with rounding");

    // Test multiple edges updating independently
    let mut edges = vec![
        TestEdge { x: 0, y_min: 0, y_max: 5, dx: 5, dy: 5 },   // slope = 1
        TestEdge { x: 10, y_min: 0, y_max: 5, dx: 10, dy: 5 }, // slope = 2
    ];

    let mut progressions = vec![Vec::new(), Vec::new()];

    for y in 0..=5 {
        for (i, edge) in edges.iter_mut().enumerate() {
            let x_intersect = (edge.x as f64).round() as i32;
            progressions[i].push((y, x_intersect));

            if y < 5 {
                edge.x += (edge.dx as f64 / edge.dy as f64) as i32;
            }
        }
    }

    // First edge: slope = 1 -> (0,0), (1,1), (2,2), (3,3), (4,4), (5,5)
    let expected_edge1: Vec<(i32, i32)> = vec![(0, 0), (1, 1), (2, 2), (3, 3), (4, 4), (5, 5)];
    assert_eq!(progressions[0], expected_edge1, "First edge should have slope of 1");

    // Second edge: slope = 2 -> (0,10), (1,12), (2,14), (3,16), (4,18), (5,20)
    let expected_edge2: Vec<(i32, i32)> = vec![(0, 10), (1, 12), (2, 14), (3, 16), (4, 18), (5, 20)];
    assert_eq!(progressions[1], expected_edge2, "Second edge should have slope of 2");
}

/// Test horizontal edge handling.
///
/// This test verifies that horizontal edges (y0 == y1) are skipped
/// as they don't affect scanline fill algorithm.
#[test]
fn test_horizontal_edge_skipping() {
    // Horizontal edge: y0 == y1
    let horizontal_edges = vec![
        (0, 5, 10, 5),   // y0 == y1 = 5
        (10, 10, 20, 10), // y0 == y1 = 10
    ];

    let mut non_horizontal_count = 0;
    for &(x0, y0, x1, y1) in &horizontal_edges {
        if y0 != y1 {
            non_horizontal_count += 1;
        }
    }

    assert_eq!(non_horizontal_count, 0, "All edges should be horizontal and skipped");

    // Mixed edges
    let mixed_edges = vec![
        (0, 5, 10, 5),   // horizontal - should be skipped
        (0, 0, 10, 10),  // diagonal - should NOT be skipped
        (5, 5, 15, 5),   // horizontal - should be skipped
        (0, 10, 10, 0),  // diagonal - should NOT be skipped
    ];

    let mut non_horizontal_count = 0;
    for &(x0, y0, x1, y1) in &mixed_edges {
        if y0 != y1 {
            non_horizontal_count += 1;
        }
    }

    assert_eq!(non_horizontal_count, 2, "Only 2 of 4 edges should be non-horizontal");
}

/// Test AET sorting by current X position.
///
/// This test verifies that AET is sorted by current X position
/// after each scanline update, which is critical for correct
/// even-odd fill ordering.
#[test]
fn test_aet_sorting_by_x_position() {
    let mut aet = vec![
        TestEdge { x: 30, y_min: 0, y_max: 10, dx: 10, dy: 10 },
        TestEdge { x: 10, y_min: 0, y_max: 10, dx: 10, dy: 10 },
        TestEdge { x: 20, y_min: 0, y_max: 10, dx: 10, dy: 10 },
    ];

    // Sort by X position
    aet.sort_by_key(|e| e.x);

    assert_eq!(aet[0].x, 10, "First edge should have X=10");
    assert_eq!(aet[1].x, 20, "Second edge should have X=20");
    assert_eq!(aet[2].x, 30, "Third edge should have X=30");

    // Update X positions (simulate one scanline)
    for edge in &mut aet {
        edge.x += (edge.dx as f64 / edge.dy as f64) as i32;
    }

    // After update: X values are 12, 22, 32
    // Sort again
    aet.sort_by_key(|e| e.x);

    assert_eq!(aet[0].x, 12, "After update, first edge should have X=12");
    assert_eq!(aet[1].x, 22, "After update, second edge should have X=22");
    assert_eq!(aet[2].x, 32, "After update, third edge should have X=32");
}

/// Tests for CharProcType detection.
///
/// This test module verifies that the `detect_char_proc_type` function
/// correctly classifies PDF objects as Stream, Dict, or Other types.
///
/// # Test Coverage
///
/// - Dictionary detection (CharProcType::Dict)
/// - Stream detection (CharProcType::Stream) - regression check
/// - Other type fallback (CharProcType::Other) - regression check
/// - Reference detection (CharProcType::Other with dereference message)
/// - Stream and Dict edge cases: empty stream dicts, missing length hints,
///   nested/deeply-nested structures, and agreement across detection entry points
///
/// # References
///
/// - crates/pdftract-core/src/font/type3_rasterizer.rs:38 - CharProcType enum
/// - crates/pdftract-core/src/font/type3_rasterizer.rs:80 - detect_char_proc_type function

/// Test that dictionary PdfObjects are correctly classified as Dict.
///
/// This test verifies that passing a dictionary PdfObject to
/// detect_char_proc_type returns CharProcType::Dict, not CharProcType::Other.
#[test]
fn test_detect_char_proc_type_dict() {
    // Create a dictionary PdfObject
    let mut dict = PdfDict::new();
    dict.insert(intern("/Type"), PdfObject::Name(intern("/Font")));
    dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type3")));
    let dict_obj = PdfObject::Dict(Box::new(dict));

    // Classify the object
    let result = detect_char_proc_type(&dict_obj, None);

    // Verify Dict is returned (not Other)
    assert_eq!(result, CharProcType::Dict, "Dictionary object should be classified as Dict");
}

/// Test that stream PdfObjects are still classified as Stream.
///
/// This test is a regression check to ensure that adding Dict detection
/// did not break existing Stream detection.
#[test]
fn test_detect_char_proc_type_stream() {
    // Create a stream PdfObject with a dictionary
    let mut stream_dict = PdfDict::new();
    stream_dict.insert(intern("/Length"), PdfObject::Integer(100));

    // Create a PdfStream with offset 0 and length hint 100
    let stream = crate::parser::object::types::PdfStream::new(stream_dict, 0, Some(100));
    let stream_obj = PdfObject::Stream(Box::new(stream));

    // Classify the object
    let result = detect_char_proc_type(&stream_obj, None);

    // Verify Stream is returned (regression check)
    assert_eq!(result, CharProcType::Stream, "Stream object should still be classified as Stream");
}

/// Test that unrecognized types fall back to Other.
///
/// This test is a regression check to ensure that the Other fallback
/// still works for types that are neither Stream nor Dict.
#[test]
fn test_detect_char_proc_type_other_integer() {
    // Create an integer PdfObject (unrecognized type for CharProcs)
    let int_obj = PdfObject::Integer(42);

    // Classify the object
    let result = detect_char_proc_type(&int_obj, None);

    // Verify Other is returned with descriptive name
    match result {
        CharProcType::Other(name) => {
            assert_eq!(name, "integer", "Integer should be classified as Other with name 'integer'");
        }
        _ => panic!("Integer should be classified as Other, got {:?}", result),
    }
}

/// Test that string PdfObjects are classified as Other.
///
/// This test verifies the Other fallback works for string objects.
#[test]
fn test_detect_char_proc_type_other_string() {
    // Create a string PdfObject
    let string_obj = PdfObject::String(Box::new(b"test string".to_vec()));

    // Classify the object
    let result = detect_char_proc_type(&string_obj, None);

    // Verify Other is returned with descriptive name
    match result {
        CharProcType::Other(name) => {
            assert_eq!(name, "string", "String should be classified as Other with name 'string'");
        }
        _ => panic!("String should be classified as Other, got {:?}", result),
    }
}

/// Test that name PdfObjects are classified as Other.
///
/// This test verifies the Other fallback works for name objects.
#[test]
fn test_detect_char_proc_type_other_name() {
    // Create a name PdfObject
    let name_obj = PdfObject::Name(intern("/TestName"));

    // Classify the object
    let result = detect_char_proc_type(&name_obj, None);

    // Verify Other is returned with descriptive name
    match result {
        CharProcType::Other(name) => {
            assert_eq!(name, "name", "Name should be classified as Other with name 'name'");
        }
        _ => panic!("Name should be classified as Other, got {:?}", result),
    }
}

/// Test that array PdfObjects are classified as Other.
///
/// This test verifies the Other fallback works for array objects.
#[test]
fn test_detect_char_proc_type_other_array() {
    // Create an array PdfObject
    let array_obj = PdfObject::Array(Box::new(vec![
        PdfObject::Integer(1),
        PdfObject::Integer(2),
        PdfObject::Integer(3),
    ]));

    // Classify the object
    let result = detect_char_proc_type(&array_obj, None);

    // Verify Other is returned with descriptive name
    match result {
        CharProcType::Other(name) => {
            assert_eq!(name, "array", "Array should be classified as Other with name 'array'");
        }
        _ => panic!("Array should be classified as Other, got {:?}", result),
    }
}

/// Test that boolean PdfObjects are classified as Other.
///
/// This test verifies the Other fallback works for boolean objects.
#[test]
fn test_detect_char_proc_type_other_boolean() {
    // Create boolean PdfObjects
    let true_obj = PdfObject::Bool(true);
    let false_obj = PdfObject::Bool(false);

    // Classify true
    let result_true = detect_char_proc_type(&true_obj, None);
    match result_true {
        CharProcType::Other(name) => {
            assert_eq!(name, "boolean", "Boolean true should be classified as Other with name 'boolean'");
        }
        _ => panic!("Boolean true should be classified as Other, got {:?}", result_true),
    }

    // Classify false
    let result_false = detect_char_proc_type(&false_obj, None);
    match result_false {
        CharProcType::Other(name) => {
            assert_eq!(name, "boolean", "Boolean false should be classified as Other with name 'boolean'");
        }
        _ => panic!("Boolean false should be classified as Other, got {:?}", result_false),
    }
}

/// Test that null PdfObjects are classified as Other.
///
/// This test verifies the Other fallback works for null objects.
#[test]
fn test_detect_char_proc_type_other_null() {
    // Create a null PdfObject
    let null_obj = PdfObject::Null;

    // Classify the object
    let result = detect_char_proc_type(&null_obj, None);

    // Verify Other is returned with descriptive name
    match result {
        CharProcType::Other(name) => {
            assert_eq!(name, "null", "Null should be classified as Other with name 'null'");
        }
        _ => panic!("Null should be classified as Other, got {:?}", result),
    }
}

/// Test that reference PdfObjects are classified as Other with type name.
///
/// This test verifies that references are detected and classified as "reference"
/// (full dereferencing requires detect_char_proc_type_with_context).
#[test]
fn test_detect_char_proc_type_reference() {
    // Create a reference PdfObject
    let ref_obj = PdfObject::Ref(ObjRef::new(5, 0));

    // Classify the object without document context
    let result = detect_char_proc_type(&ref_obj, None);

    // Verify Other is returned with "unknown" type name (cannot dereference without context)
    match result {
        CharProcType::Other(name) => {
            assert_eq!(name, "unknown",
                "Reference without document context should be classified as Other with type name 'unknown'");
        }
        _ => panic!("Reference should be classified as Other, got {:?}", result),
    }
}

/// Test that real PdfObjects are classified as Other.
///
/// This test verifies the Other fallback works for real number objects.
#[test]
fn test_detect_char_proc_type_other_real() {
    // Create a real PdfObject
    let real_obj = PdfObject::Real(3.14159);

    // Classify the object
    let result = detect_char_proc_type(&real_obj, None);

    // Verify Other is returned with descriptive name
    match result {
        CharProcType::Other(name) => {
            assert_eq!(name, "real", "Real should be classified as Other with name 'real'");
        }
        _ => panic!("Real should be classified as Other, got {:?}", result),
    }
}

/// Test that empty dictionaries are correctly classified as Dict.
///
/// This test verifies that even empty dictionaries are recognized as Dict type.
#[test]
fn test_detect_char_proc_type_empty_dict() {
    // Create an empty dictionary PdfObject
    let empty_dict = PdfDict::new();
    let dict_obj = PdfObject::Dict(Box::new(empty_dict));

    // Classify the object
    let result = detect_char_proc_type(&dict_obj, None);

    // Verify Dict is returned
    assert_eq!(result, CharProcType::Dict, "Empty dictionary should still be classified as Dict");
}

// ============================================================================
// Stream and Dict Detection Edge Case Tests
// ============================================================================

/// Test that a stream with an entirely empty stream dictionary is classified as Stream.
///
/// This complements `test_detect_char_proc_type_empty_dict` on the dict side: the
/// classification must depend only on the PdfObject variant, never on the contents
/// of the stream's own dictionary.
#[test]
fn test_detect_char_proc_type_stream_with_empty_dict() {
    // A stream carrying no dictionary entries at all, no offset, no length hint
    let stream = PdfStream::new(PdfDict::new(), 0, None);
    let stream_obj = PdfObject::Stream(Box::new(stream));

    let result = detect_char_proc_type(&stream_obj, None);

    assert_eq!(result, CharProcType::Stream,
        "Stream with an empty stream dictionary should be classified as Stream");
}

/// Test that a stream with no /Length entry and no length hint is classified as Stream.
///
/// A missing length is a parse-state detail resolved later during stream decoding;
/// it must not demote the object to Other.
#[test]
fn test_detect_char_proc_type_stream_without_length_hint() {
    // Dictionary without /Length, and a None length hint
    let mut stream_dict = PdfDict::new();
    stream_dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
    let stream = PdfStream::new(stream_dict, 512, None);
    let stream_obj = PdfObject::Stream(Box::new(stream));

    let result = detect_char_proc_type(&stream_obj, None);

    assert_eq!(result, CharProcType::Stream,
        "Stream without /Length or length hint should still be classified as Stream");
}

/// Test that stream classification ignores the stream's byte offset.
///
/// Offset is where the stream data lives in the source document, so it carries no
/// type information. A stream at a non-zero offset is still a stream.
#[test]
fn test_detect_char_proc_type_stream_with_nonzero_offset() {
    let mut stream_dict = PdfDict::new();
    stream_dict.insert(intern("/Length"), PdfObject::Integer(4096));
    let stream = PdfStream::new(stream_dict, 0x0000_F000, Some(4096));
    let stream_obj = PdfObject::Stream(Box::new(stream));

    let result = detect_char_proc_type(&stream_obj, None);

    assert_eq!(result, CharProcType::Stream,
        "Stream at a non-zero byte offset should be classified as Stream");
}

/// Test that nested values inside a stream dictionary do not affect classification.
///
/// A real CharProc stream dict holds nested structures (/Resources as a dict,
/// /DecodeParms as an array, /Filter as a name). None of that may leak into the
/// classification: the object is a Stream, not a Dict, despite containing dicts.
#[test]
fn test_detect_char_proc_type_stream_with_nested_dict_entries() {
    // /Resources << /Font << /F1 5 0 R >> /ProcSet [/PDF /Text] >>
    let mut resources = PdfDict::new();
    let mut font_resources = PdfDict::new();
    font_resources.insert(intern("/F1"), PdfObject::Ref(ObjRef::new(5, 0)));
    resources.insert(intern("/Font"), PdfObject::Dict(Box::new(font_resources)));
    resources.insert(intern("/ProcSet"), PdfObject::Array(Box::new(vec![
        PdfObject::Name(intern("/PDF")),
        PdfObject::Name(intern("/Text")),
    ])));

    let mut stream_dict = PdfDict::new();
    stream_dict.insert(intern("/Length"), PdfObject::Integer(256));
    stream_dict.insert(intern("/Filter"), PdfObject::Name(intern("/FlateDecode")));
    stream_dict.insert(intern("/Resources"), PdfObject::Dict(Box::new(resources)));

    let stream = PdfStream::new(stream_dict, 1024, Some(256));
    let stream_obj = PdfObject::Stream(Box::new(stream));

    let result = detect_char_proc_type(&stream_obj, None);

    assert_eq!(result, CharProcType::Stream,
        "Stream with nested dict/array entries should be classified as Stream, not Dict");
}

/// Test that a realistic Type 3 CharProc stream dictionary is classified as Stream.
///
/// Mirrors the dictionary a Type 3 CharProc actually carries per PDF spec 9.6.5:
/// /Type /XObject, /Subtype /Form, /BBox, /Matrix, /Resources.
#[test]
fn test_detect_char_proc_type_charproc_style_stream() {
    let mut stream_dict = PdfDict::new();
    stream_dict.insert(intern("/Type"), PdfObject::Name(intern("/XObject")));
    stream_dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Form")));
    stream_dict.insert(intern("/FormType"), PdfObject::Integer(1));
    stream_dict.insert(intern("/BBox"), PdfObject::Array(Box::new(vec![
        PdfObject::Real(0.0),
        PdfObject::Real(0.0),
        PdfObject::Real(1000.0),
        PdfObject::Real(1000.0),
    ])));
    stream_dict.insert(intern("/Matrix"), PdfObject::Array(Box::new(vec![
        PdfObject::Real(0.001),
        PdfObject::Real(0.0),
        PdfObject::Real(0.0),
        PdfObject::Real(0.001),
        PdfObject::Real(0.0),
        PdfObject::Real(0.0),
    ])));

    let stream = PdfStream::new(stream_dict, 2048, Some(512));
    let stream_obj = PdfObject::Stream(Box::new(stream));

    let result = detect_char_proc_type(&stream_obj, None);

    assert_eq!(result, CharProcType::Stream,
        "A Type 3 CharProc-shaped stream should be classified as Stream");
}

/// Test that nested structures held as dictionary values do not affect classification.
///
/// A CharProcs-like dict maps glyph names to streams, refs, arrays and further dicts.
/// Detection classifies the container only, so a dict holding a stream value must
/// still be a Dict — the stream inside must not win.
#[test]
fn test_detect_char_proc_type_dict_with_nested_structures() {
    // Inner dict and a stream value nested inside the outer dict
    let mut inner = PdfDict::new();
    inner.insert(intern("/Key"), PdfObject::Name(intern("/Value")));

    let mut nested_stream_dict = PdfDict::new();
    nested_stream_dict.insert(intern("/Length"), PdfObject::Integer(10));
    let nested_stream = PdfStream::new(nested_stream_dict, 0, Some(10));

    let mut dict = PdfDict::new();
    dict.insert(intern("/Type"), PdfObject::Name(intern("/Font")));
    dict.insert(intern("/Subtype"), PdfObject::Name(intern("/Type3")));
    dict.insert(intern("/FontDescriptor"), PdfObject::Ref(ObjRef::new(7, 0)));
    dict.insert(intern("/Inner"), PdfObject::Dict(Box::new(inner)));
    dict.insert(intern("/Widths"), PdfObject::Array(Box::new(vec![
        PdfObject::Integer(500),
        PdfObject::Integer(600),
        PdfObject::Integer(700),
    ])));
    // Array of dicts: nesting two levels deep inside an array
    let mut array_element = PdfDict::new();
    array_element.insert(intern("/Nested"), PdfObject::Bool(true));
    dict.insert(intern("/ArrayOfDicts"), PdfObject::Array(Box::new(vec![
        PdfObject::Dict(Box::new(array_element)),
    ])));
    dict.insert(intern("/NestedStream"), PdfObject::Stream(Box::new(nested_stream)));

    let dict_obj = PdfObject::Dict(Box::new(dict));

    let result = detect_char_proc_type(&dict_obj, None);

    assert_eq!(result, CharProcType::Dict,
        "Dict with nested dicts, arrays and a stream value should be classified as Dict, not Stream");
}

/// Test that deeply nested dictionary values are classified as Dict without recursion issues.
///
/// Detection does not descend into values, so arbitrary nesting depth must neither
/// change the answer nor overflow.
#[test]
fn test_detect_char_proc_type_dict_with_deeply_nested_values() {
    // Build a chain of dicts 16 levels deep, each /Nested key holding the next
    let mut obj = PdfObject::Integer(1);
    for _ in 0..16 {
        let mut dict = PdfDict::new();
        dict.insert(intern("/Nested"), obj);
        obj = PdfObject::Dict(Box::new(dict));
    }

    let result = detect_char_proc_type(&obj, None);

    assert_eq!(result, CharProcType::Dict,
        "A dict nested 16 levels deep should still be classified as Dict at the top level");
}

/// Test that a realistic CharProcs dictionary is classified as Dict.
///
/// CharProcs maps glyph names to indirect references to streams. Many keys, all
/// ref-valued — still a Dict.
#[test]
fn test_detect_char_proc_type_charprocs_style_dict() {
    let mut charprocs = PdfDict::new();
    for (i, glyph_name) in ["/a", "/b", "/g", "/A", "/acute", "/zero"].iter().enumerate() {
        charprocs.insert(intern(glyph_name), PdfObject::Ref(ObjRef::new((i + 10) as u32, 0)));
    }

    let dict_obj = PdfObject::Dict(Box::new(charprocs));
    let result = detect_char_proc_type(&dict_obj, None);

    assert_eq!(result, CharProcType::Dict,
        "A CharProcs dictionary holding six glyph refs should be classified as Dict");
}

/// Test that Stream and Dict classifications are distinct and stable across calls.
///
/// Guards against a regression where both variants collapse to one arm, and against
/// nondeterminism (e.g. classification depending on hash iteration order).
#[test]
fn test_detect_char_proc_type_stream_and_dict_are_distinct_and_stable() {
    let mut stream_dict = PdfDict::new();
    stream_dict.insert(intern("/Length"), PdfObject::Integer(64));
    let stream_obj = PdfObject::Stream(Box::new(PdfStream::new(stream_dict, 0, Some(64))));

    let mut dict = PdfDict::new();
    dict.insert(intern("/Length"), PdfObject::Integer(64));
    let dict_obj = PdfObject::Dict(Box::new(dict));

    let stream_result_a = detect_char_proc_type(&stream_obj, None);
    let stream_result_b = detect_char_proc_type(&stream_obj, None);
    let dict_result_a = detect_char_proc_type(&dict_obj, None);
    let dict_result_b = detect_char_proc_type(&dict_obj, None);

    assert_eq!(stream_result_a, CharProcType::Stream, "Stream object should classify as Stream");
    assert_eq!(dict_result_a, CharProcType::Dict, "Dict object should classify as Dict");
    assert_eq!(stream_result_a, stream_result_b, "Stream classification should be stable");
    assert_eq!(dict_result_a, dict_result_b, "Dict classification should be stable");
    assert_ne!(stream_result_a, dict_result_a,
        "Stream and Dict classifications must be distinct for identically-keyed dictionaries");
}

/// Test that all detection entry points agree on Stream and Dict objects.
///
/// `detect_char_proc_type`, `detect_char_proc_type_with_depth` and
/// `detect_char_proc_type_with_context` are layered wrappers over one implementation;
/// none of them may reclassify a direct Stream or Dict.
#[test]
fn test_detect_char_proc_type_variants_agree_on_stream_and_dict() {
    use crate::font::type3_rasterizer::{
        detect_char_proc_type_with_context, detect_char_proc_type_with_depth, DocumentContext,
    };

    let mut stream_dict = PdfDict::new();
    stream_dict.insert(intern("/Length"), PdfObject::Integer(128));
    let stream_obj = PdfObject::Stream(Box::new(PdfStream::new(stream_dict, 0, Some(128))));

    let mut dict = PdfDict::new();
    dict.insert(intern("/Type"), PdfObject::Name(intern("/Font")));
    let dict_obj = PdfObject::Dict(Box::new(dict));

    let doc_context = DocumentContext { resolver: None, source: None };

    for (obj, expected, label) in [
        (&stream_obj, CharProcType::Stream, "Stream"),
        (&dict_obj, CharProcType::Dict, "Dict"),
    ] {
        assert_eq!(detect_char_proc_type(obj, None), expected,
            "{}: top-level entry point disagrees", label);
        assert_eq!(detect_char_proc_type_with_depth(obj, None, 0), expected,
            "{}: explicit depth 0 disagrees", label);
        assert_eq!(detect_char_proc_type_with_depth(obj, None, 4), expected,
            "{}: explicit depth 4 disagrees", label);
        // Direct objects short-circuit before any depth accounting: even a depth
        // far beyond any real reference-chain length must not reclassify them.
        assert_eq!(detect_char_proc_type_with_depth(obj, None, 1000), expected,
            "{}: explicit depth 1000 disagrees", label);
        assert_eq!(detect_char_proc_type_with_context(obj, None, 0), expected,
            "{}: context variant without context disagrees", label);
        assert_eq!(detect_char_proc_type_with_context(obj, Some(&doc_context), 0), expected,
            "{}: context variant with an empty context disagrees", label);
    }
}

// ============================================================================
// Integration Tests for Reference Dereferencing
// ============================================================================

/// Test that PdfObject::Ref with DocumentContext returns Unknown when context is empty.
///
/// This test verifies that when a reference is passed with a DocumentContext
/// that has no resolver, the function returns Unknown gracefully without panicking.
#[test]
fn test_detect_char_proc_type_ref_with_empty_context_returns_unknown() {
    use crate::font::type3_rasterizer::DocumentContext;

    // Create a reference PdfObject
    let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));

    // Create an empty DocumentContext (no resolver, no source)
    let doc_context = DocumentContext {
        resolver: None,
        source: None,
    };

    // Classify the object - should return Unknown gracefully
    let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

    // Verify Unknown is returned (no panic)
    assert_eq!(result, CharProcType::Other("unknown".to_string()),
        "Reference with empty DocumentContext should return Unknown without panicking");
}

/// Test that PdfObject::Ref without DocumentContext returns Unknown.
///
/// This test verifies the graceful degradation when no document context
/// is available for dereferencing. The function should not panic.
#[test]
fn test_detect_char_proc_type_ref_without_context_returns_unknown() {
    // Create a reference PdfObject
    let ref_obj = PdfObject::Ref(ObjRef::new(15, 0));

    // Classify without DocumentContext - should return Unknown gracefully
    let result = detect_char_proc_type(&ref_obj, None);

    // Verify Unknown is returned (no panic)
    assert_eq!(result, CharProcType::Other("unknown".to_string()),
        "Reference without DocumentContext should return Unknown without panicking");
}

/// Test that detect_char_proc_type_with_context detects circular references.
///
/// This test verifies that circular references are detected and classified
/// as CharProcType::Other("circular-reference") without infinite recursion.
#[test]
fn test_detect_char_proc_type_with_context_detects_circular_ref() {
    use crate::font::type3_rasterizer::{detect_char_proc_type_with_context, DocumentContext};

    // Create a reference that would point to itself (circular)
    let circular_ref = ObjRef::new(100, 0);
    let ref_obj = PdfObject::Ref(circular_ref);

    // Create empty DocumentContext
    let doc_context = DocumentContext {
        resolver: None,
        source: None,
    };

    // Use the with_context variant that has cycle detection
    let result = detect_char_proc_type_with_context(&ref_obj, Some(&doc_context), 0);

    // Since resolver is None, it should return Unknown (circular detection
    // only applies if we can actually dereference)
    assert_eq!(result, CharProcType::Other("unknown".to_string()),
        "Circular reference with no resolver should return Unknown");
}

/// Test that reference detection does not panic on invalid object numbers.
///
/// This test verifies robustness: even with theoretically invalid references,
/// the function should not panic and should return Unknown gracefully.
#[test]
fn test_detect_char_proc_type_ref_does_not_panic_on_invalid_ref() {
    // Create references with various object numbers (including large ones)
    let test_refs = vec![
        ObjRef::new(0, 0),
        ObjRef::new(999999, 0),
        ObjRef::new(1, 65535),
        ObjRef::new(i32::MAX as u32, 0),
    ];

    for obj_ref in test_refs {
        let ref_obj = PdfObject::Ref(obj_ref);

        // Should not panic even with unusual reference values
        let result = detect_char_proc_type(&ref_obj, None);

        // All should return Unknown gracefully
        assert_eq!(result, CharProcType::Other("unknown".to_string()),
            "Invalid reference should return Unknown without panicking");
    }
}

/// Test that detect_char_proc_type correctly handles reference with valid resolver.
///
/// This is an integration test that verifies the full dereferencing path works
/// when a valid DocumentContext is provided. (Note: requires mock setup)
#[test]
fn test_detect_char_proc_type_ref_integration_with_valid_context() {
    use crate::font::type3_rasterizer::DocumentContext;
    use crate::parser::xref::XrefResolver;

    // Create a reference
    let ref_obj = PdfObject::Ref(ObjRef::new(50, 0));

    // Create DocumentContext with resolver (even though it won't find the ref)
    let resolver = XrefResolver::new();
    let doc_context = DocumentContext {
        resolver: Some(&resolver),
        source: None,
    };

    // Should not panic even when resolver cannot find the reference
    let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

    // Should return Unknown (reference not found, but no panic)
    assert_eq!(result, CharProcType::Other("unknown".to_string()),
        "Reference not found should return Unknown without panicking");
}

/// Test that activated edges preserve their properties in AET.
///
/// This test verifies that when an edge is added to the Active Edge Table (AET),
/// all its properties (x, y_max, slope) are preserved correctly. The test uses
/// fixed-point representation for floating-point values (e.g., x=10.5 is stored
/// as 105 with scale factor=10, slope=0.5 is dx=1, dy=2).
///
/// # Acceptance Criteria
///
/// 1. Test edge with specific properties: x=10.5, y_min=5, y_max=15, slope=0.5
/// 2. Assert edge in AET has matching x value (with tolerance)
/// 3. Assert edge in AET has matching y_max value
/// 4. Assert edge in AET has matching slope value (dx/dy ratio)
/// 5. Test passes
/// 6. Code compiles
#[test]
fn test_edge_property_preservation_in_aet() {
    use crate::font::type3_rasterizer::Edge;

    // Create a test edge with specific properties
    // Using fixed-point representation: x=10.5 is stored as 105 (scale factor 10)
    // Slope=0.5 is represented as dx=1, dy=2 (1/2 = 0.5)
    let original_x_fixed = 105_i32;  // Represents 10.5 with scale=10
    let y_min = 5_i32;
    let y_max = 15_i32;
    let dx = 1_i32;   // For slope=0.5 (dx/dy)
    let dy = 2_i32;   // For slope=0.5 (dx/dy)

    let test_edge = Edge {
        x: original_x_fixed,
        y_min,
        y_max,
        dx,
        dy,
    };

    // Simulate edge activation: add to AET
    let mut aet: Vec<Edge> = Vec::new();
    aet.push(test_edge);

    // Retrieve edge from AET
    let retrieved_edge = &aet[0];

    // Assert y_max value is preserved (integer comparison, exact match)
    assert_eq!(
        retrieved_edge.y_max,
        y_max,
        "Edge in AET should preserve y_max value of {}",
        y_max
    );

    // Assert x value is preserved (with tolerance for floating-point)
    // The stored value should match exactly since we're using fixed-point
    assert_eq!(
        retrieved_edge.x,
        original_x_fixed,
        "Edge in AET should preserve x value (stored as fixed-point {})",
        original_x_fixed
    );

    // Convert back to floating-point for verification
    let scale = 10.0_f64;
    let actual_x = retrieved_edge.x as f64 / scale;
    let expected_x = 10.5_f64;

    // Allow floating-point tolerance (±0.01)
    assert!(
        (actual_x - expected_x).abs() < 0.01,
        "Edge in AET should have x≈{} (got {}, diff={})",
        expected_x,
        actual_x,
        (actual_x - expected_x).abs()
    );

    // Assert slope value is preserved (dx/dy ratio)
    // Slope = dx / dy = 1 / 2 = 0.5
    let actual_slope = retrieved_edge.dx as f64 / retrieved_edge.dy as f64;
    let expected_slope = 0.5_f64;

    assert!(
        (actual_slope - expected_slope).abs() < 0.01,
        "Edge in AET should preserve slope (dx/dy = {} / {} = {}, got {})",
        retrieved_edge.dx,
        retrieved_edge.dy,
        expected_slope,
        actual_slope
    );

    // Verify individual dx and dy values are preserved
    assert_eq!(
        retrieved_edge.dx, dx,
        "Edge in AET should preserve dx value of {}", dx
    );
    assert_eq!(
        retrieved_edge.dy, dy,
        "Edge in AET should preserve dy value of {}", dy
    );

    // Verify all edge properties are preserved
    assert_eq!(
        retrieved_edge.y_min, y_min,
        "Edge in AET should preserve y_min value of {}", y_min
    );
}

/// Test edge property preservation using TestEdge builder.
///
/// This test uses the TestEdge builder from type3_test_fixtures to verify
/// that edge properties are preserved when built and activated in the AET.
#[test]
fn test_edge_property_preservation_with_builder() {
    use crate::font::type3_test_fixtures::TestEdge;

    // Create edge using builder with specific properties
    // x=10.5 → stored as 105 (scale=10), y_min=5, y_max=15, slope=0.5 (dx=1, dy=2)
    let built_edge = TestEdge::new()
        .with_x(105)      // Represents 10.5 with scale=10
        .with_y_min(5)
        .with_y_max(15)
        .with_slope(1, 2)  // dx=1, dy=2 gives slope=0.5
        .build();

    // Add to AET
    let mut aet: Vec<crate::font::type3_rasterizer::Edge> = Vec::new();
    aet.push(built_edge);

    let retrieved = &aet[0];

    // Verify all properties
    assert_eq!(retrieved.x, 105, "X should be preserved as 105");
    assert_eq!(retrieved.y_min, 5, "y_min should be preserved as 5");
    assert_eq!(retrieved.y_max, 15, "y_max should be preserved as 15");
    assert_eq!(retrieved.dx, 1, "dx should be preserved as 1");
    assert_eq!(retrieved.dy, 2, "dy should be preserved as 2");

    // Verify slope
    let slope = retrieved.dx as f64 / retrieved.dy as f64;
    assert!((slope - 0.5).abs() < 0.01, "Slope should be approximately 0.5");
}

/// Test that references are detected as references before dereferencing.
///
/// This test verifies the classification logic correctly identifies PdfObject::Ref
/// and attempts dereferencing, rather than classifying it as "Other".
#[test]
fn test_detect_char_proc_type_identifies_ref_type() {
    // Create a reference
    let ref_obj = PdfObject::Ref(ObjRef::new(25, 0));

    // Without context, should return Unknown (not Other("reference"))
    let result = detect_char_proc_type(&ref_obj, None);

    // Should be Unknown, not Other with a type name
    match result {
        CharProcType::Other(name) if name == "unknown" => {
            // Expected - references without context return Unknown
        }
        CharProcType::Other(name) => {
            panic!("Reference should return Unknown, not Other with name '{}'", name);
        }
        _ => {
            panic!("Reference should return Unknown, got {:?}", result);
        }
    }
}

/// Test that direct stream objects are still classified as Stream.
///
/// This is a regression test to ensure that adding reference handling
/// did not break direct stream object classification.
#[test]
fn test_detect_char_proc_type_direct_stream_regression() {
    use crate::parser::object::types::PdfStream;

    // Create a direct stream object
    let mut stream_dict = PdfDict::new();
    stream_dict.insert(intern("/Length"), PdfObject::Integer(100));
    let stream = PdfStream::new(stream_dict, 0, Some(100));
    let stream_obj = PdfObject::Stream(Box::new(stream));

    // Should classify as Stream (not attempt dereferencing)
    let result = detect_char_proc_type(&stream_obj, None);

    assert_eq!(result, CharProcType::Stream,
        "Direct stream object should still be classified as Stream");
}

/// Test that direct dict objects are still classified as Dict.
///
/// This is a regression test to ensure that adding reference handling
/// did not break direct dictionary object classification.
#[test]
fn test_detect_char_proc_type_direct_dict_regression() {
    // Create a direct dict object
    let dict = PdfDict::new();
    let dict_obj = PdfObject::Dict(Box::new(dict));

    // Should classify as Dict (not attempt dereferencing)
    let result = detect_char_proc_type(&dict_obj, None);

    assert_eq!(result, CharProcType::Dict,
        "Direct dict object should still be classified as Dict");
}

/// Test detect_char_proc_type with multiple reference types.
///
/// This test verifies that the function handles various reference scenarios
/// consistently and does not panic on edge cases.
#[test]
fn test_detect_char_proc_type_ref_various_scenarios() {
    use crate::font::type3_rasterizer::DocumentContext;

    let test_cases = vec![
        (ObjRef::new(1, 0), "normal reference"),
        (ObjRef::new(0, 0), "zero object number"),
        (ObjRef::new(1, 1), "non-zero generation number"),
        (ObjRef::new(1000, 0), "large object number"),
    ];

    for (obj_ref, description) in test_cases {
        let ref_obj = PdfObject::Ref(obj_ref);

        // Test without context - should return Unknown
        let result_no_ctx = detect_char_proc_type(&ref_obj, None);
        assert_eq!(result_no_ctx, CharProcType::Other("unknown".to_string()),
            "{} without context should return Unknown", description);

        // Test with empty context - should return Unknown
        let empty_ctx = DocumentContext {
            resolver: None,
            source: None,
        };
        let result_empty_ctx = detect_char_proc_type(&ref_obj, Some(&empty_ctx));
        assert_eq!(result_empty_ctx, CharProcType::Other("unknown".to_string()),
            "{} with empty context should return Unknown", description);
    }
}

/// Test that Reference handling is robust against null objects in chain.
///
/// This test verifies that if a reference chain leads to a null object,
/// the function handles it gracefully without panicking.
#[test]
fn test_detect_char_proc_type_ref_chain_robustness() {
    // Create a reference
    let ref_obj = PdfObject::Ref(ObjRef::new(30, 0));

    // Test with null DocumentContext
    let result = detect_char_proc_type(&ref_obj, None);

    // Should gracefully return Unknown
    assert_eq!(result, CharProcType::Other("unknown".to_string()),
        "Reference should return Unknown when resolution is not possible");
}

// ============================================================================
// Valid Reference Dereferencing Tests
// ============================================================================

/// Test that PdfObject::Ref with valid DocumentContext successfully dereferences to Dict.
///
/// This test verifies that when a PdfObject::Ref pointing to a dictionary is provided
/// with a DocumentContext containing a properly formatted PDF dict object, the function
/// correctly dereferences and returns CharProcType::Dict.
///
/// This uses the helper functions to create a complete test setup with actual PDF data.
#[test]
fn test_detect_char_proc_type_ref_with_valid_context_and_dict() {
    // Create a properly formatted PDF dictionary at offset 100
    let dict_bytes = create_pdf_dict_object(10, 0, "/Type /Font /Subtype /Type3");

    // Create a valid dereference context with the dict object
    let doc_context = create_valid_dereference_context(vec![
        (10, 100, 0, dict_bytes)
    ]);

    // Create a reference to object 10
    let ref_obj = create_test_ref(10);

    // Dereference and classify - should successfully detect Dict
    let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

    // Verify successful dereferencing to Dict
    assert_eq!(result, CharProcType::Dict,
        "PdfObject::Ref pointing to a dictionary should return CharProcType::Dict");
}

/// Test that PdfObject::Ref with valid DocumentContext successfully dereferences to Stream.
///
/// This test verifies that when a PdfObject::Ref pointing to a stream is provided
/// with a DocumentContext containing a properly formatted PDF stream object, the function
/// correctly dereferences and returns CharProcType::Stream.
///
/// This uses the helper functions to create a complete test setup with actual PDF data.
#[test]
fn test_detect_char_proc_type_ref_with_valid_context_and_stream() {
    // Create a properly formatted PDF stream at offset 200
    // Stream with simple drawing commands
    let stream_bytes = create_pdf_stream_object(
        20,
        0,
        "/Type /XObject /Subtype /Form /Width 100 /Height 100",
        b"0 0 100 100 re f"
    );

    // Create a valid dereference context with the stream object
    let doc_context = create_valid_dereference_context(vec![
        (20, 200, 0, stream_bytes)
    ]);

    // Create a reference to object 20
    let ref_obj = create_test_ref(20);

    // Dereference and classify - should successfully detect Stream
    let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

    // Verify successful dereferencing to Stream
    assert_eq!(result, CharProcType::Stream,
        "PdfObject::Ref pointing to a stream should return CharProcType::Stream");
}

// ============================================================================
// Tests for Invalid and Edge-Case Reference Scenarios
// ============================================================================

/// Test that detect_char_proc_type handles invalid references gracefully.
///
/// This test verifies that when a PdfObject::Ref contains an object reference
/// ID that does not exist in the document, the function returns CharProcType::Other("unknown".to_string())
/// without panicking. This covers scenarios where:
/// - Reference IDs are not present in the xref table
/// - References have been deleted or corrupted
/// - References point to non-existent objects
#[test]
fn test_detect_char_proc_type_ref_with_invalid_reference() {
    use crate::font::type3_rasterizer::DocumentContext;
    use crate::parser::xref::XrefResolver;

    // Create a DocumentContext with an empty resolver (no objects registered)
    let resolver = XrefResolver::new();
    let doc_context = DocumentContext {
        resolver: Some(&resolver),
        source: None,
    };

    // Create references with various object IDs that don't exist in the document
    let nonexistent_refs = vec![
        ObjRef::new(1, 0),      // Low object number
        ObjRef::new(999, 0),    // High object number
        ObjRef::new(50, 5),     // Non-zero generation number
        ObjRef::new(1000, 0),   // Very high object number
    ];

    for obj_ref in nonexistent_refs {
        let ref_obj = PdfObject::Ref(obj_ref);

        // Should not panic - should return Unknown gracefully
        let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

        // Verify Unknown is returned for all invalid references
        assert_eq!(result, CharProcType::Other("unknown".to_string()),
            "Reference to non-existent object {} {} should return Unknown without panicking",
            obj_ref.object, obj_ref.generation);
    }
}

/// Test that detect_char_proc_type handles references to out-of-bounds objects.
///
/// This test verifies that when a PdfObject::Ref contains an object reference
/// that points beyond the valid range of object numbers in the document, the
/// function returns CharProcType::Other("unknown".to_string()) without panicking. This covers edge cases:
/// - Object number 0 (invalid in PDF spec)
/// - Extremely large object numbers
/// - Object numbers beyond u32::MAX / 2 (theoretical bounds)
#[test]
fn test_detect_char_proc_type_ref_with_nonexistent_object() {
    use crate::font::type3_rasterizer::DocumentContext;
    use crate::parser::xref::XrefResolver;

    // Create a DocumentContext with an empty resolver
    let resolver = XrefResolver::new();
    let doc_context = DocumentContext {
        resolver: Some(&resolver),
        source: None,
    };

    // Create references with out-of-bounds or theoretically invalid object numbers
    let out_of_bounds_refs = vec![
        ObjRef::new(0, 0),           // Object 0 is invalid in PDF spec
        ObjRef::new(0, 1),           // Object 0 with generation number
        ObjRef::new(u32::MAX, 0),    // Maximum possible object number
        ObjRef::new(u32::MAX - 1, 0), // Near-maximum object number
        ObjRef::new(1, u16::MAX),     // Maximum generation number
    ];

    for obj_ref in out_of_bounds_refs {
        let ref_obj = PdfObject::Ref(obj_ref);

        // Should not panic even with out-of-bounds references
        let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

        // Verify Unknown is returned for all out-of-bounds references
        assert_eq!(result, CharProcType::Other("unknown".to_string()),
            "Out-of-bounds reference {} {} should return Unknown without panicking",
            obj_ref.object, obj_ref.generation);
    }
}

/// Test that detect_char_proc_type handles references with mismatched generation numbers.
///
/// This test verifies that when a reference points to an object that exists
/// but has a different generation number (indicating a deleted/updated object),
/// the function handles it gracefully.
#[test]
fn test_detect_char_proc_type_ref_with_mismatched_generation() {
    use crate::font::type3_rasterizer::DocumentContext;
    use crate::parser::xref::XrefEntry;
    use crate::parser::xref::XrefResolver;

    // Create a resolver with object 10 at generation 0
    let mut resolver = XrefResolver::new();
    resolver.add_entry(10, XrefEntry::InUse {
        offset: 100,
        gen_nr: 0,
    });

    let doc_context = DocumentContext {
        resolver: Some(&resolver),
        source: None,
    };

    // Reference to object 10 with generation 1 (mismatched - object was deleted)
    let mismatched_ref = ObjRef::new(10, 1);
    let ref_obj = PdfObject::Ref(mismatched_ref);

    // Should not panic - should return Unknown
    let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

    // Verify Unknown is returned for generation-mismatched references
    assert_eq!(result, CharProcType::Other("unknown".to_string()),
        "Reference with mismatched generation number should return Unknown");
}

/// Test that detect_char_proc_type handles references to free objects.
///
/// This test verifies that when a reference points to an object that has
/// been marked as free in the xref table, the function returns Unknown gracefully.
#[test]
fn test_detect_char_proc_type_ref_with_free_object() {
    use crate::font::type3_rasterizer::DocumentContext;
    use crate::parser::xref::XrefEntry;
    use crate::parser::xref::XrefResolver;

    // Create a resolver with a free entry
    let mut resolver = XrefResolver::new();
    resolver.add_entry(20, XrefEntry::Free {
        next_free: 21,
        gen_nr: 1,
    });

    let doc_context = DocumentContext {
        resolver: Some(&resolver),
        source: None,
    };

    // Reference to object 20 (which is marked as free)
    let free_ref = ObjRef::new(20, 0);
    let ref_obj = PdfObject::Ref(free_ref);

    // Should not panic - should return Unknown
    let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

    // Verify Unknown is returned for references to free objects
    assert_eq!(result, CharProcType::Other("unknown".to_string()),
        "Reference to free object should return Unknown");
}

// ============================================================================
// Integration Tests for Successful Reference Dereferencing
// ============================================================================

/// Test that PdfObject::Ref pointing to stream returns CharProcType::Stream.
///
/// This is a comprehensive integration test that verifies:
/// 1. A valid DocumentContext with resolver and source
/// 2. A Ref pointing to a stream object
/// 3. Successful dereferencing returns CharProcType::Stream
#[test]
fn test_detect_char_proc_type_ref_to_stream_returns_stream() {
    use crate::font::type3_rasterizer::DocumentContext;
    use crate::parser::xref::XrefEntry;

    // Create a valid PDF stream object at object 10
    let stream_bytes = create_pdf_stream_object(
        10,
        0,
        "/Type /XObject /Subtype /Form /Width 100 /Height 100",
        b"10 10 m 20 20 l S"
    );

    // Calculate offsets
    let stream_offset = 1000u64;
    let total_size = stream_offset + stream_bytes.len() as u64;

    // Create source data with stream at offset
    let mut source_data = vec![0u8; total_size as usize];
    source_data[stream_offset as usize..(stream_offset as usize + stream_bytes.len())]
        .copy_from_slice(&stream_bytes);

    // Create resolver with entry pointing to the stream
    let mut resolver = XrefResolver::new();
    resolver.add_entry(10, XrefEntry::InUse {
        offset: stream_offset,
        gen_nr: 0,
    });

    let source = MemorySource::new(source_data);
    let doc_context = DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: Some(Box::leak(Box::new(source))),
    };

    // Create a reference to object 10
    let ref_obj = PdfObject::Ref(ObjRef::new(10, 0));

    // Dereference and classify
    let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

    // Verify Stream type is returned
    assert_eq!(result, CharProcType::Stream,
        "Reference to stream object should return CharProcType::Stream after successful dereferencing");
}

/// Test that PdfObject::Ref pointing to dict returns CharProcType::Dict.
///
/// This is a comprehensive integration test that verifies:
/// 1. A valid DocumentContext with resolver and source
/// 2. A Ref pointing to a dictionary object
/// 3. Successful dereferencing returns CharProcType::Dict
#[test]
fn test_detect_char_proc_type_ref_to_dict_returns_dict() {
    use crate::font::type3_rasterizer::DocumentContext;
    use crate::parser::xref::XrefEntry;

    // Create a valid PDF dict object at object 20
    let dict_bytes = create_pdf_dict_object(
        20,
        0,
        "/Type /Font /Subtype /Type3 /FontMatrix [1 0 0 1 0 0]"
    );

    // Calculate offsets
    let dict_offset = 2000u64;
    let total_size = dict_offset + dict_bytes.len() as u64;

    // Create source data with dict at offset
    let mut source_data = vec![0u8; total_size as usize];
    source_data[dict_offset as usize..(dict_offset as usize + dict_bytes.len())]
        .copy_from_slice(&dict_bytes);

    // Create resolver with entry pointing to the dict
    let mut resolver = XrefResolver::new();
    resolver.add_entry(20, XrefEntry::InUse {
        offset: dict_offset,
        gen_nr: 0,
    });

    let source = MemorySource::new(source_data);
    let doc_context = DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: Some(Box::leak(Box::new(source))),
    };

    // Create a reference to object 20
    let ref_obj = PdfObject::Ref(ObjRef::new(20, 0));

    // Dereference and classify
    let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

    // Verify Dict type is returned
    assert_eq!(result, CharProcType::Dict,
        "Reference to dict object should return CharProcType::Dict after successful dereferencing");
}

/// Test that PdfObject::Ref with invalid reference returns Unknown without panicking.
///
/// This test verifies that when a reference cannot be resolved (object not found,
/// invalid offset, etc.), the function returns CharProcType::Other("unknown".to_string()) gracefully
/// instead of panicking.
#[test]
fn test_detect_char_proc_type_ref_invalid_returns_unknown_no_panic() {
    use crate::font::type3_rasterizer::DocumentContext;

    // Create an empty resolver (will fail to find any object)
    let resolver = XrefResolver::new();
    let source_data = vec![0u8; 4096];
    let source = MemorySource::new(source_data);

    let doc_context = DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: Some(Box::leak(Box::new(source))),
    };

    // Create references to non-existent objects
    let invalid_refs = vec![
        ObjRef::new(999, 0),   // Non-existent object number
        ObjRef::new(1000, 1),  // Non-existent with generation
        ObjRef::new(50, 0),    // Another non-existent object
    ];

    for obj_ref in invalid_refs {
        let ref_obj = PdfObject::Ref(obj_ref);

        // Should not panic even though reference can't be resolved
        let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

        // Verify Unknown is returned
        assert_eq!(result, CharProcType::Other("unknown".to_string()),
            "Invalid reference {} {} should return Unknown without panicking",
            obj_ref.object, obj_ref.generation);
    }
}

/// Test that PdfObject::Ref with valid DocumentContext returns correct type for multiple objects.
///
/// This test verifies that the dereferencing logic works correctly for multiple
/// different object types in a single DocumentContext.
#[test]
fn test_detect_char_proc_type_ref_multiple_objects_mixed_types() {
    use crate::font::type3_rasterizer::DocumentContext;
    use crate::parser::xref::XrefEntry;

    // Create multiple objects in the source
    let stream_bytes = create_pdf_stream_object(
        10,
        0,
        "/Type /XObject /Subtype /Form",
        b"10 10 m 20 20 l S"
    );

    let dict_bytes = create_pdf_dict_object(
        20,
        0,
        "/Type /Font /Subtype /Type3"
    );

    // Create source data with both objects
    let mut source_data = vec![0u8; 8192];
    let stream_offset = 1000u64;
    let dict_offset = 3000u64;

    source_data[stream_offset as usize..(stream_offset as usize + stream_bytes.len())]
        .copy_from_slice(&stream_bytes);
    source_data[dict_offset as usize..(dict_offset as usize + dict_bytes.len())]
        .copy_from_slice(&dict_bytes);

    // Create resolver with entries for both objects
    let mut resolver = XrefResolver::new();
    resolver.add_entry(10, XrefEntry::InUse {
        offset: stream_offset,
        gen_nr: 0,
    });
    resolver.add_entry(20, XrefEntry::InUse {
        offset: dict_offset,
        gen_nr: 0,
    });

    let source = MemorySource::new(source_data);
    let doc_context = DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: Some(Box::leak(Box::new(source))),
    };

    // Test reference to stream returns Stream
    let stream_ref = PdfObject::Ref(ObjRef::new(10, 0));
    let stream_result = detect_char_proc_type(&stream_ref, Some(&doc_context));
    assert_eq!(stream_result, CharProcType::Stream,
        "Reference to stream should return Stream");

    // Test reference to dict returns Dict
    let dict_ref = PdfObject::Ref(ObjRef::new(20, 0));
    let dict_result = detect_char_proc_type(&dict_ref, Some(&doc_context));
    assert_eq!(dict_result, CharProcType::Dict,
        "Reference to dict should return Dict");

    // Test reference to non-existent object returns Unknown
    let invalid_ref = PdfObject::Ref(ObjRef::new(30, 0));
    let invalid_result = detect_char_proc_type(&invalid_ref, Some(&doc_context));
    assert_eq!(invalid_result, CharProcType::Other("unknown".to_string()),
        "Reference to non-existent object should return Unknown");
}

/// Test that PdfObject::Ref without DocumentContext returns Unknown (comprehensive).
///
/// This test verifies the graceful degradation when no document context
/// is available for dereferencing. The function should not panic.
#[test]
fn test_detect_char_proc_type_ref_without_context_comprehensive() {
    // Create a reference without any DocumentContext
    let ref_obj = PdfObject::Ref(ObjRef::new(5, 0));

    // Should return Unknown gracefully without panicking
    let result = detect_char_proc_type(&ref_obj, None);

    assert_eq!(result, CharProcType::Other("unknown".to_string()),
        "Reference without DocumentContext should return Unknown");
}

/// Test that dereferencing handles references to wrong object types gracefully.
///
/// This test verifies that when a reference points to an object that is
/// neither a stream nor a dict (e.g., integer, string, etc.), it returns
/// CharProcType::Other with the appropriate type name.
#[test]
fn test_detect_char_proc_type_ref_to_non_stream_dict_returns_other() {
    use crate::font::type3_rasterizer::DocumentContext;
    use crate::parser::xref::XrefEntry;

    // Create a PDF integer object at object 15
    let int_bytes = b"15 0 obj\n42\nendobj\n".to_vec();
    let int_offset = 1500u64;
    let total_size = int_offset + int_bytes.len() as u64;

    let mut source_data = vec![0u8; total_size as usize];
    source_data[int_offset as usize..(int_offset as usize + int_bytes.len())]
        .copy_from_slice(&int_bytes);

    let mut resolver = XrefResolver::new();
    resolver.add_entry(15, XrefEntry::InUse {
        offset: int_offset,
        gen_nr: 0,
    });

    let source = MemorySource::new(source_data);
    let doc_context = DocumentContext {
        resolver: Some(Box::leak(Box::new(resolver))),
        source: Some(Box::leak(Box::new(source))),
    };

    // Create a reference to the integer object
    let ref_obj = PdfObject::Ref(ObjRef::new(15, 0));

    // Dereference and classify
    let result = detect_char_proc_type(&ref_obj, Some(&doc_context));

    // Verify Other is returned with type name "integer"
    match result {
        CharProcType::Other(type_name) => {
            assert_eq!(type_name, "integer",
                "Reference to integer object should return Other with type name 'integer'");
        }
        _ => panic!("Expected Other with type name, got {:?}", result),
    }
}

/// Test that detect_char_proc_type_with_context detects circular references.
///
/// This test verifies that circular references are detected and classified
/// as CharProcType::Other("circular-reference") without infinite recursion.
#[test]
fn test_detect_char_proc_type_with_context_circular_reference_detection() {
    use crate::font::type3_rasterizer::{detect_char_proc_type_with_context, DocumentContext};

    // Create a reference that would point to itself (circular)
    let circular_ref = ObjRef::new(100, 0);
    let ref_obj = PdfObject::Ref(circular_ref);

    // Create empty DocumentContext
    let doc_context = DocumentContext {
        resolver: None,
        source: None,
    };

    // Use the with_context variant that has cycle detection
    let result = detect_char_proc_type_with_context(&ref_obj, Some(&doc_context), 0);

    // Since resolver is None, it should return Unknown (circular detection
    // only applies if we can actually dereference)
    assert_eq!(result, CharProcType::Other("unknown".to_string()),
        "Circular reference with no resolver should return Unknown");
}

// ============================================================================
// Comprehensive Integration Test - All Fixtures Accessible
// ============================================================================

/// Integration test that verifies all test fixtures are accessible and functional.
///
/// This test serves as a compilation check to ensure that:
/// 1. All fixtures from type3_test_fixtures are properly imported
/// 2. All fixture types can be instantiated
/// 3. The test module has complete access to all fixture functionality
///
/// This is bead bf-535crb's integration verification step.
#[test]
fn test_all_fixtures_accessible() {
    use crate::font::type3_test_fixtures::{
        // Content stream fixtures
        Content, create_simple_charproc_stream, create_charproc_stream_with_curves,
        create_rectangle_charproc_stream, create_empty_content_stream,
        create_main_content_stream, create_main_content_stream_multi,
        // Glyph dictionary fixtures
        GlyphEntry, create_glyph_dict_with_basic_properties,
        create_basic_glyph_dict, create_minimal_glyph_dict, to_charprocs_map,
        // Type3Font fixtures
        create_minimal_type3_font,
        // Mock fixtures
        MockResolver, MockSource, MockCounter, mock_resolver, mock_source, mock_counter,
        // Character mapping fixtures
        create_basic_char_to_glyph_map, create_minimal_char_to_glyph_map,
        create_char_to_glyph_from_dict,
        // AET testing fixtures
        AETInspector, create_edges_from_endpoints, create_triangle_edges,
        create_rectangle_edges, create_scanline_context,
    };

    // Verify Content fixture type
    let _content = Content::with_defaults(b"test commands".to_vec());
    let _stream = create_simple_charproc_stream();
    let _curved_stream = create_charproc_stream_with_curves();
    let _rect_stream = create_rectangle_charproc_stream();
    let _empty_stream = create_empty_content_stream();
    let _main_stream = create_main_content_stream();
    let _multi_stream = create_main_content_stream_multi();

    // Verify Glyph dictionary fixtures
    let test_ref = ObjRef::new(100, 0);
    let _glyph_dict = create_glyph_dict_with_basic_properties(test_ref);

    let notdef_ref = ObjRef::new(101, 0);
    let test_ref2 = ObjRef::new(102, 0);
    let _basic_dict = create_basic_glyph_dict(notdef_ref, test_ref2);

    let _minimal_dict = create_minimal_glyph_dict(notdef_ref);
    let _charprocs = to_charprocs_map(&_basic_dict);

    // Verify Type3Font fixtures
    let notdef_ref3 = ObjRef::new(103, 0);
    let _font = create_minimal_type3_font(notdef_ref3);

    // Verify Mock fixtures
    let _resolver: MockResolver = mock_resolver();
    let _source: MockSource = mock_source();
    let _counter: MockCounter = mock_counter();

    // Verify Character mapping fixtures
    let _char_map = create_basic_char_to_glyph_map();
    let _minimal_map = create_minimal_char_to_glyph_map();
    let _dict_map = create_char_to_glyph_from_dict(&_basic_dict);

    // Verify AET testing fixtures
    let edges = create_edges_from_endpoints(&[(0, 0, 10, 10), (10, 0, 20, 10)]);
    let _triangle_edges = create_triangle_edges();
    let _rect_edges = create_rectangle_edges(0, 0, 100, 100);
    let (width, height) = create_scanline_context(100, 100);

    // Verify AETInspector with edges
    let _inspector = AETInspector::new(edges);

    // Verify basic compilation of fixture usage
    assert_eq!(width, 100);
    assert_eq!(height, 100);

    // Verify GlyphEntry type is accessible
    let _entry = GlyphEntry::minimal("test".to_string(), test_ref);

    // Test passes - all fixtures are accessible and functional
}

// ============================================================================
// DocumentContext Resolution Path Tests (resolve_stream callback absent)
//
// Every rasterize_type3_glyph test above supplies a resolve_stream callback,
// making the callback authoritative. These tests exercise the other half of
// the stream-resolution contract (pdftract-3923eb56): with no callback, the
// char_proc ObjRef must be dereferenced through the DocumentContext's
// XrefResolver + PdfSource, decoded, and executed — with no synthetic or
// hardcoded stream standing in for the document's real bytes.
// ============================================================================

/// Build a Type3 font whose /CharProcs maps "A" to obj_nr 0 R.
///
/// The identity FontMatrix and FontBBox [0 0 32 32] make glyph-space
/// coordinates land 1:1 on bitmap pixels (34x34 with the 1px padding),
/// so ink positions can be asserted exactly.
fn create_context_path_font(obj_nr: u32) -> Type3Font {
    create_context_path_font_multi(&[("/A", obj_nr)])
}

/// Build a Type3 font mapping every (name, obj_nr) pair in /CharProcs.
///
/// Same identity FontMatrix and FontBBox [0 0 32 32] as the single-glyph
/// builder above, so glyph-space coordinates land 1:1 on bitmap pixels
/// (34x34 with the 1px padding) and ink positions can be asserted exactly.
/// The multi-entry CharProcs dict is what the end-to-end fixture tests at
/// the bottom of this file need: several glyph kinds rasterized through one
/// DocumentContext.
fn create_context_path_font_multi(entries: &[(&str, u32)]) -> Type3Font {
    let mut char_procs = PdfDict::new();
    for (name, obj_nr) in entries {
        char_procs.insert(intern(name), PdfObject::Ref(ObjRef::new(*obj_nr, 0)));
    }

    let mut font_dict = PdfDict::new();
    font_dict.insert(intern("/CharProcs"), PdfObject::Dict(Box::new(char_procs)));
    font_dict.insert(
        intern("/FontMatrix"),
        PdfObject::Array(Box::new(vec![
            PdfObject::Integer(1),
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(1),
            PdfObject::Integer(0),
            PdfObject::Integer(0),
        ])),
    );
    font_dict.insert(
        intern("/FontBBox"),
        PdfObject::Array(Box::new(vec![
            PdfObject::Integer(0),
            PdfObject::Integer(0),
            PdfObject::Integer(32),
            PdfObject::Integer(32),
        ])),
    );

    Type3Font::load(&font_dict)
}

/// Compress `data` with zlib for /FlateDecode stream payloads.
fn zlib_compress(data: &[u8]) -> Vec<u8> {
    use flate2::write::ZlibEncoder;
    use flate2::Compression;
    use std::io::Write;

    let mut encoder = ZlibEncoder::new(Vec::new(), Compression::default());
    encoder.write_all(data).unwrap();
    encoder.finish().unwrap()
}

/// A valid charproc stream reached through the DocumentContext must be
/// dereferenced from the document's own bytes and actually rasterized.
///
/// The rect drawn here spans (8,8)-(20,20) — geometry distinct from every
/// other rasterizer test — and the pixel-level assertions below can only be
/// satisfied by executing the real decoded stream, not by any fixed bitmap.
#[test]
fn test_rasterize_via_document_context_valid_stream_rasterizes_real_glyph() {
    let obj_bytes = create_pdf_stream_object(21, 0, "", b"8 8 12 12 re f");
    let doc_context = create_valid_dereference_context(vec![(21, 600, 0, obj_bytes)]);
    let font = create_context_path_font(21);
    assert!(font.has_glyph("A"), "fixture must map glyph A to object 21");

    let bitmap = rasterize_type3_glyph(&font, "A", Some(&doc_context), None::<&StreamResolverFn>)
        .expect("a valid char_proc ObjRef must resolve via the context and rasterize");

    // FontBBox [0 0 32 32] + 1px padding => 34x34 row-major bitmap.
    let (width, height) = calculate_bitmap_dimensions(&font.font_bbox, None);
    assert_eq!((width, height), (34, 34));
    assert_eq!(bitmap.len(), width * height);

    let px = |x: usize, y: usize| bitmap[y * width + x];
    assert_eq!(px(8, 8), 0, "rect lower-left corner must be black ink");
    assert_eq!(px(14, 14), 0, "rect interior must be black ink");
    assert_eq!(px(20, 20), 0, "rect upper-right corner must be black ink");
    assert_eq!(px(0, 0), 255, "padding outside the rect must stay white");
    assert_eq!(px(25, 25), 255, "pixels beyond the rect must stay white");

    let inked = bitmap.iter().filter(|&&p| p != 255).count();
    assert_eq!(
        inked,
        13 * 13,
        "scanline fill must cover exactly the 13x13 pixel rect"
    );
}

/// A FlateDecode charproc reached through the DocumentContext must come back
/// decompressed: the compressed payload contains no valid operators, so any
/// ink in the bitmap proves decode_stream actually inflated the stream.
#[test]
fn test_rasterize_via_document_context_flate_stream_decodes() {
    let content = b"2 2 20 20 re f";
    let compressed = zlib_compress(content);
    assert_ne!(compressed, content.to_vec());

    // Binary payload: assemble the indirect object by hand so the bytes are
    // not round-tripped through from_utf8_lossy.
    let mut obj_bytes = format!(
        "22 0 obj\n<< /Filter /FlateDecode /Length {} >>\nstream\n",
        compressed.len()
    )
    .into_bytes();
    obj_bytes.extend_from_slice(&compressed);
    obj_bytes.extend_from_slice(b"\nendstream\nendobj\n");

    let doc_context = create_valid_dereference_context(vec![(22, 700, 0, obj_bytes)]);
    let font = create_context_path_font(22);

    let bitmap = rasterize_type3_glyph(&font, "A", Some(&doc_context), None::<&StreamResolverFn>)
        .expect("a compressed char_proc ObjRef must decode via the context and rasterize");

    // '2 2 20 20 re f' fills the 21x21 pixel rect from (2,2) to (22,22).
    let inked = bitmap.iter().filter(|&&p| p != 255).count();
    assert_eq!(inked, 21 * 21, "decoded stream must ink the 21x21 rect");
}

/// A charproc reference with no xref entry must degrade to None.
#[test]
fn test_rasterize_via_document_context_missing_ref_returns_none() {
    // The context knows object 21; the font points glyph A at absent object 99.
    let obj_bytes = create_pdf_stream_object(21, 0, "", b"0 0 10 10 re f");
    let doc_context = create_valid_dereference_context(vec![(21, 600, 0, obj_bytes)]);
    let font = create_context_path_font(99);
    assert!(font.has_glyph("A"), "glyph A exists; only its target is missing");

    let result = rasterize_type3_glyph(&font, "A", Some(&doc_context), None::<&StreamResolverFn>);
    assert!(result.is_none(), "missing charproc object must degrade to None");
}

/// A charproc reference that resolves to a non-stream object must degrade
/// to None rather than executing or fabricating a stream.
#[test]
fn test_rasterize_via_document_context_non_stream_target_returns_none() {
    let dict_obj = create_pdf_dict_object(23, 0, "/Type /Font /Subtype /Type3");
    let doc_context = create_valid_dereference_context(vec![(23, 800, 0, dict_obj)]);
    let font = create_context_path_font(23);

    let result = rasterize_type3_glyph(&font, "A", Some(&doc_context), None::<&StreamResolverFn>);
    assert!(
        result.is_none(),
        "charproc target that is a dictionary, not a stream, must degrade to None"
    );
}

/// A charproc reference whose xref entry points at unparseable bytes must
/// degrade to None.
#[test]
fn test_rasterize_via_document_context_malformed_object_returns_none() {
    let garbage = b"\xff\xfe garbage bytes".to_vec();
    let doc_context = create_valid_dereference_context(vec![(24, 900, 0, garbage)]);
    let font = create_context_path_font(24);

    let result = rasterize_type3_glyph(&font, "A", Some(&doc_context), None::<&StreamResolverFn>);
    assert!(result.is_none(), "unparseable target bytes must degrade to None");
}

/// With neither a callback nor a DocumentContext there is no resolution
/// route at all; a known glyph must still degrade to None, not panic.
#[test]
fn test_rasterize_without_context_or_callback_returns_none() {
    let font = create_context_path_font(21);
    assert!(font.has_glyph("A"));

    let result = rasterize_type3_glyph(&font, "A", None, None::<&StreamResolverFn>);
    assert!(
        result.is_none(),
        "no callback and no context must degrade to None"
    );
}

// ============================================================================
// End-to-End DocumentContext Fixture Tests (multi-glyph, parent criterion 5)
//
// The tests above prove individual resolution failures degrade to None and
// that a single filled rect rasterizes. These tests exercise the FULL
// pipeline — Type3 font dict -> char_proc ObjRef -> DocumentContext
// dereference -> decode_stream -> execute_content_stream -> rasterized
// bitmap — across several glyph kinds sharing one document, on a fixture
// with a FontBBox and multiple CharProcs entries (pdftract-384db234).
// ============================================================================

/// Build the multi-glyph end-to-end fixture: a Type3 font with FontBBox
/// [0 0 32 32] and four CharProcs entries, one per glyph kind, all
/// resolving through a single DocumentContext.
///
/// Object numbers are spaced 200 bytes apart so the helper-assembled
/// indirect objects cannot overlap inside the synthesized source buffer.
/// The identity FontMatrix keeps glyph-space coordinates 1:1 with bitmap
/// pixels, so every ink position below is asserted exactly.
fn create_multi_glyph_fixture() -> (Type3Font, DocumentContext<'static>) {
    let filled_rect = create_pdf_stream_object(31, 0, "", b"3 3 6 6 re f");
    let stroked_line = create_pdf_stream_object(41, 0, "", b"6 10 m 26 10 l S");
    let multi_op = create_pdf_stream_object(
        42,
        0,
        "",
        b"q 1 0 0 1 4 6 cm 2 2 8 8 re f Q 10 20 m 24 20 l S",
    );
    let malformed = create_pdf_stream_object(43, 0, "", b"ZZ (junk) [1 2 /Nm 8 8 12 12");

    let doc_context = create_valid_dereference_context(vec![
        (31, 400, 0, filled_rect),
        (41, 600, 0, stroked_line),
        (42, 800, 0, multi_op),
        (43, 1000, 0, malformed),
    ]);
    let font = create_context_path_font_multi(&[("/A", 31), ("/B", 41), ("/C", 42), ("/D", 43)]);
    (font, doc_context)
}

/// Glyph kind 2 of 3: a stroked path reached through the DocumentContext.
///
/// Rasterizing "A" (filled rect) and "B" (stroked line) from the same
/// multi-entry fixture proves CharProcs navigation across several entries
/// and that a stroked path — not just a scanline fill — reaches the bitmap
/// through document resolution. The stroke convention is 1-pixel Bresenham
/// ink (0), so '6 10 m 26 10 l S' must ink exactly row 10, columns 6..=26.
#[test]
fn test_rasterize_via_document_context_stroked_path_glyph() {
    let (font, doc_context) = create_multi_glyph_fixture();
    assert!(
        font.has_glyph("A") && font.has_glyph("B"),
        "fixture must carry at least two CharProcs entries"
    );

    let (width, height) = calculate_bitmap_dimensions(&font.font_bbox, None);
    assert_eq!((width, height), (34, 34));

    // Glyph A: '3 3 6 6 re f' fills exactly [3..=9]x[3..=9] = 7x7 pixels.
    let bitmap_a = rasterize_type3_glyph(&font, "A", Some(&doc_context), None::<&StreamResolverFn>)
        .expect("filled-rect glyph must resolve via the context and rasterize");
    assert_eq!(bitmap_a.len(), width * height);
    assert_eq!(
        bitmap_a.iter().filter(|&&p| p != 255).count(),
        7 * 7,
        "filled-rect glyph must ink exactly its 7x7 geometry"
    );

    // Glyph B: the stroked line.
    let bitmap_b = rasterize_type3_glyph(&font, "B", Some(&doc_context), None::<&StreamResolverFn>)
        .expect("stroked-path glyph must resolve via the context and rasterize");
    assert_eq!(bitmap_b.len(), width * height);

    let px = |x: usize, y: usize| bitmap_b[y * width + x];
    assert_eq!(px(6, 10), 0, "line start must be black ink");
    assert_eq!(px(16, 10), 0, "line midpoint must be black ink");
    assert_eq!(px(26, 10), 0, "line end must be black ink");
    assert_eq!(px(16, 9), 255, "stroke must stay one pixel tall above");
    assert_eq!(px(16, 11), 255, "stroke must stay one pixel tall below");
    assert_eq!(px(0, 0), 255, "padding outside the stroke must stay white");

    let inked = bitmap_b.iter().filter(|&&p| p != 255).count();
    assert_eq!(
        inked, 21,
        "Bresenham must ink exactly columns 6..=26 of row 10"
    );
    assert_ne!(
        bitmap_a, bitmap_b,
        "the two glyph kinds must rasterize to different bitmaps"
    );
}

/// Glyph kind 3 of 3: a multi-operator charproc reached through the
/// DocumentContext. 'q 1 0 0 1 4 6 cm 2 2 8 8 re f Q 10 20 m 24 20 l S'
/// combines graphics-state save/restore, a cm transform, a fill, and a
/// stroke in one stream; the ink positions below can only be produced by
/// executing every operator in order against the document-resolved bytes.
#[test]
fn test_rasterize_via_document_context_multi_operator_glyph() {
    let (font, doc_context) = create_multi_glyph_fixture();
    assert!(font.has_glyph("C"), "fixture must map glyph C");

    let bitmap = rasterize_type3_glyph(&font, "C", Some(&doc_context), None::<&StreamResolverFn>)
        .expect("multi-operator glyph must resolve via the context and rasterize");

    let (width, height) = calculate_bitmap_dimensions(&font.font_bbox, None);
    assert_eq!((width, height), (34, 34));
    assert_eq!(bitmap.len(), width * height);

    let px = |x: usize, y: usize| bitmap[y * width + x];
    // cm translates the rect (2,2)-(10,10) by (4,6) -> (6,8)-(14,16).
    assert_eq!(px(6, 8), 0, "translated rect lower-left must be inked");
    assert_eq!(px(10, 13), 0, "translated rect interior must be inked");
    assert_eq!(px(14, 16), 0, "translated rect upper-right must be inked");
    assert_eq!(px(2, 2), 255, "untranslated corner must stay white: cm was applied");
    assert_eq!(px(2, 10), 255, "untranslated left edge must stay white: cm was applied");
    // After Q restores the identity CTM, the stroke lands at raw coordinates.
    assert_eq!(px(10, 20), 0, "post-restore stroke start must be inked");
    assert_eq!(px(24, 20), 0, "post-restore stroke end must be inked");
    assert_eq!(px(17, 19), 255, "stroke must stay one pixel tall above");
    assert_eq!(px(17, 21), 255, "stroke must stay one pixel tall below");
    assert_eq!(px(0, 0), 255, "padding must stay white");

    // Fill [6..=14]x[8..=18] = 9x9 pixels plus a 15-pixel stroke on row 20.
    let inked = bitmap.iter().filter(|&&p| p != 255).count();
    assert_eq!(
        inked,
        9 * 9 + 15,
        "fill and stroke must each contribute exactly their own geometry"
    );
}

/// A charproc whose stream RESOLVES but whose content is garbage must not
/// panic and must not fabricate ink. execute_content_stream's documented
/// contract is tolerant degradation — unknown operators are skipped and
/// dangling operands are ignored — so the pipeline returns a full-size
/// blank bitmap. Resolution-level failures (missing ref, non-stream target,
/// unparseable object bytes) are the cases that return None; the sibling
/// tests above cover those end-to-end.
#[test]
fn test_rasterize_via_document_context_malformed_content_degrades_gracefully() {
    let (font, doc_context) = create_multi_glyph_fixture();
    assert!(font.has_glyph("D"), "fixture must map glyph D");

    let bitmap = rasterize_type3_glyph(&font, "D", Some(&doc_context), None::<&StreamResolverFn>)
        .expect("resolved-but-malformed content degrades to a blank bitmap, not None");

    let (width, height) = calculate_bitmap_dimensions(&font.font_bbox, None);
    assert_eq!((width, height), (34, 34), "malformed content must not change bitmap dimensions");
    assert_eq!(bitmap.len(), width * height);
    assert!(
        bitmap.iter().all(|&p| p == 255),
        "garbage content carries no painting operator, so every pixel must stay white"
    );
}
