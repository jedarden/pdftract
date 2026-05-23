//! pdftract-core — Core PDF parsing and text extraction primitives.
//!
//! This crate provides the foundational data structures and parsers for
//! processing PDF documents, including the lexer, object parser, and
//! text extraction engines.

pub mod diagnostics;
pub mod document;
pub mod fingerprint;
pub mod parser;
pub mod receipts;
pub mod schema;
