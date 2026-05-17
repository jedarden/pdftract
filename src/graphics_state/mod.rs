//! Graphics state machine for PDF content stream processing.
//!
//! This module implements the full PDF graphics state including:
//! - Current transformation matrix (CTM)
//! - Text matrices (Tm and Tlm)
//! - Font binding and text state parameters
//! - Color state
//! - State stack (q/Q operators)

mod color;
mod diagnostics;
mod matrix;
mod state;
mod stack;

pub use color::Color;
pub use diagnostics::{Diagnostic, Severity};
pub use matrix::Matrix3x3;
pub use state::{GraphicsState, TextRenderingMode};
pub use stack::GraphicsStateStack;
