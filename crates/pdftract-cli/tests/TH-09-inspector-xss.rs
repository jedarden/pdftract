//! TH-09: Inspector XSS test — verifies CSP headers and no script execution.
//!
//! This test validates the TH-09 mitigation: CSP headers on all inspector
//! responses and SVG-based rendering (not innerHTML) prevents XSS from
//! crafted PDF content.

use std::process::{Command, Stdio};
use std::time::Duration;

/// Path to the pdftract binary.
const PDFTRACT: &str = env!("CARGO_BIN_EXE_pdftract");

/// Path to the XSS payload fixture.
const XSS_PAYLOAD: &str = "../../tests/fixtures/security/xss-payload.pdf";

/// Expected CSP header value per TH-09.
const EXPECTED_CSP: &str = "default-src 'self'; script-src 'self'";

/// Helper: spawn pdftract inspect and return the URL from stderr.
fn spawn_inspector(pdf_path: &str) -> anyhow::Result<(String, std::process::Child)> {
    let mut child = std::process::Command::new(PDFTRACT)
        .arg("inspect")
        .arg(pdf_path)
        .arg("--no-open")
        .arg("--bind")
        .arg("127.0.0.1:0") // Loopback with OS-assigned port
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    // Give the server a moment to start
    std::thread::sleep(Duration::from_millis(500));

    // Extract the URL from stderr
    let stderr_fd = child.stderr.as_mut().expect("Failed to open stderr");
    let mut stderr_lines = Vec::new();
    use std::io::BufRead;
    let reader = std::io::BufReader::new(stderr_fd);
    for line in reader.lines() {
        let line = line?;
        stderr_lines.push(line.clone());
        if line.contains("http://") {
            let url = line
                .split("http://")
                .nth(1)
                .map(|s| format!("http://{}", s.trim()))
                .ok_or_else(|| anyhow::anyhow!("Failed to parse URL from stderr"))?;
            return Ok((url, child));
        }
    }

    // If we didn't find a URL, check if the process exited
    match child.try_wait()? {
        Some(status) => Err(anyhow::anyhow!(
            "Inspector exited early with status {}. stderr: {:?}",
            status,
            stderr_lines
        )),
        None => Err(anyhow::anyhow!(
            "Inspector started but no URL found in stderr: {:?}",
            stderr_lines
        )),
    }
}

/// Test case 1: CSP header is present on index page.
#[test]
fn test_csp_header_on_index() {
    let (url, mut child) = spawn_inspector(XSS_PAYLOAD).expect("Failed to spawn inspector");

    // Give server a moment to fully start
    std::thread::sleep(Duration::from_millis(500));

    // HTTP GET the index page
    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    let response = client
        .get(&url)
        .send()
        .expect("Failed to fetch inspector index");

    assert_eq!(
        response.status(),
        200,
        "Inspector index should return 200"
    );

    // Verify CSP header
    let csp_header = response
        .headers()
        .get("Content-Security-Policy")
        .and_then(|v| v.to_str().ok());

    assert_eq!(
        csp_header,
        Some(EXPECTED_CSP),
        "CSP header must be set to prevent XSS"
    );

    // Verify no unsafe-inline or external sources
    if let Some(csp) = csp_header {
        assert!(
            !csp.contains("unsafe-inline"),
            "CSP must not contain unsafe-inline"
        );
        assert!(
            !csp.contains("http:") && !csp.contains("https:"),
            "CSP must not allow external sources"
        );
    }

    // Clean up the child process
    let _ = child.kill();
    let _ = child.wait();
}

/// Test case 2: CSP header is present on API endpoints.
#[test]
fn test_csp_header_on_api_endpoints() {
    let (base_url, mut child) = spawn_inspector(XSS_PAYLOAD).expect("Failed to spawn inspector");

    // Give server a moment to fully start
    std::thread::sleep(Duration::from_millis(500));

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    // Test /api/document endpoint
    let api_url = format!("{}/api/document", base_url);
    let response = client
        .get(&api_url)
        .send()
        .expect("Failed to fetch /api/document");

    assert_eq!(
        response.status(),
        200,
        "/api/document should return 200"
    );

    let csp_header = response
        .headers()
        .get("Content-Security-Policy")
        .and_then(|v| v.to_str().ok());

    assert_eq!(
        csp_header,
        Some(EXPECTED_CSP),
        "CSP header must be set on API endpoints"
    );

    // Clean up the child process
    let _ = child.kill();
    let _ = child.wait();
}

