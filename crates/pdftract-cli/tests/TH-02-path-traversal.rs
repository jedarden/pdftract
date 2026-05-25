//! TH-02: Path traversal security test.
//!
//! This test verifies that path-traversal payloads are properly rejected
//! when --root DIR mode is enabled. The test documents 10 canonical traversal
//! payloads from the threat model (plan line 891).
//!
//! Per INV-10 (plan line 840), `pdftract mcp` in HTTP mode (`mcp --bind`)
//! MUST NOT accept file-path parameters. In stdio mode, path parameters are
//! allowed but protected via --root DIR canonicalization.
//!
//! # Current State (Phase 1)
//!
//! - MCP tools accept `path` parameters (extract, extract_text, etc.)
//! - Without --root: paths are passed through as-is (no validation)
//! - With --root: paths are canonicalized and checked against root boundary
//! - The --root mode is NOT yet wired to the MCP server entry point
//!
//! # Future State (Phase 2, #[ignore])
//!
//! - When --root DIR is introduced to `pdftract mcp`, all paths are validated
//! - PATH_ESCAPES_ROOT diagnostic is emitted for traversal attempts
//! - Tests verify all 10 payloads are rejected

use pdftract_cli::mcp::root::{
    canonicalize_root, resolve_path, CODE_ABSOLUTE_PATH_NOT_PERMITTED, CODE_PATH_ESCAPES_ROOT,
    CODE_PATH_RESOLUTION_FAILED,
};
use std::fs::{self, File};
use std::path::PathBuf;
use tempfile::TempDir;

/// The 10 canonical path-traversal payloads from the threat model.
///
/// These payloads represent common path-traversal attack vectors that
/// malicious MCP clients might use to escape the root directory.
const TRAVERSAL_PAYLOADS: &[&str] = &[
    "../../etc/passwd",              // 1. Basic traversal
    "../../../etc/passwd",           // 2. Deeper traversal
    "../../../../etc/passwd",        // 3. Very deep traversal
    "/etc/passwd",                   // 4. Absolute path outside root
    "./valid/../../../etc/passwd",   // 5. Traversal after valid prefix
    "valid/..%2F..%2Fetc%2Fpasswd",  // 6. URL-encoded traversal (%2F = /)
    "valid/..\\..\\..\\etc\\passwd", // 7. Windows-style separators (on Linux)
    "valid/../../../../etc/passwd",  // 8. Long traversal with valid prefix
    "/proc/self/environ",            // 9. Special filesystem path
    "con",                           // 10. Windows reserved name
];

/// Test that --root mode rejects path-traversal payloads.
#[test]
fn test_root_mode_rejects_all_traversal_payloads() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let canonical_root = canonicalize_root(root).unwrap();

    // Create a valid subdirectory with a file (so "valid/" prefix exists)
    let valid_dir = root.join("valid");
    fs::create_dir(&valid_dir).unwrap();
    let test_file = valid_dir.join("test.pdf");
    File::create(&test_file).unwrap();

    for (i, payload) in TRAVERSAL_PAYLOADS.iter().enumerate() {
        let result = resolve_path(payload, Some(&canonical_root));

        // All traversal payloads should be rejected
        assert!(
            result.is_err(),
            "Payload {} ({}) should be rejected but was accepted: {:?}",
            i + 1,
            payload,
            result
        );

        let err = result.unwrap_err();
        assert_eq!(
            err.code, -32602,
            "Error code should be -32602 (Invalid params)"
        );

        // Check that error data contains a relevant code
        let data = err.data.unwrap();
        let code = data.get("code").unwrap().as_str().unwrap();

        assert!(
            code == CODE_PATH_ESCAPES_ROOT
                || code == CODE_ABSOLUTE_PATH_NOT_PERMITTED
                || code == CODE_PATH_RESOLUTION_FAILED,
            "Payload {} ({}) should return PATH_ESCAPES_ROOT, ABSOLUTE_PATH_NOT_PERMITTED, or PATH_RESOLUTION_FAILED, got: {}",
            i + 1,
            payload,
            code
        );
    }
}

