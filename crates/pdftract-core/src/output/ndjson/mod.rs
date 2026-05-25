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
pub use frames::{FooterFrame, HeaderFrame, PageFrame};
pub use pipeline::extract_streaming;
