//! pdftract-libpdftract — C/C++ FFI library for pdftract.
//!
//! This crate provides the extern "C" API surface for C/C++ integrations.
//! It compiles to both shared (cdylib) and static (staticlib) libraries,
//! allowing downstream projects to link dynamically or statically.
//!
//! ## Output artifacts
//!
//! - Linux: `target/debug/libpdftract.so` (shared), `target/debug/libpdftract.a` (static)
//! - macOS: `target/debug/libpdftract.dylib` (shared), `target/debug/libpdftract.a` (static)
//! - Windows: `target/debug/pdftract.dll` (shared), `target/debug/pdftract.lib` (static)

// Public API modules will be added here in sibling beads.
// This scaffold provides the minimal structure for cdylib + staticlib builds.
