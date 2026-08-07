//! pdftract binary invocation utilities
//!
//! This module provides utilities for spawning and interacting with the pdftract
//! binary in tests. It handles command construction, process spawning, stdio
//! configuration, and basic error handling for missing binaries.

use std::path::{Path, PathBuf};
use std::process::{Command, Stdio, Child, Output};
use std::io;

/// Path to the pdftract binary.
///
/// This uses the CARGO_BIN_EXE_pdftract environment variable which is set by
/// cargo test to point to the built binary.
pub fn pdftract_binary_path() -> PathBuf {
    PathBuf::from(env!("CARGO_BIN_EXE_pdftract"))
}

/// Error type for pdftract binary invocation failures.
#[derive(Debug)]
pub enum InvocationError {
    /// The pdftract binary was not found at the expected path.
    BinaryNotFound(PathBuf),
    /// Failed to spawn the process.
    SpawnFailed(io::Error),
    /// The binary path is not a valid executable.
    NotExecutable(PathBuf),
    /// Process exited with non-zero status code.
    ProcessFailed { exit_code: Option<i32>, stdout: String, stderr: String },
    /// Failed to read process output.
    OutputReadFailed(io::Error),
}

impl std::fmt::Display for InvocationError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            InvocationError::BinaryNotFound(path) => {
                write!(f, "pdftract binary not found at: {}", path.display())
            }
            InvocationError::SpawnFailed(e) => {
                write!(f, "failed to spawn pdftract process: {}", e)
            }
            InvocationError::NotExecutable(path) => {
                write!(f, "pdftract binary is not executable: {}", path.display())
            }
            InvocationError::ProcessFailed { exit_code, stdout, stderr } => {
                write!(f, "pdftract process failed with exit code: {:?}", exit_code)?;
                if !stdout.is_empty() {
                    write!(f, "\nstdout: {}", stdout)?;
                }
                if !stderr.is_empty() {
                    write!(f, "\nstderr: {}", stderr)?;
                }
                Ok(())
            }
            InvocationError::OutputReadFailed(e) => {
                write!(f, "failed to read pdftract output: {}", e)
            }
        }
    }
}

impl std::error::Error for InvocationError {}

/// Captured output from a pdftract process execution.
///
/// This struct holds the raw output bytes from stdout and stderr, along with
/// the exit status of the process.
#[derive(Debug, Clone)]
pub struct CapturedOutput {
    /// Raw bytes from stdout
    pub stdout: Vec<u8>,
    /// Raw bytes from stderr
    pub stderr: Vec<u8>,
    /// Exit code of the process (None if terminated by signal)
    pub exit_code: Option<i32>,
}

impl CapturedOutput {
    /// Create a new CapturedOutput from individual components.
    pub fn new(stdout: Vec<u8>, stderr: Vec<u8>, exit_code: Option<i32>) -> Self {
        Self { stdout, stderr, exit_code }
    }

    /// Get the exit code of the process.
    ///
    /// Returns `None` if the process was terminated by a signal.
    pub fn exit_code(&self) -> Option<i32> {
        self.exit_code
    }

    /// Check if the process exited successfully (exit code 0).
    pub fn success(&self) -> bool {
        self.exit_code == Some(0)
    }

    /// Get stdout as a UTF-8 string, with lossy conversion.
    ///
    /// This replaces invalid UTF-8 sequences with the replacement character.
    pub fn stdout_str(&self) -> String {
        String::from_utf8_lossy(&self.stdout).to_string()
    }

    /// Get stderr as a UTF-8 string, with lossy conversion.
    ///
    /// This replaces invalid UTF-8 sequences with the replacement character.
    pub fn stderr_str(&self) -> String {
        String::from_utf8_lossy(&self.stderr).to_string()
    }

    /// Combine stdout and stderr into a single string for debugging.
    pub fn combined_output(&self) -> String {
        let mut combined = String::new();
        if !self.stdout.is_empty() {
            combined.push_str("=== STDOUT ===\n");
            combined.push_str(&self.stdout_str());
            combined.push('\n');
        }
        if !self.stderr.is_empty() {
            combined.push_str("=== STDERR ===\n");
            combined.push_str(&self.stderr_str());
            combined.push('\n');
        }
        combined
    }
}

