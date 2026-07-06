//! CMap (Character Map) parsing for PDF Type0 fonts and CID fonts.
//!
//! This module provides parsing for CMap streams used in PDF fonts to map
//! character codes to CID (Character ID) values and Unicode codepoints.

pub mod codespace;

#[cfg(feature = "cjk")]
pub mod tokenize;

pub use codespace::{
    parse_codespace_ranges, parse_codespace_ranges_with_diags, CodespaceRange, CodespaceRanges,
};

#[cfg(feature = "cjk")]
pub use tokenize::tokenize_cjk_bytes;
