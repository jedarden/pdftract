//! xtask library for pdftract development tasks.
//!
//! This library exposes reusable modules for development tasks including
//! schema migration, CI/CD workflows, and other utilities.

pub mod ci;
pub mod migrate;

// Re-export commonly used functions for convenience
pub use ci::{
    submit_rust_verify_workflow, wait_for_workflow_completion, get_workflow_output,
    ArgoConfig, RustVerifyParams,
};
pub use migrate::migrate;
