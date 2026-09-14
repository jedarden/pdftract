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
//! # Status
//!
//! Scaffold only: this crate proves the `pdftract-core` wasm32 compilation
//! edge (enforced in CI by the `wasm32-check` leg of `pdftract-ci`) and pins
//! the public binding surface. The extraction bindings and the static demo
//! page are follow-up work, gated on the native extraction pipeline going
//! green on the regression corpus (see ADR-011).

use wasm_bindgen::prelude::wasm_bindgen;

/// Returns the pdftract version this binding was built against.
///
/// The workspace shares a single version across all pdftract crates, so this
/// is also the `pdftract-core` version linked into the wasm bundle.
#[wasm_bindgen]
pub fn pdftract_version() -> String {
    env!("CARGO_PKG_VERSION").to_string()
}

#[cfg(test)]
mod tests {
    #[test]
    fn version_is_non_empty() {
        assert!(!super::pdftract_version().is_empty());
        assert!(super::pdftract_version().contains('.'));
    }
}
