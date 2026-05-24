//! Integration tests for --root path-traversal protection.
//!
//! These tests verify the acceptance criteria from bead pdftract-6696g:
//! - Path-traversal attempts are rejected with -32602
//! - Absolute paths are rejected when --root is set
//! - HTTPS URLs bypass the path check
//! - Without --root, paths are not validated
//! - Symlink escapes are rejected
//! - Invalid root paths cause startup errors

use pdftract_cli::mcp::root::{canonicalize_root, resolve_path};
use std::fs;
use std::path::Path;

#[test]
fn test_acceptance_criteria_path_traversal_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    // Create a file inside root
    let file_path = root.join("test.txt");
    fs::write(&file_path, b"test content").unwrap();

    // Try to escape with ../..
    let result = resolve_path("../../../etc/passwd", Some(root));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err.code, -32602,
        "Should return -32602 (Invalid params) for path traversal"
    );
    assert!(err.message.contains("escapes root"));
}

#[test]
fn test_acceptance_criteria_valid_path_within_root() {
    let temp_dir = tempfile::tempdir().unwrap();
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
fn test_acceptance_criteria_absolute_path_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    let result = resolve_path("/etc/passwd", Some(root));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, -32602, "Should return -32602 for absolute paths");
    assert!(err.message.contains("absolute paths not permitted"));
}

#[test]
fn test_acceptance_criteria_https_url_bypasses_check() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    let result = resolve_path("https://example.com/file.pdf", Some(root));
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        std::path::PathBuf::from("https://example.com/file.pdf")
    );
}

#[test]
fn test_acceptance_criteria_no_root_trust_the_caller() {
    // Without --root, paths should be returned as-is (trust-the-caller mode)
    let result = resolve_path("../../../etc/passwd", None);
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        std::path::PathBuf::from("../../../etc/passwd")
    );
}

#[test]
fn test_acceptance_criteria_symlink_escape_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    // Create a symlink inside root that points outside
    let symlink_path = root.join("escape");
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc/passwd", &symlink_path).unwrap();
    }

    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(r"C:\Windows\System32\drivers\etc\hosts", &symlink_path)
            .unwrap();
    }

    // Try to access the symlink
    let result = resolve_path("./escape", Some(root));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, -32602, "Should return -32602 for symlink escape");
    assert!(err.message.contains("escapes root"));
}

#[test]
fn test_acceptance_criteria_nonexistent_root_startup_error() {
    let result = canonicalize_root(Path::new("/nonexistent/path/that/does/not/exist"));
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("does not exist"));
}

#[test]
fn test_acceptance_criteria_file_not_directory_startup_error() {
    let temp_dir = tempfile::tempdir().unwrap();
    let file_path = temp_dir.path().join("file.txt");
    fs::write(&file_path, b"test").unwrap();

    let result = canonicalize_root(&file_path);
    assert!(result.is_err());
    assert!(result.unwrap_err().contains("must be a directory"));
}

#[test]
fn test_plan_critical_test_path_traversal_with_root() {
    // Plan section 6.7 line 2346 critical test:
    // "Path-traversal attempt with --root /var/data: path=\"../../etc/passwd\" rejected with -32602"
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path(); // Simulating /var/data

    let result = resolve_path("../../etc/passwd", Some(root));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(
        err.code, -32602,
        "Critical test: path traversal must return -32602"
    );
    assert!(err.message.contains("escapes root"));

    // Verify the error data contains the expected code
    let data = err.data.unwrap();
    assert_eq!(
        data.get("code").unwrap().as_str(),
        Some("PATH_ESCAPES_ROOT")
    );
}

#[test]
fn test_http_url_bypasses_check() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    let result = resolve_path("http://example.com/file.pdf", Some(root));
    assert!(result.is_ok());
    assert_eq!(
        result.unwrap(),
        std::path::PathBuf::from("http://example.com/file.pdf")
    );
}

#[test]
fn test_dotdot_at_boundary_rejected() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    // Try to escape to parent of root
    let result = resolve_path("..", Some(root));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, -32602);
    assert!(err.message.contains("escapes root"));
}

#[test]
fn test_nonexistent_file_within_root_returns_error() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    let result = resolve_path("nonexistent.pdf", Some(root));
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert_eq!(err.code, -32602, "Non-existent file should return -32602");
    assert!(err.message.contains("path resolution failed"));

    // Verify the error data contains the expected code
    let data = err.data.unwrap();
    assert_eq!(
        data.get("code").unwrap().as_str(),
        Some("PATH_RESOLUTION_FAILED")
    );
}

#[test]
fn test_complex_path_traversal_patterns() {
    let temp_dir = tempfile::tempdir().unwrap();
    let root = temp_dir.path();

    // Test various traversal patterns
    let traversal_patterns = [
        "../..",
        "../../.",
        "./../..",
        "foo/../../etc",
        "....//./../etc",
    ];

    for pattern in traversal_patterns {
        let result = resolve_path(pattern, Some(root));
        assert!(result.is_err(), "Pattern '{}' should be rejected", pattern);
        let err = result.unwrap_err();
        assert_eq!(
            err.code, -32602,
            "Pattern '{}' should return -32602",
            pattern
        );
    }
}
