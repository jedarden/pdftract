//! MCP tool catalog and registry.
//!
//! This module implements the 10 MCP tools that pdftract exposes via tools/list
//! and tools/call. Each tool wraps an existing pdftract surface with a typed
//! argument schema (JSON Schema via schemars), structured error mapping, and
//! per-invocation observability.

mod args;
mod registry;

pub use registry::{all_tools, Tool, ToolRegistry, ToolResult};

// Error codes for pdftract-specific errors (-32099..-32000)
pub const ERROR_NOT_YET_IMPLEMENTED: i64 = -32000;
pub const ERROR_PDF_ENCRYPTED: i64 = -32001;
pub const ERROR_IO_ERROR: i64 = -32002;
pub const ERROR_PATH_INVALID: i64 = -32003;
pub const ERROR_SSRF_BLOCKED: i64 = -32004;

// Data codes for error responses
pub const CODE_PDF_ENCRYPTED: &str = "PDF_ENCRYPTED";
pub const CODE_IO_ERROR: &str = "IO_ERROR";
pub const CODE_PATH_INVALID: &str = "PATH_INVALID";
pub const CODE_NOT_YET_IMPLEMENTED: &str = "NOT_YET_IMPLEMENTED";
pub const CODE_SSRF_BLOCKED: &str = "SSRF_BLOCKED";

