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
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use crate::font::encoding::NamedEncoding;
use crate::font::type3_rasterizer::{rasterize_type3_glyph, DocumentContext, StreamResolverFn};
use crate::font::type3::Type3Font;
use crate::graphics_state::Matrix3x3;
use crate::parser::object::types::{intern, ObjRef, PdfDict, PdfObject};

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

    // For a 32x32 bitmap, we expect 1024 bytes (32 * 32)
    assert_eq!(bitmap_bytes.len(), 1024, "32x32 bitmap should be 1024 bytes");
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
