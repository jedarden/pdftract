//! PDF parsing primitives.
//!
//! This module provides the lexer and object parser for reading PDF documents.

pub mod diagnostic;
pub mod lexer;
pub mod object;
pub mod objstm;
pub mod xref;
pub mod catalog;
pub mod stream;
pub mod secrets;
pub mod pages;
pub mod outline;
pub mod resources;
pub mod ocg;

pub use diagnostic::{Diagnostic, Severity, DiagCode};
pub use object::{ObjRef, PdfObject};
pub use objstm::{ObjectStmParser, ObjStmCacheEntry, ObjStmResult, ObjStmError};
pub use xref::{XrefResolver, XrefEntry, ResolveError, ResolveResult, XrefSection, XrefDiagnostic, XrefDiagCode, parse_traditional_xref};
pub use catalog::{Catalog, MarkInfo, PageLabel, PageLabelsTree, PageLabelStyle, parse_catalog};
pub use ocg::{OcProperties, OcGroup, Ocmd, OcmdPolicy, BaseState, parse_oc_properties};
pub use stream::{
    StreamDecoder, FlateDecoder, ASCII85Decoder, ASCIIHexDecoder, CryptDecoder, PassthroughDecoder,
    normalize_filter_name, get_decoder, FilterError, DEFAULT_MAX_DECOMPRESS_BYTES,
};