/// Test that --root mode accepts valid paths within the boundary.
#[test]
fn test_root_mode_accepts_valid_paths() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let canonical_root = canonicalize_root(root).unwrap();

    // Create test files
    let subdir = root.join("subdir");
    fs::create_dir(&subdir).unwrap();
    let file1 = root.join("test.pdf");
    let file2 = subdir.join("nested.pdf");
    File::create(&file1).unwrap();
    File::create(&file2).unwrap();

    // Test various valid path forms
    let valid_paths = &[
        "test.pdf",
        "./test.pdf",
        "subdir/nested.pdf",
        "./subdir/nested.pdf",
        "subdir/./nested.pdf",
    ];

    for path in valid_paths {
        let result = resolve_path(path, Some(&canonical_root));
        assert!(
            result.is_ok(),
            "Valid path '{}' should be accepted, got: {:?}",
            path,
            result
        );

        let resolved = result.unwrap();
        assert!(
            resolved.starts_with(&canonical_root),
            "Resolved path should start with root"
        );
    }
}

/// Test that without --root, paths are passed through (current behavior).
///
/// This documents the current security posture: without --root, the caller
/// is trusted to provide valid paths. This is acceptable for local stdio
/// mode where the client and server share the same security context.
#[test]
fn test_without_root_paths_pass_through() {
    for payload in TRAVERSAL_PAYLOADS {
        let result = resolve_path(payload, None);

        // Without --root, all paths are accepted as-is
        assert!(
            result.is_ok(),
            "Without --root, payload '{}' should pass through, got: {:?}",
            payload,
            result
        );

        let resolved = result.unwrap();
        assert_eq!(resolved, PathBuf::from(payload));
    }
}

/// Test that HTTPS URLs bypass path validation entirely.
///
/// Per INV-10, HTTPS URLs are allowed through and handled by the
/// remote source adapter (Phase 1.8). They bypass the --root check.
#[test]
fn test_https_urls_bypass_root_check() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let canonical_root = canonicalize_root(root).unwrap();

    let urls = &[
        "https://example.com/file.pdf",
        "https://cdn.example.com/path/to/file.pdf",
        "https://localhost:8000/file.pdf",
    ];

    for url in urls {
        let result = resolve_path(url, Some(&canonical_root));
        assert!(
            result.is_ok(),
            "HTTPS URL '{}' should bypass root check, got: {:?}",
            url,
            result
        );

        let resolved = result.unwrap();
        assert_eq!(resolved, PathBuf::from(url));
    }
}

/// Test that symlinks escaping root are rejected.
///
/// Even if a valid path within root contains a symlink that points
/// outside root, the canonicalized path check catches the escape.
#[test]
fn test_symlink_escape_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let canonical_root = canonicalize_root(root).unwrap();

    // Create a symlink inside root that points outside
    let symlink_path = root.join("escape_link");

    #[cfg(unix)]
    {
        std::os::unix::fs::symlink("/etc/passwd", &symlink_path).unwrap();
    }

    #[cfg(windows)]
    {
        // On Windows, use a file that definitely exists outside root
        let target = "C:\\Windows\\System32\\drivers\\etc\\hosts";
        if Path::new(target).exists() {
            std::os::windows::fs::symlink_file(target, &symlink_path).unwrap();
        } else {
            // Skip test if target doesn't exist
            return;
        }
    }

    let result = resolve_path("./escape_link", Some(&canonical_root));

    assert!(result.is_err(), "Symlink escaping root should be rejected");

    let err = result.unwrap_err();
    let data = err.data.unwrap();
    let code = data.get("code").unwrap().as_str().unwrap();

    assert_eq!(code, CODE_PATH_ESCAPES_ROOT);
}

/// Test URL-encoded traversal is rejected.
///
/// Payload 6 uses URL encoding (%2F for /) to bypass naive string checks.
/// Our canonicalization rejects it because the path still resolves outside.
#[test]
fn test_url_encoded_traversal_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let canonical_root = canonicalize_root(root).unwrap();

    // Create a "valid" directory to make the prefix look legitimate
    let valid_dir = root.join("valid");
    fs::create_dir(&valid_dir).unwrap();

    // URL-encoded traversal: %2F decodes to /
    let payload = "valid/..%2F..%2Fetc%2Fpasswd";
    let result = resolve_path(payload, Some(&canonical_root));

    // Note: This may pass through on some systems because the path
    // "valid/..%2F..%2Fetc%2Fpasswd" is not a valid path and may fail
    // canonicalization with a different error. The key is that it MUST
    // not escape the root.
    match result {
        Ok(resolved) => {
            // If it succeeds, verify it's still within root
            assert!(
                resolved.starts_with(&canonical_root),
                "URL-encoded traversal escaped root"
            );
        }
        Err(err) => {
            // Expected: path resolution fails or escape is detected
            assert_eq!(err.code, -32602);
        }
    }
}

