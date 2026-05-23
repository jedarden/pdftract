//! pdftract-core — Core PDF parsing and text extraction primitives.
//!
//! This crate provides the foundational data structures and parsers for
//! processing PDF documents, including the lexer, object parser, and
//! text extraction engines.

pub mod cache;
pub mod diagnostics;
pub mod document;
pub mod extract;
pub mod fingerprint;
pub mod options;
pub mod parser;
pub mod receipts;
pub mod schema;
pub mod semaphore;
