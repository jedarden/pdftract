use anyhow::{bail, Context, Result};
use std::net::{SocketAddr, ToSocketAddrs};

/// Exit code for configuration errors (sysexits.h EX_CONFIG)
pub const EXIT_CONFIG_ERROR: u8 = 78;

/// Checks whether binding to the given address is secure.
///
/// Per TH-03:
/// - If the resolved address is loopback (127.0.0.0/8 or ::1) AND no token is provided -> OK
/// - If the resolved address is non-loopback AND no token is provided -> ERROR (exit 78)
/// - If a token is provided -> OK regardless of address
///
/// This check MUST run BEFORE the listener binds to avoid exposing an unauthenticated
/// service during the failure window.
///
/// # Arguments
/// * `bind_addr` - The bind address string (e.g., "0.0.0.0:8080", "[::1]:9000", "localhost:3000")
/// * `has_token` - Whether a bearer token was provided
///
/// # Returns
/// * Ok(()) if binding is permitted
/// * Err if binding should be refused (exit code 78)
pub fn check_bind_security(bind_addr: &str, has_token: bool) -> Result<()> {
    // If a token is provided, any bind address is acceptable
    if has_token {
        return Ok(());
    }

    // Resolve the bind address
    let is_loopback = is_bind_addr_loopback(bind_addr)?;

    if is_loopback {
        // Loopback addresses are exempt from the token requirement
        Ok(())
    } else {
        // Non-loopback bind without a token is a security violation (TH-03)
        bail!(
            "ERROR: pdftract mcp --bind {} requires --auth-token-file PATH or PDFTRACT_MCP_TOKEN env \
             (loopback addresses 127.0.0.1 / ::1 exempt). Refusing to bind to {} without authentication.",
            bind_addr, bind_addr
        );
    }
}

/// Determines whether a bind address string resolves to a loopback address.
///
/// This function:
/// 1. Parses the bind address
/// 2. Resolves hostnames via DNS (for hostnames like "localhost")
/// 3. Returns true ONLY if ALL resolved addresses are loopback
/// 4. Fails closed: if resolution fails or returns mixed addresses, returns false
///
/// # Arguments
/// * `bind_addr` - The bind address string
///
/// # Returns
/// * Ok(true) if the address is definitely loopback
/// * Ok(false) if the address is definitely non-loopback or resolution failed
fn is_bind_addr_loopback(bind_addr: &str) -> Result<bool> {
    // Try to parse as a SocketAddr first (handles IP:PORT directly)
    if let Ok(addr) = bind_addr.parse::<SocketAddr>() {
        return Ok(addr.ip().is_loopback());
    }

    // If not a direct SocketAddr, try to resolve as a hostname
    let addrs: Vec<SocketAddr> = bind_addr
        .to_socket_addrs()
        .with_context(|| format!("Failed to resolve bind address: {}", bind_addr))?
        .collect();

    if addrs.is_empty() {
        // Resolution failed - fail closed
        return Ok(false);
    }

    // ALL resolved addresses must be loopback for the hostname to be considered loopback
    // A hostname that resolves to mixed loopback + non-loopback MUST be treated as non-loopback
    Ok(addrs.iter().all(|addr| addr.ip().is_loopback()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_check_bind_security_with_token_allows_any_address() {
        // With a token, any bind address should be allowed
        assert!(check_bind_security("0.0.0.0:8080", true).is_ok());
        assert!(check_bind_security("[::]:9000", true).is_ok());
        assert!(check_bind_security("192.168.1.1:3000", true).is_ok());
    }

    #[test]
    fn test_check_bind_security_loopback_without_token() {
        // Loopback addresses should be allowed without a token
        assert!(check_bind_security("127.0.0.1:8080", false).is_ok());
        assert!(check_bind_security("127.0.0.2:9000", false).is_ok());
        assert!(check_bind_security("[::1]:3000", false).is_ok());
        assert!(check_bind_security("localhost:4000", false).is_ok());
    }

    #[test]
    fn test_check_bind_security_non_loopback_without_token_fails() {
        // Non-loopback addresses should fail without a token
        let result = check_bind_security("0.0.0.0:8080", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires --auth-token-file"));

        let result = check_bind_security("192.168.1.1:3000", false);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("requires --auth-token-file"));
    }

    #[test]
    fn test_is_bind_addr_loopback_ipv4() {
        assert!(is_bind_addr_loopback("127.0.0.1:8080").unwrap());
        assert!(is_bind_addr_loopback("127.0.0.2:9000").unwrap());
        assert!(is_bind_addr_loopback("127.255.255.255:3000").unwrap());
    }

    #[test]
    fn test_is_bind_addr_loopback_ipv6() {
        assert!(is_bind_addr_loopback("[::1]:8080").unwrap());
    }

    #[test]
    fn test_is_bind_addr_loopback_non_loopback() {
        assert!(!is_bind_addr_loopback("0.0.0.0:8080").unwrap());
        assert!(!is_bind_addr_loopback("192.168.1.1:3000").unwrap());
        assert!(!is_bind_addr_loopback("10.0.0.1:9000").unwrap());
        assert!(!is_bind_addr_loopback("[::]:3000").unwrap());
        assert!(!is_bind_addr_loopback("[2001:db8::1]:8080").unwrap());
    }

    #[test]
    fn test_is_bind_addr_loopback_hostname() {
        // "localhost" typically resolves to 127.0.0.1 and/or ::1
        // This test may fail on systems with unusual /etc/hosts configurations
        let result = is_bind_addr_loopback("localhost:8080");
        // We don't assert the exact result since it depends on system config
        // but the function should not panic or return an error
        assert!(result.is_ok());
    }

    #[test]
    fn test_is_bind_addr_loopback_invalid_address() {
        // Invalid addresses should fail (return Err)
        assert!(is_bind_addr_loopback("invalid:address").is_err());
        // Invalid IP addresses may resolve to error or return false depending on system
        let result = is_bind_addr_loopback("999.999.999.999:8080");
        // Either is acceptable - fail closed
        assert!(result.is_err() || result.unwrap() == false);
    }
}
