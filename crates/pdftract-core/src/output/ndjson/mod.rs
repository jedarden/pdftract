//! NDJSON streaming output mode.
//!
//! This module implements the streaming NDJSON output format, where
//! extraction results are emitted as a sequence of newline-delimited
//! JSON frames:
//!
//! - Header frame: Document metadata and outline
//! - Page frames: One per page, emitted as pages complete
//! - Footer frame: Aggregated quality metrics and diagnostics
//!
//! The streaming mode keeps memory bounded by using a fixed-size
//! out-of-order buffer to handle rayon's parallel page extraction.

pub mod buffer;
pub mod frames;
pub mod pipeline;

pub use buffer::OutOfOrderBuffer;
pub use frames::{FooterFrame, HeaderFrame, NdjsonFrame, PageFrame};
// serde is an optional capability: JSON call sites gate on the feature so `--no-default-features` (the wasm32 library edge) compiles (pdftract-c1fceb36).
#[cfg(feature = "serde")]
pub use frames::write_frame;
#[cfg(feature = "serde")]
pub use pipeline::{extract_streaming, footer_errors};
