//! PDF object model.
//!
//! This module defines the core PDF object types and the object reference type.

pub mod cache;
pub mod cycle;
pub mod parser;
pub mod types;

pub use cache::ObjectCache;
pub use cycle::{is_resolving, ResolutionGuard, RESOLVING};
pub use parser::ObjectParser;
pub use types::{intern, ObjRef, PdfDict, PdfIndirect, PdfObject, PdfStream};
