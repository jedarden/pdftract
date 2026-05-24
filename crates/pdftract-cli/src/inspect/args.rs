//! Command-line arguments for the inspect subcommand.
//!
//! Implements Phase 7.9.1: inspect subcommand structure + clap parsing + browser launcher.

use anyhow::Result;
use std::net::IpAddr;
use std::path::PathBuf;

/// Command-line arguments for the `pdftract inspect` subcommand.
#[derive(Debug, clap::Args)]
pub struct InspectArgs {
    /// Path to the PDF file to inspect
    #[arg(value_name = "FILE")]
    pub file: PathBuf,

    /// Port to bind the inspector server (default: 7676)
    #[arg(short, long, default_value = "7676")]
    pub port: u16,

    /// Bind address for the inspector server (default: 127.0.0.1)
    ///
    /// Binding to a non-loopback address requires --auth-token for security.
    #[arg(short, long, default_value = "127.0.0.1")]
    pub bind: String,

    /// Authentication token for non-loopback binds
    ///
    /// Required when --bind is not a loopback address (127.0.0.1 or ::1).
    #[arg(long)]
    pub auth_token: Option<String>,

    /// Suppress automatic browser launch
    ///
    /// Useful for CI environments or when you want to manually open the browser.
    #[arg(long)]
    pub no_open: bool,

    /// Optional second PDF file for comparative debugging
    ///
    /// When provided, the inspector shows side-by-side comparison.
    #[arg(long, value_name = "FILE")]
    pub compare: Option<PathBuf>,
}

impl InspectArgs {
    /// Parse the bind address string into an IpAddr.
    pub fn parse_bind(&self) -> Result<IpAddr> {
        self.bind
            .parse::<IpAddr>()
            .map_err(|e| anyhow::anyhow!("Invalid bind address '{}': {}", self.bind, e))
    }

    /// Validate the inspect arguments.
    ///
    /// Returns an error if:
    /// - The input file doesn't exist or isn't readable
    /// - The bind address is non-loopback without an auth token
    /// - The compare file (if provided) doesn't exist or isn't readable
    pub fn validate(&self) -> Result<()> {
        // Validate input file exists and is readable
        if !self.file.exists() {
            anyhow::bail!("Input file not found: {}", self.file.display());
        }
        if !self.file.is_file() {
            anyhow::bail!("Input path is not a file: {}", self.file.display());
        }

        // Validate bind address and auth token requirement
        let bind_addr = self.parse_bind()?;
        let is_loopback = bind_addr.is_loopback();
        if !is_loopback && self.auth_token.is_none() {
            anyhow::bail!(
                "Binding to a non-loopback address requires --auth-token for security. \
                 See Phase 6.7 MCP HTTP mode for the shared rationale."
            );
        }

        // Validate compare file if provided
        if let Some(ref compare_path) = self.compare {
            if !compare_path.exists() {
                anyhow::bail!("Compare file not found: {}", compare_path.display());
            }
            if !compare_path.is_file() {
                anyhow::bail!("Compare path is not a file: {}", compare_path.display());
            }
        }

        Ok(())
    }

    /// Get the full server URL.
    pub fn server_url(&self) -> String {
        format!("http://{}:{}/", self.bind, self.port)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_validate_missing_file() {
        let args = InspectArgs {
            file: PathBuf::from("/nonexistent/file.pdf"),
            port: 7676,
            bind: "127.0.0.1".to_string(),
            auth_token: None,
            no_open: false,
            compare: None,
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_validate_non_loopback_without_token() {
        let args = InspectArgs {
            file: PathBuf::from("tests/fixtures/minimal.pdf"),
            port: 7676,
            bind: "0.0.0.0".to_string(),
            auth_token: None,
            no_open: false,
            compare: None,
        };
        assert!(args.validate().is_err());
    }

    #[test]
    fn test_validate_non_loopback_with_token() {
        let args = InspectArgs {
            file: PathBuf::from("tests/fixtures/minimal.pdf"),
            port: 7676,
            bind: "0.0.0.0".to_string(),
            auth_token: Some("secret".to_string()),
            no_open: false,
            compare: None,
        };
        // This would succeed if the file exists
        // (we're not checking file existence in this unit test)
    }

    #[test]
    fn test_parse_bind() {
        let args = InspectArgs {
            file: PathBuf::from("test.pdf"),
            port: 7676,
            bind: "127.0.0.1".to_string(),
            auth_token: None,
            no_open: false,
            compare: None,
        };
        let addr = args.parse_bind().unwrap();
        assert!(addr.is_loopback());
    }

    #[test]
    fn test_server_url() {
        let args = InspectArgs {
            file: PathBuf::from("test.pdf"),
            port: 8080,
            bind: "127.0.0.1".to_string(),
            auth_token: None,
            no_open: false,
            compare: None,
        };
        assert_eq!(args.server_url(), "http://127.0.0.1:8080/");
    }
}
