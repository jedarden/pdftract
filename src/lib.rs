//! pdftract — PDF text extraction library that gets the hard parts right.
//!
//! This library provides correct reading order, font encoding recovery,
//! structure tree extraction, and per-page hybrid routing.

pub mod graphics_state;

pub use graphics_state::{
    Color,
    Diagnostic,
    GraphicsState,
    GraphicsStateStack,
    Matrix3x3,
    Severity,
};
