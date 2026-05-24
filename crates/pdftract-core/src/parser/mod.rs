//! PDF parsing primitives.
//!
//! This module provides the lexer and object parser for reading PDF documents.

pub mod catalog;
pub mod diagnostic;
pub mod lexer;
pub mod marked_content;
pub mod marked_content_operators;
pub mod marked_content_stack;
pub mod object;
pub mod objstm;
pub mod ocg;
pub mod outline;
pub mod pages;
pub mod resources;
pub mod secrets;
pub mod stream;
pub mod struct_tree;
pub mod xref;

// Re-export from the unified diagnostics module (Phase 1.6)
pub use crate::diagnostics::{DiagCode, Diagnostic, ObjRef, Severity};
pub use catalog::{
    parse_catalog, Catalog, MarkInfo, PageLabel, PageLabelStyle, PageLabelsTree,
    ReadingOrderAlgorithm,
};
pub use marked_content::{
    compute_coverage, compute_coverage_from_sets, CoverageResult, McidTracker,
};
pub use marked_content_operators::{parse_bdc, parse_bmc, parse_emc};
pub use marked_content_stack::{MarkedContentFrame, MarkedContentStack};
pub use object::PdfObject;
pub use objstm::{ObjStmCacheEntry, ObjStmError, ObjStmResult, ObjectStmParser};
pub use ocg::{parse_oc_properties, BaseState, OcGroup, OcProperties, Ocmd, OcmdPolicy};
pub use pages::{flatten_page_tree, PageDict, DEFAULT_MEDIABOX};
pub use resources::{extract_resources, merge_resources, ResourceDict};
pub use stream::{
    get_decoder, normalize_filter_name, ASCII85Decoder, ASCIIHexDecoder, CryptDecoder, FilterError,
    FlateDecoder, PassthroughDecoder, StreamDecoder, DEFAULT_MAX_DECOMPRESS_BYTES,
};
pub use struct_tree::{
    check_coverage_for_pages, is_artifact, map_element_to_block, parse_struct_tree,
    structure_type_to_block_kind, BlockKind, CoverageCheckResult, Kid, MappingResult,
    ParentTreeEntry, ParentTreeResolver, RoleMap, StructElemNode, StructTreeRoot, StructureType,
};
pub use xref::{
    detect_linearization, is_hybrid_trailer, load_xref_linearized, load_xref_with_prev_chain,
    merge_hybrid, merge_linearized_xrefs, parse_traditional_xref, parse_xref_stream,
    LinearizationInfo, ResolveError, ResolveResult, XrefEntry, XrefResolver, XrefSection,
};
