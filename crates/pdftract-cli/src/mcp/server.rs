use crate::mcp::{auth, bind};
use anyhow::Result;
use secrecy::SecretString;
use std::env;

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
///
/// # Returns
/// * Ok(()) if the server started successfully
/// * Err if there was an error (exit code 78 for config errors, 64 for usage errors)
pub fn run(
    bind_addr: String,
    auth_token_file: Option<std::path::PathBuf>,
    auth_token: Option<String>,
) -> Result<()> {
    // Resolve the bearer token
    let token: Option<SecretString> = match auth::resolve_token(
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

    // Check bind security per TH-03
    let has_token = token.is_some();
    if let Err(e) = bind::check_bind_security(&bind_addr, has_token) {
        eprintln!("Error: {}", e);
        std::process::exit(bind::EXIT_CONFIG_ERROR as i32);
    }

    // Report configuration
    if has_token {
        eprintln!("Bearer token provided via secure channel");
    } else {
        eprintln!("No bearer token (loopback-only mode)");
    }
    eprintln!("Bind address: {}", bind_addr);

    // Start the MCP server
    start_server(bind_addr, token)?;

    Ok(())
}

/// Starts the actual MCP server.
///
/// This is a stub implementation. The full MCP server implementation
/// will be done in a separate bead (see plan for MCP server beads).
fn start_server(bind_addr: String, _token: Option<SecretString>) -> Result<()> {
    eprintln!("Starting MCP server on {}...", bind_addr);
    eprintln!("NOTE: Full MCP server implementation is pending (see plan for MCP server beads)");

    // TODO: Implement actual MCP server
    // This will be done in the MCP server implementation beads
    // For now, just sleep to simulate a running server
    eprintln!("Press Ctrl+C to stop the server");

    #[cfg(unix)]
    {
        use std::thread;
        use std::time::Duration;
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }

    #[cfg(not(unix))]
    {
        use std::thread;
        use std::time::Duration;
        loop {
            thread::sleep(Duration::from_secs(1));
        }
    }
}
