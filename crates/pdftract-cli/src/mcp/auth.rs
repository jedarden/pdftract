use anyhow::{Context, Result};
use secrecy::SecretString;
use std::env;
use std::fs;
use std::path::Path;

/// Exit code for usage errors (invalid flag combination)
pub const EXIT_USAGE_ERROR: u8 = 64;

/// Minimum recommended token length (bytes)
const MIN_TOKEN_LENGTH: usize = 32;

/// The source of the authentication token.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthSource {
    /// Token from --auth-token-file PATH (recommended)
    TokenFile,
    /// Token from PDFTRACT_MCP_TOKEN environment variable
    EnvVar,
    /// Token from --auth-token VALUE (deprecated, requires PDFTRACT_INSECURE_CLI_TOKEN=1)
    CliInsecure,
}

impl std::fmt::Display for AuthSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AuthSource::TokenFile => write!(f, "--auth-token-file"),
            AuthSource::EnvVar => write!(f, "PDFTRACT_MCP_TOKEN env var"),
            AuthSource::CliInsecure => write!(f, "--auth-token (insecure)"),
        }
    }
}

/// Resolves the MCP bearer token from multiple possible sources.
///
/// Priority order:
/// 1. `--auth-token-file PATH` (reads file, strips terminating newline) — RECOMMENDED
/// 2. `PDFTRACT_MCP_TOKEN` env var
/// 3. `--auth-token VALUE` (only if `PDFTRACT_INSECURE_CLI_TOKEN=1`) — DEPRECATED
/// 4. None
///
/// Returns the token (if any) and the source that provided it.
/// Tokens shorter than 32 characters emit a warning but are accepted
/// to avoid breaking existing deployments.
pub fn resolve_token(
    token_file: Option<&Path>,
    env_token: Option<String>,
    cli_token: Option<String>,
) -> Result<Option<(SecretString, AuthSource)>> {
    // Priority 1: --auth-token-file
    if let Some(path) = token_file {
        let token_content = fs::read_to_string(path)
            .with_context(|| format!("Failed to read token file: {}", path.display()))?;
        let token = token_content.trim_end().to_string();
        check_token_length(&token);
        return Ok(Some((SecretString::new(token.into()), AuthSource::TokenFile)));
    }

    // Priority 2: PDFTRACT_MCP_TOKEN env var
    if let Some(token) = env_token {
        if !token.is_empty() {
            check_token_length(&token);
            return Ok(Some((SecretString::new(token.into()), AuthSource::EnvVar)));
        }
    }

    // Priority 3: --auth-token VALUE (only if PDFTRACT_INSECURE_CLI_TOKEN=1)
    if let Some(token) = cli_token {
        let insecure_allowed = env::var("PDFTRACT_INSECURE_CLI_TOKEN")
            .ok()
            .as_deref()
            == Some("1");

        if !insecure_allowed {
            anyhow::bail!(
                "The --auth-token VALUE flag is REJECTED for security reasons.\n\
                 Use --auth-token-file PATH (RECOMMENDED) or PDFTRACT_MCP_TOKEN env var instead.\n\
                 To use this insecure flag anyway, set PDFTRACT_INSECURE_CLI_TOKEN=1."
            );
        }

        eprintln!(
            "WARNING: Using --auth-token VALUE is INSECURE. The token is visible in process listings.\n\
             Recommended: Use --auth-token-file PATH or PDFTRACT_MCP_TOKEN env var."
        );
        check_token_length(&token);
        return Ok(Some((SecretString::new(token.into()), AuthSource::CliInsecure)));
    }

    // No token provided
    Ok(None)
}

/// Emits a warning if the token is shorter than the recommended minimum length.
fn check_token_length(token: &str) {
    if token.len() < MIN_TOKEN_LENGTH {
        eprintln!(
            "WARNING: Token length is {} bytes, which is below the recommended minimum of {} bytes. \
             Consider using a longer token for better security.",
            token.len(),
            MIN_TOKEN_LENGTH
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::ExposeSecret;
    use std::fs::write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_resolve_token_priority_file_first() {
        let temp_file = NamedTempFile::new().unwrap();
        write(temp_file.path(), "file-token\n").unwrap();

        let (token, source) = resolve_token(
            Some(temp_file.path()),
            Some("env-token".to_string()),
            Some("cli-token".to_string()),
        )
        .unwrap()
        .unwrap();

        assert_eq!(token.expose_secret(), "file-token");
        assert_eq!(source, AuthSource::TokenFile);
    }

    #[test]
    fn test_resolve_token_priority_env_second() {
        let (token, source) = resolve_token(
            None,
            Some("env-token".to_string()),
            Some("cli-token".to_string()),
        )
        .unwrap()
        .unwrap();

        assert_eq!(token.expose_secret(), "env-token");
        assert_eq!(source, AuthSource::EnvVar);
    }

    #[test]
    fn test_resolve_token_rejects_cli_token_without_insecure_flag() {
        let result = resolve_token(None, None, Some("cli-token".to_string()));
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("REJECTED"));
    }

    #[test]
    fn test_resolve_token_accepts_cli_token_with_insecure_flag() {
        env::set_var("PDFTRACT_INSECURE_CLI_TOKEN", "1");
        let (token, source) = resolve_token(None, None, Some("cli-token".to_string()))
            .unwrap()
            .unwrap();
        env::remove_var("PDFTRACT_INSECURE_CLI_TOKEN");

        assert_eq!(token.expose_secret(), "cli-token");
        assert_eq!(source, AuthSource::CliInsecure);
    }

    #[test]
    fn test_resolve_token_none() {
        let token = resolve_token(None, None, None).unwrap();
        assert!(token.is_none());
    }

    #[test]
    fn test_resolve_token_empty_env_var() {
        let token = resolve_token(None, Some("".to_string()), None).unwrap();
        assert!(token.is_none());
    }

    #[test]
    fn test_resolve_token_file_strips_newline() {
        let temp_file = NamedTempFile::new().unwrap();
        write(temp_file.path(), "token-with-newline\n").unwrap();

        let (token, source) = resolve_token(Some(temp_file.path()), None, None)
            .unwrap()
            .unwrap();

        assert_eq!(token.expose_secret(), "token-with-newline");
        assert_eq!(source, AuthSource::TokenFile);
    }

    #[test]
    fn test_resolve_token_short_token_warning() {
        let temp_file = NamedTempFile::new().unwrap();
        write(temp_file.path(), "short").unwrap();

        // Should succeed but emit warning (captured in test output)
        let (token, source) = resolve_token(Some(temp_file.path()), None, None)
            .unwrap()
            .unwrap();

        assert_eq!(token.expose_secret(), "short");
        assert_eq!(source, AuthSource::TokenFile);
    }
}
