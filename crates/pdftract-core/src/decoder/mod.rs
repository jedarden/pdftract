//! PDF stream decoders for filter processing.
//!
//! This module provides specialized decoders for PDF stream filters that
//! require metadata extraction or diagnostic emission beyond simple
//! passthrough.

pub mod jbig2;
pub mod jpx;

pub use jbig2::{Jbig2Decoder, Jbig2GlobalsRef};
pub use jpx::JpxDecoder;
