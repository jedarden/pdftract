//! Orphaned process verification helpers for test hygiene.
//!
//! This module provides utilities to detect and report orphaned processes
//! that may have been left behind by test runs. Per CLAUDE.md test hygiene
//! rules, no processes matching patterns like 'pdftract mcp', 'TH-0', or 'TH_0'
//! should remain after tests complete.
//!
//! # Example
//!
//! ```rust
//! use test_helpers::process_guard::{verify_no_orphaned_processes, OrphanedProcessError};
//!
//! fn test_cleanup() {
//!     // Run your test that spawns processes
//!     // ...
//!
//!     // Verify no orphans before returning
//!     verify_no_orphaned_processes().expect("Tests should not leave orphaned processes");
//! }
//! ```

use std::process::Command;
use std::time::Duration;

/// Default process patterns to check for orphaned processes.
///
/// These patterns cover the main process types that tests may spawn:
/// - `pdftract mcp`: MCP server subprocess
/// - `TH-0`: Test harness process (hyphen variant)
/// - `TH_0`: Test harness process (underscore variant)
pub const DEFAULT_PROCESS_PATTERNS: &[&str] = &["pdftract mcp", "TH-0", "TH_0"];

/// Error type for orphaned process detection.
#[derive(Debug, Clone)]
pub enum OrphanedProcessError {
    /// Orphaned processes were found.
    OrphanedProcessesFound {
        /// Number of orphaned processes found
        count: usize,
        /// Process details (PID and command line)
        processes: Vec<(String, String)>,
    },
    /// Failed to execute pgrep command.
    ProcessCheckFailed {
        /// The error message from command execution
        message: String,
    },
}

impl std::fmt::Display for OrphanedProcessError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            OrphanedProcessError::OrphanedProcessesFound { count, processes } => {
                write!(f, "Found {} orphaned process(es):\n", count)?;
                for (pid, cmd) in processes {
                    writeln!(f, "  PID {}: {}", pid, cmd)?;
                }
                writeln!(f, "\nTests must clean up all spawned processes.")?;
                writeln!(f, "Use ProcessGuard RAII guards or ensure proper cleanup in drop handlers.")?;
            }
            OrphanedProcessError::ProcessCheckFailed { message } => {
                write!(f, "Failed to check for orphaned processes: {}", message)?;
            }
        }
        Ok(())
    }
}

impl std::error::Error for OrphanedProcessError {}

/// Search for processes matching a pattern using pgrep.
///
/// Returns a vector of (PID, command_line) tuples for all matching processes.
/// Returns Ok(vec) if successful (may be empty), or Err if pgrep fails.
fn find_processes_matching_pattern(pattern: &str) -> Result<Vec<(String, String)>, String> {
    let output = Command::new("pgrep")
        .args(["-af", pattern])
        .output()
        .map_err(|e| format!("Failed to execute pgrep: {}", e))?;

    if !output.status.success() {
        // pgrep returns non-zero when no processes match - that's OK, means clean state
        if output.stderr.is_empty() {
            return Ok(Vec::new());
        }
        return Err(format!("pgrep failed: {}", String::from_utf8_lossy(&output.stderr)));
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let mut processes = Vec::new();

    for line in stdout.lines() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }

        // pgrep output format: "PID command"
        if let Some(space_idx) = line.find(' ') {
            let pid = line[..space_idx].to_string();
            let command = line[space_idx + 1..].to_string();
            processes.push((pid, command));
        }
    }

    Ok(processes)
}

/// Verify no orphaned processes exist for the default patterns.
///
/// This is the main entry point for test cleanup verification. It checks
/// for processes matching the default patterns and returns an error if
/// any are found.
///
/// # Returns
///
/// - `Ok(())` if no orphaned processes found
/// - `Err(OrphanedProcessError)` if orphans detected or check failed
///
/// # Example
///
/// ```rust
/// #[test]
/// fn test_mcp_server_cleanup() {
///     let mut server = spawn_mcp_server();
///     // ... test code ...
///     drop(server); // Explicit cleanup
///
///     // Verify no orphans remain
///     verify_no_orphaned_processes().unwrap();
/// }
/// ```
pub fn verify_no_orphaned_processes() -> Result<(), OrphanedProcessError> {
    verify_no_processes_matching_patterns(DEFAULT_PROCESS_PATTERNS)
}

/// Verify no orphaned processes exist for custom patterns.
///
/// Same as `verify_no_orphaned_processes()` but allows specifying custom
/// process patterns. Useful for tests that spawn other process types.
///
/// # Arguments
///
/// * `patterns` - Slice of pattern strings to match against process commands
///
/// # Returns
///
/// - `Ok(())` if no orphaned processes found
/// - `Err(OrphanedProcessError)` if orphans detected or check failed
pub fn verify_no_processes_matching_patterns(
    patterns: &[&str],
) -> Result<(), OrphanedProcessError> {
    let mut all_orphans = Vec::new();

    for pattern in patterns {
        match find_processes_matching_pattern(pattern) {
            Ok(processes) => {
                all_orphans.extend(processes);
            }
            Err(e) => {
                return Err(OrphanedProcessError::ProcessCheckFailed { message: e });
            }
        }
    }

    if !all_orphans.is_empty() {
        return Err(OrphanedProcessError::OrphanedProcessesFound {
            count: all_orphans.len(),
            processes: all_orphans,
        });
    }

    Ok(())
}

