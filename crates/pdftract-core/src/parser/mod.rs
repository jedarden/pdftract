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

// Re-export from the unified diagnostics module (Phase 1.6)
pub use crate::diagnostics::{Diagnostic, Severity, DiagCode, ObjRef};
pub use object::{PdfObject};
pub use objstm::{ObjectStmParser, ObjStmCacheEntry, ObjStmResult, ObjStmError};
pub use xref::{
    XrefResolver, XrefEntry, ResolveError, ResolveResult, XrefSection,
    parse_traditional_xref, parse_xref_stream, merge_hybrid, is_hybrid_trailer,
    LinearizationInfo, detect_linearization, load_xref_linearized, merge_linearized_xrefs,
    load_xref_with_prev_chain,
};
pub use catalog::{Catalog, MarkInfo, PageLabel, PageLabelsTree, PageLabelStyle, parse_catalog};
pub use ocg::{OcProperties, OcGroup, Ocmd, OcmdPolicy, BaseState, parse_oc_properties};
pub use resources::{ResourceDict, merge_resources, extract_resources};
pub use pages::{PageDict, flatten_page_tree, DEFAULT_MEDIABOX};
pub use stream::{
    StreamDecoder, FlateDecoder, ASCII85Decoder, ASCIIHexDecoder, CryptDecoder, PassthroughDecoder,
    normalize_filter_name, get_decoder, FilterError, DEFAULT_MAX_DECOMPRESS_BYTES,
};
