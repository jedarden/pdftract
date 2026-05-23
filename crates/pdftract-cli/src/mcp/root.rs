//! Path resolution and escape checking for MCP tools.
//!
//! This module implements the --root DIR security boundary:
//! - All local path arguments are resolved relative to DIR
//! - Paths that escape DIR are rejected with -32602 (Invalid params)
//! - HTTPS URLs bypass the check entirely
//! - Absolute paths are rejected when --root is set

use crate::mcp::framing::ErrorObject;
use serde_json::json;
use std::path::{Path, PathBuf};

/// Error codes for MCP path operations.
pub const CODE_PATH_ESCAPES_ROOT: &str = "PATH_ESCAPES_ROOT";
pub const CODE_ABSOLUTE_PATH_NOT_PERMITTED: &str = "ABSOLUTE_PATH_NOT_PERMITTED";
pub const CODE_PATH_RESOLUTION_FAILED: &str = "PATH_RESOLUTION_FAILED";
pub const CODE_ROOT_INVALID: &str = "ROOT_INVALID";

/// Resolve a path argument against an optional root directory.
///
/// # Security
///
/// This function is the primary security boundary for path-traversal protection:
/// - HTTPS URLs bypass the check entirely (handled by HttpRangeSource)
/// - If no root is set, the path is returned as-is (trust-the-caller mode)
/// - If root is set, the path is resolved relative to root
/// - The canonical path is checked to ensure it doesn't escape root
/// - Absolute paths are rejected when root is set
///
/// # Arguments
///
/// * `arg` - The path argument from the MCP tool call
/// * `root` - The optional root directory (canonicalized at startup)
///
/// # Returns
///
/// * `Ok(PathBuf)` - The resolved, canonical path if it's within bounds
/// * `Err(ErrorObject)` - JSON-RPC error with code -32602 if path escapes or is invalid
pub fn resolve_path(arg: &str, root: Option<&Path>) -> Result<PathBuf, ErrorObject> {
    // https:// URLs bypass the check entirely (HttpRangeSource handles them)
    if arg.starts_with("http://") || arg.starts_with("https://") {
        return Ok(arg.into());
    }

    // If root is None, return the arg as-is (no protection)
    let root = match root {
        Some(r) => r,
        None => return Ok(arg.into()),
    };

    // Reject absolute paths when --root is set
    if arg.starts_with('/') || Path::new(arg).is_absolute() {
        return Err(ErrorObject::invalid_params()
            .with_message(format!("absolute paths not permitted under --root: '{}'", arg))
            .with_data(json!({ "code": CODE_ABSOLUTE_PATH_NOT_PERMITTED, "path": arg })));
    }

    // Resolve arg as a path relative to root
    let candidate = root.join(arg);

    // Canonicalize follows symlinks; this is what makes the check secure
    let canonical = std::fs::canonicalize(&candidate).map_err(|e| {
        ErrorObject::invalid_params()
            .with_message(format!("path resolution failed: {}", e))
            .with_data(json!({ "code": CODE_PATH_RESOLUTION_FAILED, "path": arg, "error": e.to_string() }))
    })?;

    // Reject if canonical is not a descendant of root
    if !canonical.starts_with(root) {
        return Err(ErrorObject::invalid_params()
            .with_message(format!("path '{}' escapes root '{}'", arg, root.display()))
            .with_data(json!({ "code": CODE_PATH_ESCAPES_ROOT, "path": arg, "root": root.display().to_string() })));
    }

    Ok(canonical)
}

