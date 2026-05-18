//! PDF object model.
//!
//! This module defines the core PDF object types and the object reference type.

pub mod types;

pub use types::{ObjRef, PdfObject, PdfDict, PdfStream, PdfIndirect, intern};
