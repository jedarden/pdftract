//! Inspector web debug viewer.
//!
//! This module implements Phase 7.9's `pdftract inspect` subcommand:
//! a local web server that renders PDF extraction results with
//! interactive debugging overlays.

pub mod args;
pub mod inspect;
pub mod render;

pub use args::InspectArgs;
pub use inspect::run;