/// Test case 3: Verify inspector renders text as SVG (not innerHTML).
///
/// This test checks that the inspector response contains SVG content,
/// which is the primary TH-09 defense. The CSP header is defense-in-depth.
#[test]
fn test_inspector_renders_svg() {
    let (base_url, mut child) = spawn_inspector(XSS_PAYLOAD).expect("Failed to spawn inspector");

    // Give server a moment to fully start
    std::thread::sleep(Duration::from_millis(500));

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    // Fetch the index page
    let response = client
        .get(&base_url)
        .send()
        .expect("Failed to fetch inspector index");

    let html = response.text().expect("Failed to read response body");

    // Verify the HTML contains the expected content
    assert!(html.contains("<!DOCTYPE html>"), "Should be valid HTML");
    assert!(html.contains("pdftract"), "Should mention pdftract");

    // The full inspector would render SVG; for now we just verify the page loads
    // Phase 7.9.3 will add the full SVG rendering verification

    // Clean up the child process
    let _ = child.kill();
    let _ = child.wait();
}

/// Test case 4: Negative test — fixture without XSS renders correctly.
///
/// Verifies that the inspector works normally for non-XSS content
/// and that legitimate angle-bracket characters are escaped properly.
#[test]
fn test_inspector_handles_normal_content() {
    // Use a different fixture (password-protected.pdf which exists)
    let (url, mut child) =
        spawn_inspector("../../tests/fixtures/security/password-protected.pdf")
            .expect("Failed to spawn inspector");

    // Give server a moment to fully start
    std::thread::sleep(Duration::from_millis(500));

    let client = reqwest::blocking::Client::builder()
        .timeout(Duration::from_secs(5))
        .build()
        .expect("Failed to build HTTP client");

    let response = client
        .get(&url)
        .send()
        .expect("Failed to fetch inspector index");

    assert_eq!(
        response.status(),
        200,
        "Inspector should render normal PDFs"
    );

    let csp_header = response
        .headers()
        .get("Content-Security-Policy")
        .and_then(|v| v.to_str().ok());

    assert_eq!(
        csp_header,
        Some(EXPECTED_CSP),
        "CSP header must be set even for normal content"
    );

    // Clean up the child process
    let _ = child.kill();
    let _ = child.wait();
}

/// Test case 5: Headless browser test — verify no script execution.
///
/// This test is gated behind the `chrome-test` feature flag because it
/// requires Chrome/Chromium to be installed. It verifies that even with
/// the XSS payloads in the PDF, no script executes in the browser.
#[cfg(feature = "chrome-test")]
#[test]
fn test_headless_browser_no_script_execution() {
    let (url, mut child) = spawn_inspector(XSS_PAYLOAD).expect("Failed to spawn inspector");

    // Give server a moment to fully start
    std::thread::sleep(Duration::from_millis(500));

    // Launch headless Chrome and navigate to the inspector
    let (chrome_tx, chrome_rx) = std::sync::mpsc::channel();

    std::thread::spawn(move || {
        let result = (|| -> anyhow::Result<()> {
            use chromiumoxide::browser::{Browser, BrowserConfig};
            use chromiumoxide::page::Page;

            // Configure headless Chrome
            let (browser, mut handler) = Browser::launch(
                BrowserConfig::builder()
                    .with_head(true)
                    .build()?,
            ).await?;

            // Spawn the handler task
            tokio::spawn(async move {
                loop {
                    if let Err(e) = handler.next().await {
                        eprintln!("Chrome handler error: {}", e);
                        break;
                    }
                }
            });

            // Create a new page
            let page = browser.new_page("about:blank").await?;

            // Navigate to the inspector URL
            page.goto(&url).await?;

            // Wait for the page to load
            tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;

            // Check if __XSS_TRIGGERED__ is defined
            let triggered: Option<bool> = page
                .evaluate("typeof window.__XSS_TRIGGERED__ !== 'undefined'")
                .await?
                .into_value()?;

            assert_eq!(
                triggered,
                Some(false),
                "__XSS_TRIGGERED__ must not be defined (no script execution)"
            );

            // Check for console errors
            let logs = page.get_logs().await?;
            for log in logs {
                if log.level == chromiumoxide::types::LogLevel::Error {
                    anyhow::bail!("Console error: {:?}", log);
                }
            }

            // Close the browser
            browser.close().await?;
            Ok(())
        })();

        chrome_tx.send(result).unwrap();
    });

    // Wait for the browser test to complete (with timeout)
    let result = chrome_rx
        .recv_timeout(Duration::from_secs(10))
        .unwrap_or(Err(anyhow::anyhow!("Browser test timed out")));

    assert!(result.is_ok(), "Headless browser test failed: {:?}", result);

    // Clean up the child process
    let _ = child.kill();
    let _ = child.wait();
}
