use crate::mcp::{auth, bind, http, AuthSource};
use anyhow::{Context, Result};
use secrecy::{ExposeSecret, SecretString};
use sha2::{Digest, Sha256};
use std::env;
use std::path::Path;

/// Runs the MCP server.
///
/// This function:
/// 1. Resolves the bearer token using the priority order defined in the auth module
/// 2. Checks bind security per TH-03 (exits 78 if non-loopback bind without token)
/// 3. Starts the MCP server on the specified bind address
///
/// # Arguments
/// * `bind_addr` - The bind address string (e.g., "127.0.0.1:8080", "0.0.0.0:3000")
/// * `auth_token_file` - Optional path to a file containing the bearer token
/// * `auth_token` - Optional bearer token value (deprecated, requires PDFTRACT_INSECURE_CLI_TOKEN=1)
/// * `max_upload_mb` - Optional maximum request body size in MB (default 256)
/// * `root` - Optional root directory for path-traversal protection
///
/// # Returns
/// * Ok(()) if the server started successfully
/// * Err if there was an error (exit code 78 for config errors, 64 for usage errors)
pub fn run(
    bind_addr: String,
    auth_token_file: Option<std::path::PathBuf>,
    auth_token: Option<String>,
    max_upload_mb: Option<usize>,
    root: Option<std::path::PathBuf>,
) -> Result<()> {
    // Resolve the bearer token
    let token_result: Option<(SecretString, AuthSource)> = match auth::resolve_token(
        auth_token_file.as_deref(),
        env::var("PDFTRACT_MCP_TOKEN").ok(),
        auth_token,
    ) {
        Ok(token) => token,
        Err(e) => {
            eprintln!("Error: {}", e);
            std::process::exit(auth::EXIT_USAGE_ERROR as i32);
        }
    };

    // Extract the token and source, then drop the source to avoid keeping it in memory
    let (token, source) = match token_result {
        Some((t, s)) => (Some(t), Some(s)),
        None => (None, None),
    };

    // Check bind security per TH-03
    let has_token = token.is_some();
    if let Err(e) = bind::check_bind_security(&bind_addr, has_token) {
        eprintln!("Error: {}", e);
        std::process::exit(bind::EXIT_CONFIG_ERROR as i32);
    }

    // Report configuration
    eprintln!("Bind address: {}", bind_addr);
    if let Some(source) = source {
        eprintln!("Bearer token source: {}", source);
        // Log SHA-256 prefix of token for verification without leaking the value
        if let Some(ref token) = token {
            let hash = Sha256::digest(token.expose_secret().as_bytes());
            let prefix = format!("{:x}", hash).chars().take(8).collect::<String>();
            eprintln!("Bearer token SHA-256 prefix: {}", prefix);
        }
    } else {
        eprintln!("No bearer token (loopback-only mode)");
    }

    // Start the HTTP+SSE server (this blocks until shutdown)
    let runtime = tokio::runtime::Runtime::new().context("Failed to create tokio runtime")?;

    runtime.block_on(http::run_server(
        bind_addr,
        token,
        max_upload_mb,
        root.as_deref(),
    ))?;

    Ok(())
}
