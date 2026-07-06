//! TLS failure tests for remote source adapter (Phase 1.8).
//!
//! These tests verify that TLS handshake failures produce clear error messages
//! and the correct exit code (6) for certificate failures.

#![cfg(feature = "remote")]

use pdftract_core::source::{open_remote, RemoteOpts};
use std::io;

/// Test 1: TLS handshake with self-signed cert (via badssl.com).
///
/// Note: ureq's rustls backend rejects self-signed certs by default.
/// This test verifies that we get a clear TLS error message.
#[tokio::test]
async fn test_tls_self_signed_cert_rejected() {
    // Use badssl.com's self-signed cert endpoint
    let url = "https://self-signed.badssl.com/";
    let opts = RemoteOpts::new();

    // TLS handshake should fail due to self-signed cert
    let result = open_remote(url, &opts, None);

    // Should fail with a TLS-related error
    assert!(result.is_err(), "Self-signed cert should be rejected");

    if let Err(e) = result {
        // Should be PermissionDenied (TLS failure) or a transport error
        let kind = e.kind();
        assert!(
            kind == io::ErrorKind::PermissionDenied || kind == io::ErrorKind::Other,
            "TLS failure should return PermissionDenied or Other, got: {:?}",
            kind
        );

        // Error message should mention TLS or certificate
        let msg = e.to_string().to_lowercase();
        assert!(
            msg.contains("tls")
                || msg.contains("certificate")
                || msg.contains("handshake")
                || msg.contains("verify"),
            "Error message should mention TLS/certificate/handshake/verify, got: {}",
            e
        );
    }
}

/// Test 2: TLS handshake with expired cert (via badssl.com).
#[tokio::test]
async fn test_tls_expired_cert_rejected() {
    // Use badssl.com's expired cert endpoint
    let url = "https://expired.badssl.com/";
    let opts = RemoteOpts::new();

    // TLS handshake should fail due to expired cert
    let result = open_remote(url, &opts, None);

    assert!(result.is_err(), "Expired cert should be rejected");

    if let Err(e) = result {
        let msg = e.to_string().to_lowercase();
        assert!(
            msg.contains("tls")
                || msg.contains("certificate")
                || msg.contains("expired")
                || msg.contains("valid"),
            "Error message should mention TLS/certificate/expired/valid, got: {}",
            e
        );
    }
}

/// Test 3: TLS handshake with wrong host cert (via badssl.com).
#[tokio::test]
async fn test_tls_wrong_host_rejected() {
    // Use badssl.com's wrong host endpoint
    let url = "https://wrong.host.badssl.com/";
    let opts = RemoteOpts::new();

    let result = open_remote(url, &opts, None);

    // Should fail due to hostname mismatch
    assert!(result.is_err());

    if let Err(e) = result {
        let msg = e.to_string().to_lowercase();
        // The error should be related to TLS validation
        assert!(
            msg.contains("tls")
                || msg.contains("certificate")
                || msg.contains("host")
                || msg.contains("verify"),
            "Error should mention TLS/certificate/host/verify, got: {}",
            e
        );
    }
}

/// Test 4: Verify TLS error produces exit code 6 (via error kind).
#[tokio::test]
async fn test_tls_error_exit_code() {
    // Use a known HTTPS endpoint with invalid cert
    let url = "https://expired.badssl.com/";
    let opts = RemoteOpts::new();

    let result = open_remote(url, &opts, None);

    if let Err(e) = result {
        // TLS errors should produce PermissionDenied kind
        // The CLI maps PermissionDenied to exit code 6
        assert_eq!(
            e.kind(),
            io::ErrorKind::PermissionDenied,
            "TLS failure should produce PermissionDenied error kind for exit code 6"
        );
    }
}

/// Test 5: Verify valid HTTPS works (via badssl.com).
#[tokio::test]
#[ignore = "Requires full internet access - may be flaky in CI"]
async fn test_tls_valid_cert_works() {
    // Use badssl.com's valid cert endpoint
    let url = "https://sha256.badssl.com/";
    let opts = RemoteOpts::new();

    let result = open_remote(url, &opts, None);

    // This should work or at least get past TLS validation
    // (might fail due to not being a PDF, but TLS should succeed)
    if let Err(e) = result {
        let msg = e.to_string().to_lowercase();
        // Should NOT be a TLS/certificate error
        assert!(
            !msg.contains("tls") && !msg.contains("certificate") && !msg.contains("handshake"),
            "Valid HTTPS should not trigger TLS errors, got: {}",
            e
        );
    }
}

/// Test 6: TLS connection timeout.
#[tokio::test]
async fn test_tls_connection_timeout() {
    // Use a non-routable IP to trigger timeout
    let url = "https://192.0.2.1/test.pdf"; // TEST-NET-1, never routable

    let opts = RemoteOpts::new();
    let result = open_remote(url, &opts, None);

    assert!(result.is_err());

    if let Err(e) = result {
        // Should be a timeout or connection error
        let kind = e.kind();
        assert!(
            kind == io::ErrorKind::TimedOut || kind == io::ErrorKind::Interrupted,
            "Connection timeout should produce TimedOut or Interrupted, got: {:?}",
            kind
        );
    }
}

/// Test 7: Verify INV-8 - no panic on TLS errors.
#[tokio::test]
async fn test_inv8_no_panic_on_tls_errors() {
    let result = std::panic::catch_unwind(|| {
        let rt = tokio::runtime::Runtime::new().unwrap();
        rt.block_on(async {
            let opts = RemoteOpts::new();
            let _ = open_remote("https://expired.badssl.com/", &opts, None);
        });
    });

    assert!(result.is_ok(), "Should not panic on TLS errors");
}

/// Test 8: Verify that HTTP URLs don't trigger TLS validation.
#[tokio::test]
#[cfg(feature = "remote")]
async fn test_http_no_tls_validation() {
    use wiremock::{
        matchers::{method, path},
        Mock, MockServer, ResponseTemplate,
    };

    let mock_server = MockServer::start().await;

    Mock::given(method("HEAD"))
        .and(path("/test.pdf"))
        .respond_with(
            ResponseTemplate::new(200)
                .insert_header("Content-Length", "1000")
                .insert_header("Accept-Ranges", "bytes")
                .insert_header("Content-Type", "application/pdf")
                .set_body_bytes(""),
        )
        .mount(&mock_server)
        .await;

    // Get the HTTP URL from wiremock
    let url = format!("{}/test.pdf", mock_server.uri());

    // Verify it's HTTP, not HTTPS
    assert!(
        url.starts_with("http://"),
        "Wiremock should provide HTTP URLs"
    );

    let opts = RemoteOpts::new();
    let result = open_remote(&url, &opts, None);

    // HTTP should work (no TLS validation needed)
    // Note: This test verifies that we correctly distinguish HTTP vs HTTPS URLs
    if let Err(e) = result {
        // If it fails, it shouldn't be a TLS error
        let msg = e.to_string().to_lowercase();
        assert!(
            !msg.contains("tls") && !msg.contains("certificate") && !msg.contains("handshake"),
            "HTTP URLs should not trigger TLS validation errors, got: {}",
            e
        );
    }
}