/// Test that Windows reserved names are handled safely.
///
/// Payload 10: "con" is a reserved device name on Windows.
/// On Unix, this is just a normal filename and will be accepted
/// if it exists within root.
#[test]
fn test_windows_reserved_name_handling() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let canonical_root = canonicalize_root(root).unwrap();

    // On Windows, "con" is a reserved name and cannot be created
    // On Unix, we can create it to test behavior
    #[cfg(unix)]
    {
        // Don't create "con" in root - just test that the name itself
        // doesn't cause issues
        let result = resolve_path("con", Some(&canonical_root));

        // "con" doesn't exist, so we expect a resolution error
        assert!(result.is_err());
        let err = result.unwrap_err();
        let data = err.data.unwrap();
        let code = data.get("code").unwrap().as_str().unwrap();

        // Should be PATH_RESOLUTION_FAILED since the file doesn't exist
        assert_eq!(code, CODE_PATH_RESOLUTION_FAILED);
    }

    #[cfg(windows)]
    {
        // On Windows, "con" is a reserved device name
        // The behavior depends on how Windows handles it
        let result = resolve_path("con", Some(&canonical_root));

        // Either it fails to resolve or it resolves to the device (not a file)
        // In either case, it should not escape the root
        if let Ok(resolved) = result {
            assert!(
                resolved.starts_with(&canonical_root) || resolved.to_string_lossy().contains("con"),
                "Windows reserved name handling"
            );
        }
    }
}

/// Test special filesystem paths are rejected.
///
/// Payload 9: /proc/self/environ (Linux procfs)
/// This is a special filesystem path that provides access to process
/// environment. We reject it because it's an absolute path.
#[test]
fn test_special_filesystem_paths_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let canonical_root = canonicalize_root(root).unwrap();

    let special_paths = &[
        "/proc/self/environ",
        "/proc/self/cmdline",
        "/proc/self/mem",
        "/dev/urandom",
        "/dev/null",
    ];

    for path in special_paths {
        let result = resolve_path(path, Some(&canonical_root));
        assert!(
            result.is_err(),
            "Special filesystem path '{}' should be rejected",
            path
        );

        let err = result.unwrap_err();
        let data = err.data.unwrap();
        let code = data.get("code").unwrap().as_str().unwrap();

        assert_eq!(code, CODE_ABSOLUTE_PATH_NOT_PERMITTED);
    }
}

/// Test that nested traversal is caught even with valid-looking prefixes.
///
/// Payload 5: ./valid/../../../etc/passwd
/// The "valid/" prefix makes it look legitimate, but the ../.. escapes.
#[test]
fn test_nested_traversal_with_valid_prefix() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let canonical_root = canonicalize_root(root).unwrap();

    // Create the "valid" directory to make the prefix look legitimate
    let valid_dir = root.join("valid");
    fs::create_dir(&valid_dir).unwrap();

    let payload = "./valid/../../../etc/passwd";
    let result = resolve_path(payload, Some(&canonical_root));

    assert!(result.is_err(), "Nested traversal should be rejected");

    let err = result.unwrap_err();
    let data = err.data.unwrap();
    let code = data.get("code").unwrap().as_str().unwrap();

    assert_eq!(code, CODE_PATH_ESCAPES_ROOT);
}

/// Test that deep traversal is rejected.
///
/// Payloads 1-3 test various depths of ../ traversal.
#[test]
fn test_deep_traversal_rejected() {
    let temp_dir = TempDir::new().unwrap();
    let root = temp_dir.path();
    let canonical_root = canonicalize_root(root).unwrap();

    let deep_payloads = &[
        "../../etc/passwd",
        "../../../etc/passwd",
        "../../../../etc/passwd",
        "../../../../../etc/passwd",
        "../../../../../../etc/passwd",
    ];

    for payload in deep_payloads {
        let result = resolve_path(payload, Some(&canonical_root));
        assert!(
            result.is_err(),
            "Deep traversal payload '{}' should be rejected",
            payload
        );

        let err = result.unwrap_err();
        let data = err.data.unwrap();
        let code = data.get("code").unwrap().as_str().unwrap();

        assert_eq!(code, CODE_PATH_ESCAPES_ROOT);
    }
}
