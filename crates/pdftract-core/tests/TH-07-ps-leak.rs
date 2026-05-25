//! TH-07: Password disclosure via process arg list (`ps aux`)
//!
//! This test validates that the PDF password ingress channels properly prevent
//! password disclosure via the process arg list. Specifically:
//!
//! 1. `--password VALUE` is rejected by default (exit 64)
//! 2. `--password VALUE` with `PDFTRACT_INSECURE_CLI_PASSWORD=1` proceeds with warning
//! 3. `--password-stdin` works correctly
//! 4. `PDFTRACT_PASSWORD` env var works correctly
//! 5. Under opt-in, password IS visible in /proc/<pid>/cmdline (proving the leak)
//! 6. Under --password-stdin or env var, password is NOT in /proc/<pid>/cmdline

use std::path::PathBuf;
use std::process::Command;

/// Test password used throughout.
const TEST_PASSWORD: &str = "secret123";

/// Get the path to a fixture file, handling both workspace and crate test locations.
fn get_fixture_path(fixture_name: &str) -> PathBuf {
    // Try workspace root first (when running from workspace)
    let workspace_path = PathBuf::from(format!("tests/fixtures/{}", fixture_name));
    if workspace_path.exists() {
        return workspace_path;
    }

    // Try from crate directory (when running from crate tests)
    let crate_path = PathBuf::from(format!("../../tests/fixtures/{}", fixture_name));
    if crate_path.exists() {
        return crate_path;
    }

    // Fall back to workspace path (will fail with a clear error)
    workspace_path
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Test case 1: --password VALUE is rejected without opt-in (exit 64).
    #[test]
    fn test_password_value_rejected_without_opt_in() {
        let fixture_path = get_fixture_path("security/password-protected.pdf");
        let output = Command::new("pdftract")
            .arg("extract")
            .arg("--password")
            .arg(TEST_PASSWORD)
            .arg(fixture_path)
            .arg("--output")
            .arg("-")
            .output()
            .expect("Failed to execute pdftract");

        // Should exit with code 64 (usage error)
        assert_eq!(
            output.status.code(), Some(64),
            "Expected exit code 64, got {:?}",
            output.status.code()
        );

        let stderr = String::from_utf8_lossy(&output.stderr);
        // Should mention --password-stdin
        assert!(
            stderr.contains("--password-stdin"),
            "stderr should mention --password-stdin, got: {}",
            stderr
        );
        // Should mention PDFTRACT_PASSWORD
        assert!(
            stderr.contains("PDFTRACT_PASSWORD"),
            "stderr should mention PDFTRACT_PASSWORD, got: {}",
            stderr
        );
        // Should mention "insecure"
        assert!(
            stderr.contains("insecure"),
            "stderr should mention 'insecure', got: {}",
            stderr
        );
    }

    /// Test case 2: --password VALUE with opt-in proceeds with warning.
    #[test]
    fn test_password_value_accepted_with_opt_in() {
        let fixture_path = get_fixture_path("security/password-protected.pdf");
        let output = Command::new("pdftract")
            .arg("extract")
            .arg("--password")
            .arg(TEST_PASSWORD)
            .arg(fixture_path)
            .arg("--output")
            .arg("-")
            .env("PDFTRACT_INSECURE_CLI_PASSWORD", "1")
            .output()
            .expect("Failed to execute pdftract");

        let stderr = String::from_utf8_lossy(&output.stderr);

        // Should NOT exit with code 64 (may succeed or fail with password error 66)
        assert_ne!(
            output.status.code(), Some(64),
            "Should not exit with 64 when opt-in is set, stderr: {}",
            stderr
        );

        // Should contain WARNING about ps aux
        assert!(
            stderr.contains("WARNING") && stderr.contains("ps aux"),
            "stderr should contain WARNING about ps aux, got: {}",
            stderr
        );
    }

    /// Test case 3: --password-stdin works correctly.
    #[test]
    fn test_password_stdin_works() {
        let fixture_path = get_fixture_path("security/password-protected.pdf");
        // Use the `echo` command to pipe the password to pdftract
        // Note: This is a basic test - full integration would require
        // more complex stdin handling
        let output = Command::new("sh")
            .arg("-c")
            .arg(&format!(
                "echo '{}' | pdftract extract --password-stdin {} --output -",
                TEST_PASSWORD, fixture_path.display()
            ))
            .output()
            .expect("Failed to execute pdftract with --password-stdin");

        // The command should execute (may fail with password error if PDF is actually encrypted)
        // but should NOT exit with 64
        assert_ne!(
            output.status.code(), Some(64),
            "--password-stdin should not be rejected, got exit code {:?}",
            output.status.code()
        );
    }

    /// Test case 4: PDFTRACT_PASSWORD env var works correctly.
    #[test]
    fn test_password_env_var_works() {
        let fixture_path = get_fixture_path("security/password-protected.pdf");
        let output = Command::new("pdftract")
            .arg("extract")
            .arg(fixture_path)
            .arg("--output")
            .arg("-")
            .env("PDFTRACT_PASSWORD", TEST_PASSWORD)
            .output()
            .expect("Failed to execute pdftract");

        // Should NOT exit with code 64
        assert_ne!(
            output.status.code(), Some(64),
            "PDFTRACT_PASSWORD should not be rejected, got exit code {:?}",
            output.status.code()
        );
    }

    /// Test case 5: Verify that --password VALUE leaks in /proc/<pid>/cmdline (Linux only).
    ///
    /// This is the POSITIVE test: we verify that the password DOES appear in the
    /// command line when using --password VALUE with opt-in. This proves that
    /// the leak exists, which is why we reject it by default.
    #[cfg(target_os = "linux")]
    #[test]
    fn test_password_leaks_in_cmdline_with_opt_in() {
        use std::fs;
        use std::thread;
        use std::time::Duration;

        // Spawn the process in the background
        let fixture_path = get_fixture_path("security/password-protected.pdf");
        let mut child = Command::new("pdftract")
            .arg("extract")
            .arg("--password")
            .arg(TEST_PASSWORD)
            .arg(fixture_path)
            .arg("--output")
            .arg("-")
            .env("PDFTRACT_INSECURE_CLI_PASSWORD", "1")
            .spawn()
            .expect("Failed to spawn pdftract");

        let pid = child.id();

        // Read /proc/<pid>/cmdline with retries
        // The process might exit quickly, so we need to read ASAP
        let cmdline_path = format!("/proc/{}/cmdline", pid);
        let mut cmdline = String::new();
        let max_retries = 10;

        for i in 0..max_retries {
            thread::sleep(Duration::from_millis(i * 10));
            match fs::read_to_string(&cmdline_path) {
                Ok(content) => {
                    cmdline = content;
                    break;
                }
                Err(_) if i < max_retries - 1 => continue,
                Err(e) => panic!("Failed to read {} after {} retries: {}", cmdline_path, max_retries, e),
            }
        }

        // Verify that the password appears in the command line
        // (cmdline is null-separated, so we check for the password string)
        assert!(
            cmdline.contains(TEST_PASSWORD),
            "Password '{}' should appear in cmdline when using --password VALUE. cmdline: {}",
            TEST_PASSWORD,
            cmdline.replace('\0', " ")
        );

        // Clean up the child process
        let _ = child.kill();
        let _ = child.wait();
    }

    /// Test case 6: Verify that --password-stdin does NOT leak password in /proc/<pid>/cmdline (Linux only).
    #[cfg(target_os = "linux")]
    #[test]
    fn test_password_stdin_does_not_leak_in_cmdline() {
        use std::fs;
        use std::thread;
        use std::time::Duration;

        // Spawn the process with --password-stdin
        let fixture_path = get_fixture_path("security/password-protected.pdf");
        let mut child = Command::new("pdftract")
            .arg("extract")
            .arg("--password-stdin")
            .arg(fixture_path)
            .arg("--output")
            .arg("-")
            .stdin(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn pdftract");

        let pid = child.id();

        // Give the process a moment to start
        thread::sleep(Duration::from_millis(100));

        // Read /proc/<pid>/cmdline
        let cmdline_path = format!("/proc/{}/cmdline", pid);
        let cmdline = fs::read_to_string(&cmdline_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", cmdline_path, e));

        // Verify that the password does NOT appear in the command line
        assert!(
            !cmdline.contains(TEST_PASSWORD),
            "Password '{}' should NOT appear in cmdline when using --password-stdin. cmdline: {}",
            TEST_PASSWORD,
            cmdline.replace('\0', " ")
        );

        // Clean up the child process
        let _ = child.kill();
        let _ = child.wait();
    }

    /// Test case 6b: Verify that PDFTRACT_PASSWORD env var does NOT leak password in /proc/<pid>/cmdline (Linux only).
    #[cfg(target_os = "linux")]
    #[test]
    fn test_password_env_var_does_not_leak_in_cmdline() {
        use std::fs;
        use std::thread;
        use std::time::Duration;

        // Spawn the process with PDFTRACT_PASSWORD env var
        let fixture_path = get_fixture_path("security/password-protected.pdf");
        let mut child = Command::new("pdftract")
            .arg("extract")
            .arg(fixture_path)
            .arg("--output")
            .arg("-")
            .env("PDFTRACT_PASSWORD", TEST_PASSWORD)
            .spawn()
            .expect("Failed to spawn pdftract");

        let pid = child.id();

        // Give the process a moment to start
        thread::sleep(Duration::from_millis(100));

        // Read /proc/<pid>/cmdline
        let cmdline_path = format!("/proc/{}/cmdline", pid);
        let cmdline = fs::read_to_string(&cmdline_path)
            .unwrap_or_else(|e| panic!("Failed to read {}: {}", cmdline_path, e));

        // Verify that the password does NOT appear in the command line
        // (env vars are NOT visible in /proc/<pid>/cmdline)
        assert!(
            !cmdline.contains(TEST_PASSWORD),
            "Password '{}' should NOT appear in cmdline when using PDFTRACT_PASSWORD env var. cmdline: {}",
            TEST_PASSWORD,
            cmdline.replace('\0', " ")
        );

        // Clean up the child process
        let _ = child.kill();
        let _ = child.wait();
    }
}