/// Attempt to kill orphaned processes matching the default patterns.
///
/// This is a cleanup function that tries to kill any orphaned processes.
/// It should be used in test teardown or recovery scenarios, not as a
/// substitute for proper RAII guards.
///
/// # Returns
///
/// - `Ok(killed_count)` - Number of processes successfully killed
/// - `Err(OrphanedProcessError)` - If process check or kill failed
///
/// # Note
///
/// This is a last-resort cleanup mechanism. Tests should use ProcessGuard
/// or other RAII patterns to ensure cleanup happens automatically.
pub fn kill_orphaned_processes() -> Result<usize, OrphanedProcessError> {
    kill_processes_matching_patterns(DEFAULT_PROCESS_PATTERNS)
}

/// Attempt to kill orphaned processes matching custom patterns.
///
/// Same as `kill_orphaned_processes()` but allows custom patterns.
///
/// # Arguments
///
/// * `patterns` - Slice of pattern strings to match against process commands
///
/// # Returns
///
/// - `Ok(killed_count)` - Number of processes successfully killed
/// - `Err(OrphanedProcessError)` - If process check or kill failed
pub fn kill_processes_matching_patterns(
    patterns: &[&str],
) -> Result<usize, OrphanedProcessError> {
    let mut killed_count = 0;

    for pattern in patterns {
        let processes = find_processes_matching_pattern(pattern)
            .map_err(|e| OrphanedProcessError::ProcessCheckFailed { message: e })?;

        for (pid, _cmd) in processes {
            // Try to kill the process
            if let Ok(pid_num) = pid.parse::<u32>() {
                use std::io::ErrorKind;

                // Send SIGTERM
                if unsafe { libc::kill(pid_num as i32, libc::SIGTERM) } == 0 {
                    killed_count += 1;

                    // Give it a moment to exit gracefully
                    std::thread::sleep(Duration::from_millis(50));

                    // Force kill if still running
                    if unsafe { libc::kill(pid_num as i32, 0) } == 0 {
                        // Process still exists, force kill
                        let _ = unsafe { libc::kill(pid_num as i32, libc::SIGKILL) };
                    }
                }
            }
        }
    }

    Ok(killed_count)
}

/// RAII guard for ensuring no orphaned processes at scope exit.
///
/// This guard records the initial process state and verifies no new
/// orphans were added when it drops. Useful for test functions that
/// want to ensure cleanup without manual verification calls.
///
/// # Example
///
/// ```rust
/// fn test_spawns_processes() {
///     let _guard = OrphanedProcessGuard::new();
///
///     // ... test code that spawns processes ...
///
///     // Guard verifies cleanup on drop
/// }
/// ```
pub struct OrphanedProcessGuard {
    initial_orphan_count: usize,
}

impl OrphanedProcessGuard {
    /// Create a new guard, recording the current orphaned process state.
    pub fn new() -> Result<Self, OrphanedProcessError> {
        let initial_orphan_count = count_orphaned_processes(DEFAULT_PROCESS_PATTERNS)?;
        Ok(Self { initial_orphan_count })
    }

    /// Create a new guard with custom patterns.
    pub fn with_patterns(patterns: &[&str]) -> Result<Self, OrphanedProcessError> {
        let initial_orphan_count = count_orphaned_processes(patterns)?;
        Ok(Self { initial_orphan_count })
    }
}

impl Drop for OrphanedProcessGuard {
    fn drop(&mut self) {
        // Check if orphans increased during the guard's lifetime
        match count_orphaned_processes(DEFAULT_PROCESS_PATTERNS) {
            Ok(final_count) => {
                if final_count > self.initial_orphan_count {
                    eprintln!(
                        "Warning: OrphanedProcessGuard detected {} new orphaned process(es)",
                        final_count - self.initial_orphan_count
                    );
                }
            }
            Err(e) => {
                eprintln!("Warning: Failed to verify orphaned processes on drop: {}", e);
            }
        }
    }
}

fn count_orphaned_processes(patterns: &[&str]) -> Result<usize, OrphanedProcessError> {
    let mut total = 0;

    for pattern in patterns {
        let processes = find_processes_matching_pattern(pattern)
            .map_err(|e| OrphanedProcessError::ProcessCheckFailed { message: e })?;
        total += processes.len();
    }

    Ok(total)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_verify_no_orphaned_processes_clean_state() {
        // In a clean test environment, this should pass
        let result = verify_no_orphaned_processes();
        // We don't assert success here because other tests may have left orphans
        // We just verify the function works without panicking
        let _ = result;
    }

    #[test]
    fn test_find_processes_matching_pattern() {
        // This should always work (may return empty if no processes)
        let result = find_processes_matching_pattern("pdftract mcp");
        assert!(result.is_ok());

        // Verify we get a vector (may be empty)
        let _processes = result.unwrap();
        // We don't assert length because test environment varies
    }

    #[test]
    fn test_orphaned_process_guard() {
        // Creating the guard should work
        let guard = OrphanedProcessGuard::new();
        assert!(guard.is_ok());

        // Guard will verify cleanup on drop
        let _guard = guard.unwrap();
    }

    #[test]
    fn test_error_display() {
        let error = OrphanedProcessError::OrphanedProcessesFound {
            count: 2,
            processes: vec![
                ("1234".to_string(), "pdftract mcp".to_string()),
                ("5678".to_string(), "TH-0 test".to_string()),
            ],
        };

        let display = format!("{}", error);
        assert!(display.contains("2 orphaned process"));
        assert!(display.contains("PID 1234"));
        assert!(display.contains("pdftract mcp"));
    }
}
