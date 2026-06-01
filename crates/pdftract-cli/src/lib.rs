//! pdftract CLI library.
//!
//! This library exports the CLI's internal modules for integration testing.

pub mod grep;
pub mod header;
pub mod inspect;
pub mod mcp;
pub mod middleware;
pub mod output;

// Re-export diagnostics for testing
pub use pdftract_core::diagnostics::{DiagCode, DiagInfo, DIAGNOSTIC_CATALOG};

// Export CLI types for documentation generation
#[cfg(doc)]
pub use crate::main::{Cli, Commands};

/// Generate CLI reference markdown from the clap command tree.
///
/// This function uses clap-markdown to auto-generate comprehensive CLI
/// documentation from the clap derive annotations. It includes all
/// subcommands, flags, arguments, and options with their types, defaults,
/// and help text.
pub fn generate_cli_markdown() -> String {
    // clap-markdown 0.1 returns a String directly
    clap_markdown::to_markdown::<crate::main::Cli>()
}