/// Canonicalize and validate the root directory at startup.
///
/// This function should be called once at server startup to validate
/// and canonicalize the --root argument.
///
/// # Arguments
///
/// * `root_arg` - The raw --root argument from the CLI
///
/// # Returns
///
/// * `Ok(PathBuf)` - The canonicalized root directory
/// * `Err(String)` - Error message if root is invalid
pub fn canonicalize_root(root_arg: &Path) -> Result<PathBuf, String> {
    // Canonicalize the root path (follows symlinks, resolves relative components)
    let canonical = std::fs::canonicalize(root_arg)
        .map_err(|e| format!("--root path does not exist or cannot be canonicalized: {}", e))?;

    // Verify it's a directory
    if !canonical.is_dir() {
        return Err(format!("--root must be a directory, not a file: {}", canonical.display()));
    }

    Ok(canonical)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::io::Write;
    use tempfile::TempDir;

    #[test]
    fn test_https_url_bypasses_check() {
        let result = resolve_path("https://example.com/file.pdf", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("https://example.com/file.pdf"));

        let result = resolve_path("https://example.com/file.pdf", Some(Path::new("/tmp")));
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("https://example.com/file.pdf"));
    }

    #[test]
    fn test_http_url_bypasses_check() {
        let result = resolve_path("http://example.com/file.pdf", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("http://example.com/file.pdf"));
    }

    #[test]
    fn test_no_root_returns_path_as_is() {
        let result = resolve_path("some/path.pdf", None);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), PathBuf::from("some/path.pdf"));
    }

    #[test]
    fn test_absolute_path_rejected_with_root() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let result = resolve_path("/etc/passwd", Some(root));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("absolute paths not permitted"));
    }

    #[test]
    fn test_path_traversal_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a file inside root
        let file_path = root.join("test.txt");
        fs::write(&file_path, b"test content").unwrap();

        // Try to escape with ../..
        let result = resolve_path("../../../etc/passwd", Some(root));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("escapes root"));
    }

    #[test]
    fn test_valid_path_within_root() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a subdirectory and file
        let subdir = root.join("subdir");
        fs::create_dir(&subdir).unwrap();
        let file_path = subdir.join("test.pdf");
        fs::write(&file_path, b"%PDF-1.4\ntest").unwrap();

        // Resolve relative path
        let result = resolve_path("./subdir/test.pdf", Some(root));
        assert!(result.is_ok());
        let resolved = result.unwrap();
        assert!(resolved.starts_with(root));
        assert_eq!(resolved, file_path.canonicalize().unwrap());
    }

    #[test]
    fn test_symlink_escape_rejected() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Create a symlink inside root that points outside
        let symlink_path = root.join("escape");
        #[cfg(unix)]
        {
            std::os::unix::fs::symlink("/etc/passwd", &symlink_path).unwrap();
        }

        #[cfg(windows)]
        {
            std::os::windows::fs::symlink_file(r"C:\Windows\System32\drivers\etc\hosts", &symlink_path).unwrap();
        }

        // Try to access the symlink
        let result = resolve_path("./escape", Some(root));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("escapes root"));
    }

    #[test]
    fn test_nonexistent_path_returns_error() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        let result = resolve_path("nonexistent.pdf", Some(root));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("path resolution failed"));
    }

    #[test]
    fn test_canonicalize_root_validates_directory() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Valid directory
        let result = canonicalize_root(root);
        assert!(result.is_ok());

        // File instead of directory
        let file_path = root.join("file.txt");
        fs::write(&file_path, b"test").unwrap();
        let result = canonicalize_root(&file_path);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("must be a directory"));
    }

    #[test]
    fn test_canonicalize_root_rejects_nonexistent() {
        let result = canonicalize_root(Path::new("/nonexistent/path/that/does/not/exist"));
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("does not exist"));
    }

    #[test]
    fn test_dotdot_rejected_at_boundary() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Try to escape to parent of root
        let result = resolve_path("..", Some(root));
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert_eq!(err.code, -32602);
        assert!(err.message.contains("escapes root"));
    }

    #[test]
    fn test_error_object_includes_code() {
        let temp_dir = TempDir::new().unwrap();
        let root = temp_dir.path();

        // Test absolute path error
        let result = resolve_path("/etc/passwd", Some(root));
        let err = result.unwrap_err();
        let data = err.data.unwrap();
        assert_eq!(data.get("code").unwrap().as_str(), Some(CODE_ABSOLUTE_PATH_NOT_PERMITTED));

        // Test traversal error
        let result = resolve_path("../../../etc/passwd", Some(root));
        let err = result.unwrap_err();
        let data = err.data.unwrap();
        assert_eq!(data.get("code").unwrap().as_str(), Some(CODE_PATH_ESCAPES_ROOT));
    }
}
