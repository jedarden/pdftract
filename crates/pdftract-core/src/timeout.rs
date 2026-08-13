//! Bounded wait and timeout protection for subprocess management.
//!
//! This module provides utilities for running subprocesses with timeout protection
//! to prevent indefinite hangs. The core function `wait_with_timeout` ensures that:
//! - Subprocesses complete within a specified time limit
//! - Processes are killed cleanly on timeout
//! - No orphaned processes are left behind
//!
//! # Usage
//!
//! ```rust
//! use std::process::Command;
//! use pdftract_core::timeout::wait_with_timeout;
//!
//! let mut child = Command::new("pdftract")
//!     .arg("extract")
//!     .arg("file.pdf")
//!     .spawn()
//!     .expect("Failed to spawn process");
//!
//! match wait_with_timeout(&mut child, 5000) {
//!     Ok(Some(exit_code)) => println!("Process exited with code {}", exit_code),
//!     Ok(None) => println!("Process terminated by signal"),
//!     Err(e) if e.kind() == std::io::ErrorKind::TimedOut => {
//!         println!("Process timed out and was killed");
//!     }
//!     Err(e) => println!("Error waiting for process: {}", e),
//! }
//! ```

use std::io;
use std::process::Child;
use std::thread;
use std::time::{Duration, Instant};

/// Wait for a child process to complete, with a bounded timeout.
///
/// This function polls the child process status until either:
/// - The process exits (returns Ok with exit code)
/// - The timeout is reached (kills the process and returns timeout error)
///
/// # Arguments
///
/// * `child` - Mutable reference to the child process
/// * `timeout_ms` - Timeout in milliseconds
///
/// # Returns
///
/// * `Ok(Some(code))` - Process exited with the given exit code
/// * `Ok(None)` - Process terminated by signal
/// * `Err(TimedOut)` - Process did not exit within timeout and was killed
/// * `Err(other)` - Other I/O error occurred
///
/// # Timeout Behavior
///
/// When timeout is reached:
/// 1. The process is killed with `child.kill()`
/// 2. A second bounded wait (100ms) waits for the process to exit after kill
/// 3. If the process still doesn't exit, returns TimedOut error
///
/// This ensures that even if a process defies SIGKILL, we don't hang forever.
pub fn wait_with_timeout(child: &mut Child, timeout_ms: u64) -> io::Result<Option<i32>> {
    let deadline = Instant::now() + Duration::from_millis(timeout_ms);

    loop {
        if let Some(status) = child.try_wait()? {
            return Ok(status.code());
        }

        if Instant::now() >= deadline {
            // Timeout: kill the process
            let _ = child.kill();

            // Wait with bounded timeout after kill - never use bare wait()
            let kill_deadline = Instant::now() + Duration::from_millis(100);
            loop {
                if let Some(status) = child.try_wait()? {
                    return Ok(status.code());
                }

                if Instant::now() >= kill_deadline {
                    // Process didn't exit after kill - return timeout error
                    return Err(io::Error::new(
                        io::ErrorKind::TimedOut,
                        "Process did not exit within timeout after kill",
                    ));
                }

                thread::sleep(Duration::from_millis(10));
            }
        }

        thread::sleep(Duration::from_millis(50));
    }
}

/// Run a command to completion with a bounded timeout.
///
/// This is a convenience wrapper that spawns a command and waits for it
/// with timeout protection, collecting stdout and stderr.
///
/// # Arguments
///
/// * `command` - The command to run (typically created with `Command::new()`)
/// * `timeout_ms` - Timeout in milliseconds
///
/// # Returns
///
/// * `Ok(output)` - Command completed successfully, returns output
/// * `Err(TimedOut)` - Command timed out and was killed
/// * `Err(other)` - Command failed to spawn or other I/O error
///
/// # Example
///
/// ```rust
/// use std::process::Command;
/// use pdftract_core::timeout::command_with_timeout;
///
/// let output = command_with_timeout(
///     &mut Command::new("pdftract")
///         .arg("extract")
///         .arg("file.pdf"),
///     5000
/// )?;
/// ```
pub fn command_with_timeout(command: &mut std::process::Command, timeout_ms: u64) -> io::Result<std::process::Output> {
    // Spawn the command with piped stdout/stderr to collect output
    command.stdout(std::process::Stdio::piped());
    command.stderr(std::process::Stdio::piped());

    let mut child = command.spawn()?;

    // Wait with timeout
    match wait_with_timeout(&mut child, timeout_ms) {
        Ok(_) => {
            // Process exited, try to collect output
            // Note: If we already have Output, this is redundant
            // This function is mainly for commands spawned without .output()
            let mut stdout = child.stdout.take().ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "Failed to capture stdout")
            })?;
            let mut stderr = child.stderr.take().ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "Failed to capture stderr")
            })?;

            use std::io::Read;
            let mut stdout_buf = Vec::new();
            let mut stderr_buf = Vec::new();
            stdout.read_to_end(&mut stdout_buf)?;
            stderr.read_to_end(&mut stderr_buf)?;

            // Get exit status
            let status = child.try_wait()?.ok_or_else(|| {
                io::Error::new(io::ErrorKind::Other, "Process status unavailable")
            })?;

            Ok(std::process::Output {
                status,
                stdout: stdout_buf,
                stderr: stderr_buf,
            })
        }
        Err(e) => {
            // Timeout or other error - ensure process is killed
            let _ = child.kill();
            Err(e)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn test_wait_with_timeout_success() {
        // Quick command that completes within timeout
        let mut child = Command::new("echo")
            .arg("test")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn()
            .expect("Failed to spawn echo");

        let result = wait_with_timeout(&mut child, 1000);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), Some(0));
    }

    #[test]
    fn test_wait_with_timeout_timeout() {
        // Sleep command that exceeds timeout
        let mut child = Command::new("sleep")
            .arg("10")  // Sleep for 10 seconds
            .stdout(std::process::Stdio::null())
            .stderr(std::process::Stdio::null())
            .spawn()
            .expect("Failed to spawn sleep");

        let result = wait_with_timeout(&mut child, 100); // 100ms timeout

        assert!(result.is_err());
        assert_eq!(result.unwrap_err().kind(), io::ErrorKind::TimedOut);
    }

    #[test]
    fn test_wait_with_timeout_invalid_command() {
        // Invalid command that fails immediately
        let mut child = Command::new("nonexistent_command_that_does_not_exist_xyz123")
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .spawn();

        // Should fail to spawn
        assert!(child.is_err());
    }
}