impl From<Output> for CapturedOutput {
    fn from(output: Output) -> Self {
        Self {
            stdout: output.stdout,
            stderr: output.stderr,
            exit_code: output.status.code(),
        }
    }
}

/// Builder for constructing pdftract command invocations.
pub struct PdftractCommand {
    command: Command,
}

impl PdftractCommand {
    /// Create a new pdftract command builder.
    pub fn new() -> Result<Self, InvocationError> {
        let binary_path = pdftract_binary_path();

        // Check if the binary exists
        if !binary_path.exists() {
            return Err(InvocationError::BinaryNotFound(binary_path));
        }

        Ok(Self {
            command: Command::new(&binary_path),
        })
    }

    /// Get the path to the pdftract binary without creating a command.
    pub fn binary_path() -> PathBuf {
        pdftract_binary_path()
    }

    /// Add an argument to the command.
    pub fn arg<S: AsRef<OsStr>>(mut self, arg: S) -> Self {
        self.command.arg(arg);
        self
    }

    /// Add multiple arguments to the command.
    pub fn args<I, S>(mut self, args: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: AsRef<OsStr>,
    {
        self.command.args(args);
        self
    }

    /// Configure stdin for the process.
    pub fn stdin(mut self, cfg: Stdio) -> Self {
        self.command.stdin(cfg);
        self
    }

    /// Configure stdout for the process.
    pub fn stdout(mut self, cfg: Stdio) -> Self {
        self.command.stdout(cfg);
        self
    }

    /// Configure stderr for the process.
    pub fn stderr(mut self, cfg: Stdio) -> Self {
        self.command.stderr(cfg);
        self
    }

    /// Spawn the pdftract process with the configured arguments.
    pub fn spawn(mut self) -> Result<Child, InvocationError> {
        self.command
            .spawn()
            .map_err(InvocationError::SpawnFailed)
    }

    /// Run the pdftract command and capture its output.
    ///
    /// This method configures stdout and stderr to be piped, spawns the process,
    /// waits for completion, and returns the captured output along with exit status.
    ///
    /// # Errors
    ///
    /// Returns `InvocationError::SpawnFailed` if the process cannot be spawned.
    /// Returns `InvocationError::OutputReadFailed` if the output cannot be read.
    /// Returns `InvocationError::ProcessFailed` if the process exits with a non-zero status.
    ///
    /// # Example
    ///
    /// ```no_run
    /// use pdftract_invocation::PdftractCommand;
    ///
    /// let output = PdftractCommand::new()?
    ///     .arg("--version")
    ///     .output()
    ///     .unwrap();
    ///
    /// println!("Exit code: {:?}", output.exit_code());
    /// println!("stdout: {}", String::from_utf8_lossy(&output.stdout));
    /// println!("stderr: {}", String::from_utf8_lossy(&output.stderr));
    /// ```
    pub fn output(mut self) -> Result<CapturedOutput, InvocationError> {
        // Configure stdout and stderr to be piped for capture
        self.command.stdout(Stdio::piped());
        self.command.stderr(Stdio::piped());

        // Spawn the process
        let child = self.command
            .spawn()
            .map_err(InvocationError::SpawnFailed)?;

        // Wait for process to complete and capture output
        let output = child
            .wait_with_output()
            .map_err(InvocationError::OutputReadFailed)?;

        // Convert to CapturedOutput
        let captured = CapturedOutput::from(output);

        // Check exit status and surface errors
        if !captured.success() {
            return Err(InvocationError::ProcessFailed {
                exit_code: captured.exit_code(),
                stdout: String::from_utf8_lossy(&captured.stdout).to_string(),
                stderr: String::from_utf8_lossy(&captured.stderr).to_string(),
            });
        }

        Ok(captured)
    }

    /// Create a pdftract MCP server command with stdio configuration.
    ///
    /// This is a convenience method for the common case of spawning an MCP server
    /// with stdin/stdout piped and stderr discarded.
    pub fn mcp_stdio() -> Result<Self, InvocationError> {
        Ok(Self::new()?
            .arg("mcp")
            .arg("--stdio")
            .stdin(Stdio::piped())
            .stdout(Stdio::piped())
            .stderr(Stdio::null()))
    }

    /// Create a pdftract command to process a specific file.
    ///
    /// This creates a command that will process the PDF file at the given path.
    pub fn with_file<P: AsRef<Path>>(path: P) -> Result<Self, InvocationError> {
        Ok(Self::new()?.arg(path.as_ref()))
    }
}

impl Default for PdftractCommand {
    fn default() -> Self {
        Self::new().expect("pdftract binary should be available")
    }
}

use std::ffi::OsStr;

/// Verify that the pdftract binary exists and is executable.
pub fn verify_binary_available() -> Result<(), InvocationError> {
    let binary_path = pdftract_binary_path();

    if !binary_path.exists() {
        return Err(InvocationError::BinaryNotFound(binary_path));
    }

    // Additional checks could be added here (e.g., verify it's executable)
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pdftract_binary_path_exists() {
        let path = pdftract_binary_path();
        assert!(path.exists(), "pdftract binary should exist at: {:?}", path);
        assert!(path.is_file(), "pdftract path should be a file");
    }

    #[test]
    fn test_verify_binary_available() {
        assert!(verify_binary_available().is_ok());
    }

    #[test]
    fn test_mcp_stdio_command_construction() {
        let cmd = PdftractCommand::mcp_stdio().unwrap();
        // Can't easily inspect the command internals, but we can verify it doesn't panic
        drop(cmd);
    }

    #[test]
    fn test_with_file_command() {
        let cmd = PdftractCommand::with_file("/tmp/test.pdf").unwrap();
        drop(cmd);
    }

    #[test]
    fn test_output_capture_success() {
        // Test with --help which should always succeed
        let result = PdftractCommand::new()
            .unwrap()
            .arg("--help")
            .output();

        assert!(result.is_ok(), "pdftract --help should succeed");
        let output = result.unwrap();

        assert!(output.success(), "--help should exit with code 0");
        assert_eq!(output.exit_code(), Some(0));

        let stdout_str = output.stdout_str();
        assert!(!stdout_str.is_empty(), "--help should produce output");
        assert!(stdout_str.contains("pdftract") || stdout_str.contains("PDF"),
                "help output should mention pdftract or PDF");
    }

    #[test]
    fn test_output_capture_failure() {
        // Test with an invalid option to trigger non-zero exit
        let result = PdftractCommand::new()
            .unwrap()
            .arg("--nonexistent-flag-xyz")
            .output();

        assert!(result.is_err(), "invalid flag should cause error");

        match result {
            Err(InvocationError::ProcessFailed { exit_code, stdout, stderr }) => {
                assert!(exit_code.is_some(), "should have exit code");
                assert_ne!(exit_code, Some(0), "exit code should be non-zero");
                // Either stdout or stderr should contain error information
                let has_error = stdout.contains("error") ||
                               stderr.contains("error") ||
                               stdout.contains("unrecognized") ||
                               stderr.contains("unrecognized");
                assert!(has_error || !stdout.is_empty() || !stderr.is_empty(),
                       "error output should contain diagnostic information");
            }
            _ => panic!("Expected ProcessFailed error, got: {:?}", result),
        }
    }

    #[test]
    fn test_captured_output_combined() {
        let output = CapturedOutput::new(
            b"stdout data".to_vec(),
            b"stderr data".to_vec(),
            Some(0),
        );

        let combined = output.combined_output();
        assert!(combined.contains("stdout data"));
        assert!(combined.contains("stderr data"));
        assert!(combined.contains("=== STDOUT ==="));
        assert!(combined.contains("=== STDERR ==="));
    }

    #[test]
    fn test_captured_output_success_check() {
        let success_output = CapturedOutput::new(
            b"".to_vec(),
            b"".to_vec(),
            Some(0),
        );
        assert!(success_output.success());

        let failure_output = CapturedOutput::new(
            b"".to_vec(),
            b"".to_vec(),
            Some(1),
        );
        assert!(!failure_output.success());

        let signaled_output = CapturedOutput::new(
            b"".to_vec(),
            b"".to_vec(),
            None,
        );
        assert!(!signaled_output.success());
    }
}
