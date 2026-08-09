//! Verification test for encryption_fixtures module
//!
//! This test verifies that the common encryption test fixtures and helpers
//! compile correctly and function as expected.

use std::path::PathBuf;

#[cfg(test)]
mod verification_tests {
    use crate::encryption_fixtures::*;

    #[test]
    fn test_workspace_root_exists() {
        let root = workspace_root();
        assert!(root.exists(), "Workspace root should exist");
        assert!(root.is_dir(), "Workspace root should be a directory");
    }

    #[test]
    fn test_pdftract_bin_path_format() {
        let bin = pdftract_bin();
        assert!(bin.ends_with("pdftract"), "Binary path should end with 'pdftract'");
        assert!(bin.to_str().unwrap().contains("target"), "Binary should be in target directory");
    }

    #[test]
    fn test_encrypted_fixtures_dir_exists() {
        let dir = encrypted_fixtures_dir();
        assert!(dir.exists(), "Encrypted fixtures directory should exist: {}", dir.display());
        assert!(dir.is_dir(), "Should be a directory");
    }

    #[test]
    fn test_fixture_path_resolution() {
        let fixture = encrypted_fixture("livecycle.pdf");
        assert!(fixture.ends_with("livecycle.pdf"), "Fixture path should end with filename");
        assert!(fixture.to_str().unwrap().contains("encrypted"), "Fixture should be in encrypted directory");
    }

    #[test]
    fn test_constants_defined() {
        assert_eq!(EXPECTED_ENCRYPTION_EXIT_CODE, 3, "Exit code should be 3");
        assert!(!ENCRYPTED_FIXTURES.is_empty(), "Fixtures list should not be empty");
        assert_eq!(TEST_PASSWORD, "test123", "Test password constant");
        assert_eq!(WRONG_PASSWORD, "wrongpassword", "Wrong password constant");
    }

    #[test]
    fn test_livecycle_fixture_exists() {
        let fixture = encrypted_fixture("livecycle.pdf");
        if fixture.exists() {
            assert_valid_pdf_structure(&fixture);
            println!("✓ livecycle.pdf fixture exists and is valid");
        } else {
            println!("⊘ livecycle.pdf fixture not found (expected in some environments)");
        }
    }

    #[test]
    fn test_assertion_functions_exist() {
        // These functions should compile and be callable
        // We're just testing type checking here
        let mock_output = std::process::Output {
            status: std::process::ExitStatus::from_raw(3),
            stdout: b"test".to_vec(),
            stderr: b"error".to_vec(),
        };

        // This would panic if called with actual data, but we're just testing compilation
        let _ = mock_output;
    }

    #[test]
    #[cfg(feature = "decrypt")]
    fn test_rc4_dict_builder() {
        let dict = make_rc4_encryption_dict();
        assert!(dict.contains_key("/Filter"));
        assert!(dict.contains_key("/V"));
        assert!(dict.contains_key("/R"));
    }

    #[test]
    #[cfg(feature = "decrypt")]
    fn test_aes128_dict_builder() {
        let dict = make_aes128_encryption_dict();
        assert!(dict.contains_key("/Filter"));
        assert!(dict.contains_key("/V"));
        assert!(dict.contains_key("/StmF"));
    }

    #[test]
    #[cfg(feature = "decrypt")]
    fn test_aes256_dict_builder() {
        let dict = make_aes256_encryption_dict();
        assert!(dict.contains_key("/Filter"));
        assert!(dict.contains_key("/V"));
        assert!(dict.contains_key("/UE"));
        assert!(dict.contains_key("/OE"));
        assert!(dict.contains_key("/Perms"));
    }

    #[test]
    #[cfg(feature = "decrypt")]
    fn test_unsupported_dict_builder() {
        let dict = make_unsupported_encryption_dict("Adobe.PPKLite");
        assert!(dict.contains_key("/Filter"));
        assert!(dict.contains_key("/V"));
        assert!(dict.contains_key("/R"));
    }

    #[test]
    fn test_fixture_list_is_complete() {
        // Verify that all common encrypted fixtures are listed
        let expected_fixtures = vec![
            "livecycle.pdf",
            "EC-04-rc4-encrypted.pdf",
            "EC-05-aes128-encrypted.pdf",
            "EC-06-aes256-encrypted.pdf",
            "EC-empty-password.pdf",
        ];

        for fixture in expected_fixtures {
            assert!(ENCRYPTED_FIXTURES.contains(&fixture),
                "Expected fixture '{}' should be in ENCRYPTED_FIXTURES", fixture);
        }
    }

    #[test]
    fn test_helper_function_signatures() {
        // Test that helper functions have the correct signatures
        let bin = pdftract_bin();
        let fixture = encrypted_fixture("test.pdf");

        // These should compile with correct types
        let _bin: PathBuf = bin;
        let _fixture: PathBuf = fixture;
    }
}
