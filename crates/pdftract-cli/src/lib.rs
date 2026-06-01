//! pdftract CLI library.
//!
//! This library exports the CLI's internal modules for integration testing.

pub mod cache_cmd;
pub mod classify;
pub mod cli;
pub mod codegen;
pub mod grep;
pub mod hash;
pub mod header;
pub mod inspect;
pub mod mcp;
pub mod middleware;
pub mod migrate;
pub mod output;
pub mod pages;
pub mod password;
pub mod profiles_cmd;
pub mod serve;
pub mod url;
pub mod validate;
pub mod verify_receipt;

// Re-export diagnostics for testing
pub use pdftract_core::diagnostics::{DiagCode, DiagInfo, DIAGNOSTIC_CATALOG};

// Export CLI types for documentation generation
pub use crate::cli::{Cli, Commands};

/// Generate CLI reference markdown from the clap command tree.
///
/// This function uses clap-markdown to auto-generate comprehensive CLI
/// documentation from the clap derive annotations. It includes all
/// subcommands, flags, arguments, and options with their types, defaults,
/// and help text.
pub fn generate_cli_markdown() -> String {
    // clap-markdown 0.1 uses help_markdown function
    clap_markdown::help_markdown::<Cli>()
}
