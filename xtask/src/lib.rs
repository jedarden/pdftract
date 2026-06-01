//! xtask library for pdftract development tasks.
//!
//! This library exposes reusable modules for development tasks including
//! schema migration and other utilities.

pub mod migrate;

// Re-export the migrate function for convenience
pub use migrate::migrate;
