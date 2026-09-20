//! pdftract CLI library.
//!
//! This library exports the CLI's internal modules for integration testing.

pub mod cache_cmd;
pub mod classify;
pub mod cli;
pub mod codegen;
#[cfg(feature = "grep")]
pub mod grep;
pub mod hash;
pub mod header;
pub mod inspect;
pub mod mcp;
pub mod middleware;
/// Metrics registry and OpenMetrics v1.0 text exposition (`metrics` feature).
#[cfg(feature = "metrics")]
pub mod metrics;
pub mod migrate;
pub mod output;
pub mod pages;
pub mod password;
pub mod profiles_cmd;
/// Feeds `pdftract_remote_bytes_downloaded_total` from the remote fetch
/// path (`metrics` + `remote` features). The cli registers the
/// registry-backed hook where it builds a remote source — `main.rs`'s
/// URL-extract path, `hash::compute_fingerprint_from_url`, and the remote
/// branch of `grep::worker::worker_run`.
#[cfg(all(feature = "metrics", feature = "remote"))]
pub mod remote_metrics;
pub mod serve;
pub mod url;
pub mod validate;
pub mod verify_receipt;

// Re-export diagnostics for testing
pub use pdftract_core::diagnostics::{DiagCode, DiagInfo, Severity, DIAGNOSTIC_CATALOG};

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
