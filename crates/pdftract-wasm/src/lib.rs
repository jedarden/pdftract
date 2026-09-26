//! WASM bindings for pdftract's vector-text extraction pipeline.
//!
//! Scaffold for ADR-010's client-side browser demo, re-scoped by ADR-011 after
//! the `bf-2uw30r` wasm32 spike. The spike confirmed the only real blockers to
//! targeting `wasm32-unknown-unknown` are native-C dependencies (zstd-sys) and
//! Unix-only filesystem code — both of which live in `pdftract-core`'s
//! filesystem-backed extraction cache, now behind the opt-out `cache` feature.
//! This crate builds `pdftract-core` with the no-native-deps feature set
//! (`serde`, `decrypt`, `quick-xml` — the native default minus `cache`) and
//! consumes PDF bytes from an in-memory source (`pdftract_core`'s
//! `MemorySource`), never from `mmap`.
//!
//! # Scope (deliberately narrowed, per ADR-010)
//!
//! - Vector-text pages only: parsing, font-encoding recovery, reading-order
//!   segmentation. OCR/scanned pages are out of scope — the demo shows an
//!   explicit "vector-only in-browser" message rather than degrading silently.
//! - Serial page processing. `rayon` page-parallelism is OCR-only in
//!   `pdftract-core` and not part of the wasm build.
//!
//! The public surface intentionally accepts bytes and returns JSON. Browser
//! callers own file loading and can pass a `Uint8Array` without giving the
//! parser access to a browser filesystem or a native path.

use wasm_bindgen::prelude::{wasm_bindgen, JsValue};

/// Returns the pdftract version this binding was built against.
///
/// The workspace shares a single version across all pdftract crates, so this
/// is also the `pdftract-core` version linked into the wasm bundle.
#[wasm_bindgen]
pub fn pdftract_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

/// Extract vector text from an in-memory PDF and return the structured result
/// as JSON.
///
/// The wasm build supports vector-text extraction only. Scanned-page OCR,
/// full-page PDFium rendering, remote HTTP sources, the filesystem cache, and
/// the `pdftract serve` process mode are native-only features.
#[wasm_bindgen]
pub fn extract_vector_text(pdf_bytes: &[u8]) -> Result<String, JsValue> {
    let mut options = pdftract_core::ExtractionOptions::default();
    options.max_parallel_pages = 1;

    let result = pdftract_core::extract_pdf_from_bytes(pdf_bytes, &options)
        .map_err(|error| JsValue::from_str(&error.to_string()))?;

    serde_json::to_string(&result)
        .map_err(|error| JsValue::from_str(&format!("failed to serialize extraction: {error}")))
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_non_empty() {
        assert!(!super::pdftract_version().is_empty());
        assert!(super::pdftract_version().contains('.'));
    }

    #[test]
    fn vector_extraction_smoke_uses_in_memory_pdf() {
        let pdf = include_bytes!("../../../tests/fixtures/test-minimal.pdf");
        let json = super::extract_vector_text(pdf).expect("vector fixture should extract");
        let value: serde_json::Value =
            serde_json::from_str(&json).expect("binding should return valid JSON");

        assert_eq!(value["metadata"]["page_count"], 1);
        assert!(value["pages"][0]["spans"]
            .as_array()
            .expect("page spans should be an array")
            .iter()
            .any(|span| span["text"] == "Dummy PDF file"));
    }
}
