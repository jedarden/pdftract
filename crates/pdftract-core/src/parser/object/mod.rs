//! PDF object model.
//!
//! This module defines the core PDF object types and the object reference type.

pub mod parser;
pub mod types;

pub use parser::ObjectParser;
pub use types::{intern, ObjRef, PdfDict, PdfIndirect, PdfObject, PdfStream};
